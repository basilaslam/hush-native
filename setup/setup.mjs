#!/usr/bin/env node
// Host-only tooling. This file and its dependencies are never part of the desktop app.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline/promises';
import { access, cp, mkdir, readFile, realpath, writeFile } from 'node:fs/promises';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const OAUTH_SCOPES = ['account:read', 'user:read', 'workers:write', 'workers_scripts:write'];
const source = path.dirname(fileURLToPath(import.meta.url));
const exists = file => access(file).then(() => true, () => false);
export function stateDirectory(platform = process.platform, env = process.env, home = homedir()) {
  return env.HUSH_SETUP_STATE ?? (platform === 'win32'
    ? path.join(env.LOCALAPPDATA ?? path.join(home, 'AppData', 'Local'), 'Hush/Hush/data/cloudflare')
    : path.join(env.XDG_DATA_HOME ?? path.join(home, '.local/share'), 'hush/cloudflare'));
}
export function wranglerEnvironment(env = process.env) {
  const result = { ...env, CLOUDFLARE_AUTH_USE_KEYRING:'true', WRANGLER_SEND_METRICS:'false', NO_COLOR:'1', CI:'true' };
  const secrets = new Set(['CLOUDFLARE_API_TOKEN','CLOUDFLARE_API_KEY','CLOUDFLARE_EMAIL','CLOUDFLARE_ACCOUNT_ID','CF_API_TOKEN','CF_API_KEY','CF_EMAIL']);
  for (const name of Object.keys(result)) {
    if (secrets.has(name.toUpperCase()) || (process.platform === 'win32' && name.toUpperCase() === 'PATH' && name !== 'PATH')) delete result[name];
  }
  // npm postinstall scripts also need the private Node executable on PATH.
  result.PATH = path.dirname(process.execPath)+path.delimiter+(env.PATH ?? env.Path ?? '');
  return result;
}
export function options(args) {
  const result = { dryRun:false, help:false, stateDir:stateDirectory(), worker:null, account:null };
  for (let i=0;i<args.length;i++) {
    if (args[i] === '--dry-run') result.dryRun = true;
    else if (args[i] === '--help') result.help = true;
    else if (['--state-dir','--worker','--account'].includes(args[i])) {
      const key = { '--state-dir':'stateDir', '--worker':'worker', '--account':'account' }[args[i]];
      if (!args[i+1] || args[i+1].startsWith('--')) throw Error(`Missing value for ${args[i]}`);
      result[key] = args[++i];
    } else throw Error(`Unknown option: ${args[i]}`);
  }
  if (!path.isAbsolute(result.stateDir)) throw Error('The state directory must be an absolute path.');
  if (result.worker && !/^[a-z0-9][a-z0-9-]{1,62}$/.test(result.worker)) throw Error('Worker names must contain 2–63 lowercase letters, digits or hyphens.');
  if (result.account && !/^[a-f0-9]{32}$/i.test(result.account)) throw Error('Invalid Cloudflare account ID.');
  return result;
}
export function workerConfig(account, name, workerFile, deploymentId) {
  if (!/^[a-f0-9]{32}$/i.test(account) || !/^[a-z0-9][a-z0-9-]{1,62}$/.test(name) || !path.isAbsolute(workerFile)) throw Error('Invalid deployment configuration.');
  return { name, account_id:account, main:workerFile, compatibility_date:'2026-11-08', compatibility_flags:['nodejs_compat'],
    workers_dev:true, preview_urls:false, durable_objects:{bindings:[{name:'ROOMS',class_name:'HushRoom'}]},
    migrations:[{tag:'v1',new_sqlite_classes:['HushRoom']}],vars:{DEPLOYMENT_ID:deploymentId} };
}
export function serverOrigin(output, name) {
  for (const value of output.match(/https:\/\/[a-z0-9.-]+\.workers\.dev/g) ?? []) {
    const url = new URL(value); if (url.hostname.startsWith(name+'.')) return url.origin;
  }
  throw Error('Wrangler did not return the expected server URL.');
}
export async function checkServer(origin, deploymentId, fetcher = fetch, {attempts=12, delay=2000}={}) {
  const url = new URL(origin);
  if (url.protocol !== 'https:' || !url.hostname.endsWith('.workers.dev') || url.username || url.password || url.port || url.pathname !== '/' || url.search || url.hash) throw Error('Invalid server URL.');
  for (let i=0;i<attempts;i++) {
    try {
      const response = await fetcher(url.origin+'/health',{redirect:'error',signal:AbortSignal.timeout(10000)});
      if (response.ok && Number(response.headers.get('content-length') ?? 0) < 16000) {
        const data = await response.json();
        if (data.app === 'hush-native' && data.protocol === 1 && data.provider === 'cloudflare' && data.deploymentId === deploymentId) return url.origin;
      }
    } catch {}
    if (i+1<attempts) await new Promise(resolve=>setTimeout(resolve,delay));
  }
  throw Error('The deployed server did not pass its health/identity check. Rerun setup to retry.');
}
async function npmCli() {
  const bin = path.dirname(process.execPath);
  const candidates = [path.join(bin,'node_modules/npm/bin/npm-cli.js'),path.join(bin,'../lib/node_modules/npm/bin/npm-cli.js')];
  for (const dir of (process.env.PATH ?? '').split(path.delimiter)) {
    candidates.push(path.join(dir,'node_modules/npm/bin/npm-cli.js'));
    try { const resolved = await realpath(path.join(dir,'npm')); if (resolved.endsWith('npm-cli.js')) candidates.push(resolved); } catch {}
  }
  for (const file of candidates) if (await exists(file)) return file;
  throw Error('npm is missing. Rerun the setup bootstrap with its local Node runtime option.');
}
let active;
function command(executable,args,{cwd,inherit=false,env=wranglerEnvironment(),timeout=300000}={}) {
  return new Promise((resolve,reject)=>{
    const child = spawn(executable,args,{cwd,env,windowsHide:true,stdio:inherit?'inherit':['ignore','pipe','pipe']});
    active = child; let stdout='',stderr='';
    const timer = setTimeout(()=>child.kill(),timeout);
    child.stdout?.on('data',data=>{stdout=(stdout+data).slice(-524288)});
    child.stderr?.on('data',data=>{stderr=(stderr+data).slice(-524288)});
    child.on('error',error=>{clearTimeout(timer);active=null;reject(error)});
    child.on('exit',code=>{clearTimeout(timer);active=null;resolve({code,stdout,stderr})});
  });
}
async function installDependencies(root) {
  const lock = await readFile(path.join(root,'package-lock.json'));
  const fingerprint = createHash('sha256').update(lock).update(process.versions.node.split('.')[0]+process.platform+process.arch).digest('hex');
  const marker = path.join(root,'.dependencies.json');
  if (await readFile(marker,'utf8').catch(()=>null) === fingerprint && await exists(path.join(root,'node_modules/wrangler/bin/wrangler.js'))) return;
  const result = await command(process.execPath,[await npmCli(),'ci','--omit=dev','--no-audit','--no-fund'],{cwd:root,inherit:true});
  if (result.code !== 0) throw Error('Installing the server tools failed. Check the network connection, then rerun setup.');
  await writeFile(marker,fingerprint);
}
export function windowsConfigBase(home, env = {}) {
  return path.join(env.XDG_CONFIG_HOME ?? path.join(env.APPDATA ?? path.join(home,'AppData/Roaming'),'xdg.config'),'.wrangler');
}
async function prepareKeyring(root) {
  if (process.platform !== 'win32') return; // Wrangler uses macOS Keychain / Linux secret-tool natively.
  const legacy = path.join(homedir(),'.wrangler');
  const base = await exists(legacy) ? legacy : windowsConfigBase(homedir(),process.env);
  const destination = path.join(base,'native/keyring/node_modules/@napi-rs');
  await mkdir(destination,{recursive:true});
  for (const name of ['keyring',`keyring-win32-${process.arch}-msvc`]) {
    const target = path.join(destination,name);
    if (!await exists(target)) await cp(path.join(root,'node_modules/@napi-rs',name),target,{recursive:true,force:false});
  }
}
function errorMessage(result) {
  const text = result.stdout+result.stderr;
  if (/workers.dev subdomain|register.*subdomain|10034/i.test(text)) return 'Finish Workers onboarding and create a workers.dev subdomain in the Cloudflare dashboard, then rerun setup.';
  if (/keyring|keychain|secret-tool|credential/i.test(text)) return 'Secure credential storage is unavailable. Unlock your keychain (Linux needs secret-tool/libsecret), then rerun setup. No plaintext fallback is enabled.';
  if (/quota|billing|paid plan|limit exceeded/i.test(text)) return 'Your Cloudflare account limits prevented deployment. Check the account dashboard, then retry.';
  return 'Cloudflare could not finish setup. Check your connection and account permissions, then rerun the script.';
}
export async function runSetup(opts, {
  root=source, run, choose, fetcher=fetch, status=console.log,
  install=installDependencies, keyring=prepareKeyring,
}={}) {
  if (Number(process.versions.node.split('.')[0]) < 22) throw Error('Node 22 or newer is required. Use the bootstrap script.');
  const workerFile = path.join(root,'worker.mjs');
  if (!await exists(workerFile)) throw Error('The compiled server is missing. Use the release setup scripts, or run npm run cloudflare:build in the source repository.');
  await mkdir(opts.stateDir,{recursive:true,mode:0o700});
  status('[1/5] Preparing host-only server tools…'); await install(root);
  const wrangler = async (args,extra={}) => run ? run(args,extra) : command(process.execPath,[path.join(root,'node_modules/wrangler/bin/wrangler.js'),...args],{cwd:opts.stateDir,...extra});
  if (opts.dryRun) {
    const config = path.join(opts.stateDir,'dry-run.json');
    await writeFile(config,JSON.stringify(workerConfig('0'.repeat(32),'hush-setup-check',workerFile,'dry-run'),null,2));
    const result = await wrangler(['deploy','--dry-run','--config',config,'--outdir',path.join(opts.stateDir,'dry-run')]);
    if (result.code !== 0) throw Error('Local server validation failed. '+(result.stderr+result.stdout).slice(-2000));
    status('Dry run passed. No browser sign-in or cloud deployment was performed.'); return {dryRun:true};
  }
  status('[2/5] Signing in with the official Cloudflare CLI…'); await keyring(root);
  const activated = await wrangler(['auth','activate','hush-native',opts.stateDir]);
  let who = activated.code === 0 ? await wrangler(['whoami','--json']) : {code:1};
  if (who.code !== 0) {
    status('Approve the Cloudflare browser sign-in. Credentials remain in your secure local Wrangler profile.');
    const login = await wrangler(['auth','create','hush-native','--scopes',...OAUTH_SCOPES],{inherit:true,timeout:360000});
    if (login.code !== 0) throw Error(errorMessage(login));
    if ((await wrangler(['auth','activate','hush-native',opts.stateDir])).code !== 0) throw Error('Could not activate the Hush sign-in profile.');
    who = await wrangler(['whoami','--json']);
  }
  if (who.code !== 0) throw Error(errorMessage(who));
  const start = who.stdout.indexOf('{'), end = who.stdout.lastIndexOf('}');
  const identity = JSON.parse(who.stdout.slice(start,end+1));
  const accounts = (identity.accounts ?? []).filter(a=>/^[a-f0-9]{32}$/i.test(a.id));
  if (!accounts.length) throw Error('No Cloudflare account is available. Create an account and finish Workers onboarding, then rerun setup.');
  status('[3/5] Choosing your room server…');
  const account = opts.account ? accounts.find(a=>a.id===opts.account) : accounts.length===1 ? accounts[0] : await choose(accounts);
  if (!account || !accounts.some(a=>a.id===account.id)) throw Error('Choose an account available to your sign-in.');
  const manifestPath = path.join(opts.stateDir,'servers.json');
  const manifest = await readFile(manifestPath,'utf8').then(JSON.parse).catch(()=>({}));
  const prior = manifest[account.id];
  const name = opts.worker ?? (prior && /^[a-z0-9][a-z0-9-]{1,62}$/.test(prior.name) ? prior.name : 'hush-squad-'+randomBytes(6).toString('hex'));
  manifest[account.id] = {name}; await writeFile(manifestPath,JSON.stringify(manifest,null,2),{mode:0o600});
  const project = path.join(opts.stateDir,name); await mkdir(project,{recursive:true});
  const deploymentId = randomUUID(); const config = path.join(project,'wrangler.json');
  await writeFile(config,JSON.stringify(workerConfig(account.id,name,workerFile,deploymentId),null,2));
  status(`[4/5] Deploying ${name} to Cloudflare…`);
  const deployed = await wrangler(['deploy','--config',config,'--profile','hush-native']);
  if (deployed.code !== 0) throw Error(errorMessage(deployed));
  status('[5/5] Verifying your live server…');
  const origin = await checkServer(serverOrigin(deployed.stdout,name),deploymentId,fetcher);
  manifest[account.id].url = origin; await writeFile(manifestPath,JSON.stringify(manifest,null,2),{mode:0o600});
  await writeFile(path.join(opts.stateDir,'server-address.txt'),origin+'\n');
  status(`\nServer ready: ${origin}\nPaste this in Hush → Voice & settings → Coordination server, then Save settings.\nShare the same address and a room invitation with your friends.\nRerun this script to update the server. Cloudflare account quotas apply.`);
  return {origin,name};
}
async function main() {
  const opts = options(process.argv.slice(2));
  if (opts.help) { console.log('Hush server setup: [--dry-run] [--worker existing-worker-name] [--account account-id] [--state-dir absolute-path]\nOnly the host runs this script. GitHub distributes the tools; Cloudflare hosts the room coordinator.'); return; }
  process.once('SIGINT',()=>{active?.kill();process.exitCode=130});
  await runSetup(opts,{choose:async accounts=>{
    accounts.forEach((a,i)=>console.log(`${i+1}. ${a.name ?? 'Cloudflare account'}`));
    const prompt = createInterface({input:process.stdin,output:process.stdout});
    try { const value = await prompt.question('Account number: '); return accounts[Number(value)-1]; } finally { prompt.close(); }
  }});
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch(error=>{console.error('\n'+error.message);process.exitCode=1});
