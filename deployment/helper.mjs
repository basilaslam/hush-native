import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { mkdir, readFile, writeFile, cp, access } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { randomBytes, randomUUID } from 'node:crypto';
export const OAUTH_SCOPES = ['account:read', 'user:read', 'workers:write', 'workers_scripts:write'];
const root = path.dirname(fileURLToPath(import.meta.url));
const cli = path.join(root, 'node_modules/wrangler/bin/wrangler.js');
const emit = (event, fields = {}) => process.stdout.write(JSON.stringify({ event, ...fields }) + '\n');
const isCli = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
const input = isCli ? createInterface({ input: process.stdin }) : null;
let active;
let commandCwd = root;
let cancelled = false;
let chooseAccount;
function stop() {
  cancelled = true;
  if (active?.pid) {
    if (process.platform === 'win32') spawn('C:/Windows/System32/taskkill.exe', ['/PID', String(active.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
    else active.kill('SIGTERM');
  }
  chooseAccount?.(null);
}
input?.on('line', line => { try { const data = JSON.parse(line); if (data.op === 'cancel') stop(); else if (data.op === 'account') chooseAccount?.(data.id); } catch {} });
input?.on('close', () => { if (!finished) stop(); });
let finished = false;
const env = { ...process.env, CLOUDFLARE_AUTH_USE_KEYRING: 'true', WRANGLER_SEND_METRICS: 'false', NO_COLOR: '1', CI: 'true' };
for (const key of ['CLOUDFLARE_API_TOKEN', 'CLOUDFLARE_API_KEY', 'CLOUDFLARE_EMAIL', 'CLOUDFLARE_ACCOUNT_ID', 'CF_API_TOKEN', 'CF_API_KEY', 'CF_EMAIL']) delete env[key];
async function run(args, timeout = 180000) {
  if (cancelled) throw new Error('cancelled');
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [cli, ...args], { env, cwd: commandCwd, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    active = child;
    let stdout = '', stderr = '';
    const timer = setTimeout(() => { stop(); }, timeout);
    child.stdout.on('data', data => { stdout = (stdout + data).slice(-524288); });
    child.stderr.on('data', data => { stderr = (stderr + data).slice(-524288); });
    child.on('error', error => { clearTimeout(timer); active = null; reject(error); });
    child.on('exit', code => { clearTimeout(timer); active = null; resolve({ code, stdout, stderr }); });
  });
}
function parseJson(output) { const first = output.indexOf('{'), last = output.lastIndexOf('}'); if (first < 0 || last < first) throw new Error('invalid-account-response'); return JSON.parse(output.slice(first, last + 1)); }
export function windowsConfigBase(home, env = {}) { return path.join(env.XDG_CONFIG_HOME ?? path.join(env.APPDATA ?? path.join(home, 'AppData', 'Roaming'), 'xdg.config'), '.wrangler'); }
async function prepareKeyring() {
  const legacy = path.join(os.homedir(), '.wrangler');
  const configBase = await access(legacy).then(() => legacy).catch(() => windowsConfigBase(os.homedir(), process.env));
  const modules = path.join(configBase, 'native/keyring/node_modules/@napi-rs');
  await mkdir(modules, { recursive: true });
  for (const name of ['keyring', 'keyring-win32-x64-msvc']) {
    const target = path.join(modules, name);
    if (await access(target).then(() => true).catch(() => false)) continue;
    await cp(path.join(root, 'node_modules/@napi-rs', name), target, { recursive: true, force: false });
  }
}
function publicError(result) {
  const text = result.stdout + result.stderr;
  if (/10034|workers.dev subdomain|register.*subdomain|workers subdomain/i.test(text)) return 'Enable a workers.dev subdomain in your Cloudflare dashboard, then try again.';
  if (/permission|authentication|10000|unauthorized|forbidden/i.test(text)) return 'Cloudflare did not grant the required Workers permissions. Sign in again with an account that can deploy Workers.';
  if (/quota|billing|paid plan|10027|limit exceeded/i.test(text)) return 'Your Cloudflare account limits or plan prevented deployment. Review them in the Cloudflare dashboard, then retry.';
  if (/keyring|keychain|Credential/i.test(text)) return 'Windows credential storage is unavailable. Restart Hush and try again. Credentials were not saved in plain text.';
  return 'Cloudflare could not finish deployment. Check your connection and account permissions, then retry.';
}
export function serverOrigin(output, expectedName) {
  const matches = output.match(/https:\/\/[a-z0-9.-]+\.workers\.dev/g) ?? [];
  for (const value of matches) { const url = new URL(value); if (url.hostname.startsWith(expectedName + '.')) return url.origin; }
  throw new Error('missing-deployment-url');
}
export async function checkServer(origin, deploymentId, fetcher = fetch, { attempts = 12, delay = 2000 } = {}) {
  const url = new URL(origin);
  if (url.protocol !== 'https:' || !url.hostname.endsWith('.workers.dev') || url.username || url.password || url.port || url.pathname !== '/' || url.search || url.hash) throw new Error('invalid-server-url');
  for (let attempt = 0; attempt < attempts; attempt++) {
    if (cancelled) throw new Error('cancelled');
    try {
      const response = await fetcher(url.origin + '/health', { redirect: 'error', signal: AbortSignal.timeout(10000) });
      if (response.ok) { const data = await response.json(); if (data.app === 'hush-native' && data.protocol === 1 && data.provider === 'cloudflare' && data.deploymentId === deploymentId) return url.origin; }
    } catch {}
    if (attempt < attempts - 1) await new Promise(resolve => setTimeout(resolve, delay));
  }
  throw new Error('health-check-failed');
}
async function main() {
  const stateIndex = process.argv.indexOf('--state-dir');
  const stateDir = stateIndex >= 0 ? process.argv[stateIndex + 1] : null;
  if (!stateDir || !path.isAbsolute(stateDir)) throw new Error('missing-state-directory');
  await mkdir(stateDir, { recursive: true });
  commandCwd = stateDir;
  emit('status', { message: 'Checking Cloudflare sign-in…' });
  await prepareKeyring();
  const activated = await run(['auth', 'activate', 'hush-native', stateDir], 30000);
  let who = activated.code === 0 ? await run(['whoami', '--json'], 30000) : { code: 1 };
  if (who.code !== 0) {
    emit('status', { message: 'Approve Cloudflare’s official Wrangler sign-in in your browser.' });
    const login = await run(['auth', 'create', 'hush-native', '--scopes', ...OAUTH_SCOPES], 360000);
    if (cancelled) throw new Error('cancelled');
    if (login.code !== 0) throw new Error(publicError(login));
    const activated = await run(['auth', 'activate', 'hush-native', stateDir], 30000);
    if (activated.code !== 0) throw new Error('Could not activate your Cloudflare sign-in. Try again.');
    who = await run(['whoami', '--json'], 30000);
  }
  if (who.code !== 0) throw new Error('Sign-in could not be verified. Try again.');
  const identity = parseJson(who.stdout);
  const accounts = (identity.accounts ?? []).filter(a => /^[a-f0-9]{32}$/i.test(a.id)).map(a => ({ id: a.id, name: String(a.name ?? 'Cloudflare account').slice(0, 120) }));
  if (!accounts.length) throw new Error('No Cloudflare account is available for deployment. Create an account in Cloudflare, then retry.');
  let account;
  if (accounts.length === 1) account = accounts[0];
  else {
    const selected = new Promise(resolve => { chooseAccount = resolve; });
    emit('accounts', { accounts });
    const id = await selected;
    if (cancelled) throw new Error('cancelled');
    account = accounts.find(a => a.id === id);
    if (!account) throw new Error('Choose one of your Cloudflare accounts and try again.');
  }
  const manifestPath = path.join(stateDir, 'servers.json');
  const manifest = await readFile(manifestPath, 'utf8').then(JSON.parse).catch(() => ({}));
  let server = manifest[account.id];
  if (!server || !/^hush-squad-[a-f0-9]{12}$/.test(server.name)) server = { name: 'hush-squad-' + randomBytes(6).toString('hex') };
  manifest[account.id] = server;
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2));
  const deploymentId = randomUUID();
  const project = path.join(stateDir, server.name);
  await mkdir(project, { recursive: true });
  const config = { name: server.name, account_id: account.id, main: path.join(root, 'worker.mjs'), compatibility_date: '2026-11-08', compatibility_flags: ['nodejs_compat'], workers_dev: true, preview_urls: false, durable_objects: { bindings: [{ name: 'ROOMS', class_name: 'HushRoom' }] }, migrations: [{ tag: 'v1', new_sqlite_classes: ['HushRoom'] }], vars: { DEPLOYMENT_ID: deploymentId } };
  const configPath = path.join(project, 'wrangler.json');
  await writeFile(configPath, JSON.stringify(config, null, 2));
  emit('status', { message: 'Deploying your Hush room server…' });
  const deployment = await run(['deploy', '--config', configPath, '--profile', 'hush-native'], 300000);
  if (cancelled) throw new Error('cancelled');
  if (deployment.code !== 0) throw new Error(publicError(deployment));
  const origin = serverOrigin(deployment.stdout, server.name);
  emit('status', { message: 'Checking your live server…' });
  const verified = await checkServer(origin, deploymentId);
  if (cancelled) throw new Error('cancelled');
  server.url = verified;
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2));
  emit('ready', { server: verified });
}
if (isCli) {
  main().catch(error => { emit(cancelled ? 'cancelled' : 'error', { message: cancelled ? 'Deployment cancelled. Any server already created remains in your Cloudflare account.' : String(error.message).slice(0, 300) }); process.exitCode = 1; }).finally(() => { finished = true; input?.close(); });
}
