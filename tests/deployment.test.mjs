import path from 'node:path';
import {test} from 'node:test';
import assert from 'node:assert/strict';
import {OAUTH_SCOPES,serverOrigin,checkServer,windowsConfigBase} from '../deployment/helper.mjs';
test('deployment URL extraction selects only the generated worker',()=>{
 assert.equal(serverOrigin('https://other.owner.workers.dev\nhttps://hush-squad-aabbccddeeff.owner.workers.dev','hush-squad-aabbccddeeff'),'https://hush-squad-aabbccddeeff.owner.workers.dev');
 assert.throws(()=>serverOrigin('https://other.owner.workers.dev','hush-squad-aabbccddeeff'));
});
test('server verification requires the deployed identity and native protocol',async()=>{
 const valid={app:'hush-native',protocol:1,provider:'cloudflare',deploymentId:'new-deployment'};
 assert.equal(await checkServer('https://hush.owner.workers.dev','new-deployment',async()=>Response.json(valid)), 'https://hush.owner.workers.dev');
 await assert.rejects(()=>checkServer('https://hush.owner.workers.dev','new-deployment',async()=>Response.json({...valid,deploymentId:'old-deployment'}),{attempts:2,delay:0}),/health-check/);
 await assert.rejects(()=>checkServer('https://hush.owner.workers.dev','new-deployment',async()=>Response.json({...valid,protocol:2}),{attempts:1,delay:0}),/health-check/);
});
test('setup rejects credential-bearing, redirected, insecure or unexpected server addresses',async()=>{
 for(const origin of ['http://hush.owner.workers.dev','https://hush.owner.workers.dev:8443','https://user:password@hush.owner.workers.dev','https://hush.owner.workers.dev/path','https://workers.dev.attacker.test']) await assert.rejects(()=>checkServer(origin,'x'));
 let redirectPolicy; await assert.rejects(()=>checkServer('https://hush.owner.workers.dev','x',async(_,options)=>{redirectPolicy=options.redirect;throw Error('redirect');},{attempts:1,delay:0})); assert.equal(redirectPolicy,'error');
});

test('requested OAuth scopes are accepted by the bundled Cloudflare CLI',async()=>{
 const {execFile}=await import('node:child_process');
 const output=await new Promise((resolve,reject)=>execFile(process.execPath,['deployment/node_modules/wrangler/bin/wrangler.js','login','--scopes-list'],{cwd:process.cwd(),windowsHide:true},(error,stdout)=>error?reject(error):resolve(stdout)));
 for(const scope of OAUTH_SCOPES)assert.ok(output.includes(scope),scope);
 assert.ok(!OAUTH_SCOPES.includes('offline_access'));
});

test("Windows configuration path works without APPDATA",()=>{
 assert.equal(windowsConfigBase("C:/Users/example"),path.join("C:/Users/example/AppData/Roaming/xdg.config/.wrangler"));
 assert.equal(windowsConfigBase("C:/Users/example",{APPDATA:"D:/Roaming"}),path.join("D:/Roaming/xdg.config/.wrangler"));
 assert.equal(windowsConfigBase("C:/Users/example",{XDG_CONFIG_HOME:"D:/Config"}),path.join("D:/Config/.wrangler"));
});
