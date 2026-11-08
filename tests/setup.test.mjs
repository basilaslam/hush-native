import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,rm,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {options,stateDirectory,wranglerEnvironment,workerConfig,serverOrigin,checkServer,windowsConfigBase,runSetup,OAUTH_SCOPES} from '../setup/setup.mjs';
test('bootstrap release pins match the version being published',async()=>{
 const {version}=JSON.parse(await readFile('package.json','utf8'));
 assert.ok((await readFile('scripts/setup-server.ps1','utf8')).includes(`[string]$Release = 'v${version}'`));
 assert.ok((await readFile('scripts/setup-server.sh','utf8')).includes('HUSH_SETUP_RELEASE:-v'+version));
});
test('setup validates arguments and isolates credentials from inherited environment',()=>{
 assert.throws(()=>options(['--worker','../other']),/Worker names/);
 assert.throws(()=>options(['--account','bad']),/account/);
 assert.throws(()=>options(['--unknown']),/Unknown option/);
 assert.throws(()=>options(['--state-dir','relative']),/absolute/);
 const env=wranglerEnvironment({CLOUDFLARE_API_TOKEN:'secret',CF_EMAIL:'private',PATH:'ok'});
 assert.equal(env.CLOUDFLARE_API_TOKEN,undefined); assert.equal(env.CF_EMAIL,undefined);
 assert.equal(env.CLOUDFLARE_AUTH_USE_KEYRING,'true'); assert.ok(env.PATH.endsWith(path.delimiter+'ok'));
 assert.ok(stateDirectory('win32',{LOCALAPPDATA:'C:/Local'},'C:/User').endsWith(path.join('Hush/Hush/data/cloudflare')));
 assert.equal(windowsConfigBase('C:/User'),path.join('C:/User/AppData/Roaming/xdg.config/.wrangler'));
});
test('deployment verifies URL, deployment identity, protocol and redirect policy',async()=>{
 assert.equal(serverOrigin('https://other.owner.workers.dev\nhttps://hush-squad-aabbccddeeff.owner.workers.dev','hush-squad-aabbccddeeff'),'https://hush-squad-aabbccddeeff.owner.workers.dev');
 assert.throws(()=>serverOrigin('https://other.owner.workers.dev','hush-squad-aabbccddeeff'));
 const valid={app:'hush-native',protocol:1,provider:'cloudflare',deploymentId:'new'};
 assert.equal(await checkServer('https://hush.owner.workers.dev','new',async()=>Response.json(valid)), 'https://hush.owner.workers.dev');
 for(const changed of [{deploymentId:'old'},{protocol:2}]) await assert.rejects(()=>checkServer('https://hush.owner.workers.dev','new',async()=>Response.json({...valid,...changed}),{attempts:1,delay:0}),/health\/identity/);
 for(const origin of ['http://hush.owner.workers.dev','https://hush.owner.workers.dev:8443','https://user:password@hush.owner.workers.dev','https://hush.owner.workers.dev/path','https://workers.dev.attacker.test']) await assert.rejects(()=>checkServer(origin,'new'));
 let policy; await assert.rejects(()=>checkServer('https://hush.owner.workers.dev','new',async(_,opts)=>{policy=opts.redirect;throw Error('redirect')},{attempts:1,delay:0})); assert.equal(policy,'error');
});
test('terminal setup uses the selected account, preserves the server on rerun and writes its verified address',async()=>{
 const folder=await mkdtemp(path.join(tmpdir(),'hush-setup-'));
 try {
  const root=path.join(folder,"host's tools"); await mkdir(root); await writeFile(path.join(root,'worker.mjs'),'export default {};');
  const opts=options(['--state-dir',path.join(folder,'state')]);
  const account={id:'a'.repeat(32),name:'Squad'}; let config; let firstName; let signedIn=false; const commands=[];
  const run=async(args)=>{
   commands.push(args);
   if(args[0]==='auth' && args[1]==='activate') return {code:signedIn?0:1,stdout:'',stderr:''};
   if(args[0]==='auth' && args[1]==='create') {signedIn=true;return {code:0,stdout:'',stderr:''}};
   if(args[0]==='whoami') return {code:0,stdout:JSON.stringify({accounts:[account]}),stderr:''};
   if(args[0]==='deploy') {config=JSON.parse(await readFile(args[args.indexOf('--config')+1],'utf8'));return {code:0,stdout:`https://${config.name}.owner.workers.dev`,stderr:''}};
   throw Error('Unexpected command');
  };
  const dependencies={root,run,install:async()=>{},keyring:async()=>{},status:()=>{},fetcher:async()=>Response.json({app:'hush-native',protocol:1,provider:'cloudflare',deploymentId:config.vars.DEPLOYMENT_ID})};
  const first=await runSetup(opts,dependencies); firstName=first.name;
  assert.equal(await readFile(path.join(opts.stateDir,'server-address.txt'),'utf8'),first.origin+'\n');
  assert.equal(config.account_id,account.id); assert.deepEqual(config.migrations,[{tag:'v1',new_sqlite_classes:['HushRoom']}]);
  assert.equal((await runSetup(opts,dependencies)).name,firstName);
  assert.equal(commands.filter(args=>args[0]==='auth' && args[1]==='create').length,1);
 } finally {await rm(folder,{recursive:true,force:true})}
});
test('dry run skips sign-in, keyring, account selection and cloud deployment',async()=>{
 const folder=await mkdtemp(path.join(tmpdir(),'hush-dry-'));
 try {
  await writeFile(path.join(folder,'worker.mjs'),'export default {};');
  let commands=0;
  const result=await runSetup(options(['--dry-run','--state-dir',path.join(folder,'state')]),{root:folder,install:async()=>{},keyring:async()=>{throw Error('keyring invoked')},status:()=>{},run:async(args)=>{assert.equal(args[0],'deploy');assert.ok(args.includes('--dry-run'));commands++;return {code:0,stdout:'',stderr:''}}});
  assert.deepEqual(result,{dryRun:true}); assert.equal(commands,1);
  assert.throws(()=>workerConfig('bad','worker',path.join(folder,'worker.mjs'),'test'));
 } finally {await rm(folder,{recursive:true,force:true})}
});
test('requested scopes are accepted by the pinned official Wrangler CLI',async()=>{
 const output=await new Promise((resolve,reject)=>execFile(process.execPath,['setup/node_modules/wrangler/bin/wrangler.js','login','--scopes-list'],{cwd:process.cwd(),windowsHide:true},(error,stdout)=>error?reject(error):resolve(stdout)));
 for(const scope of OAUTH_SCOPES) assert.ok(output.includes(scope),scope);
 assert.ok(!OAUTH_SCOPES.includes('offline_access'));
});
