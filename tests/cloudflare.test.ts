import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { before } from 'node:test';
import { spawn, execFile, type ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import path from 'node:path';
let child: ChildProcess;
let origin: string;
let output = '';
before(async () => {
 const listener = createServer(); await new Promise<void>(resolve => listener.listen(0, '127.0.0.1', resolve));
 const port = (listener.address() as {port:number}).port; await new Promise<void>(resolve=>listener.close(()=>resolve()));
 origin = 'http://127.0.0.1:' + port;
 child = spawn(process.execPath, ['deployment/node_modules/wrangler/bin/wrangler.js','dev','--local','--config','cloudflare/wrangler.json','--port',String(port),'--ip','127.0.0.1'], {cwd:process.cwd(),env:{...process.env,WRANGLER_SEND_METRICS:'false',NO_COLOR:'1'},windowsHide:true,stdio:['ignore','pipe','pipe']});
 child.stdout?.on('data', data=>{output=(output+data).slice(-12000)}); child.stderr?.on('data',data=>{output=(output+data).slice(-12000)});
 for(let attempt=0;attempt<60;attempt++) { try { const r=await fetch(origin+'/health'); if(r.ok)return; }catch{} if(child.exitCode!==null)throw Error(output); await new Promise(r=>setTimeout(r,500)); }
 throw Error('Cloudflare local runtime did not start: '+output);
});
after(async () => { if(child?.pid && child.exitCode===null) await new Promise<void>(resolve=>{ if(process.platform==='win32')execFile('C:/Windows/System32/taskkill.exe',['/PID',String(child.pid),'/T','/F'],{windowsHide:true},()=>resolve());else{child.kill();resolve();} }); });
const mf={ dispatchFetch:(url:string, options?:RequestInit)=>fetch(url.replace('http://localhost',origin),options) };
const identity = (seed: number) => ({ certificate: Buffer.alloc(256, seed).toString('base64'), endpoints: ['127.0.0.1:41642'] });
const request = async (data: object, ip = '192.0.2.10') => {
 const res = await mf.dispatchFetch('http://localhost/api/native-room', { method: 'POST', headers: { 'content-type': 'application/json', 'CF-Connecting-IP': ip }, body: JSON.stringify(data) });
 return { status: res.status, data: await res.json() as any };
};
test('Cloudflare health identifies this deployment and rejects invalid HTTP requests', async () => {
 const health = await mf.dispatchFetch('http://localhost/health');
 assert.deepEqual(await health.json(), { app: 'hush-native', protocol: 1, provider: 'cloudflare', deploymentId: 'development' });
 assert.equal((await mf.dispatchFetch('http://localhost/api/native-room')).status, 405);
 assert.equal((await request({ op: 'create', name: 'A', roomName: 'Room', ...identity(1), padding: 'x'.repeat(17000) })).status, 413);
 assert.equal((await mf.dispatchFetch('http://localhost/api/native-room', { method: 'POST', body: '{' })).status, 400);
});
test('Cloudflare durable rooms preserve protocol, authentication, atomic capacity and leaving', async () => {
 const created = await request({ op: 'create', name: 'Owner', roomName: 'Squad', ...identity(1) });
 assert.equal(created.status, 200);
 const owner = created.data;
 const joined = await Promise.all(Array.from({length:8}, (_,i) => request({ op:'join', room:owner.room, invite:owner.invite, name:'Peer '+i, ...identity(i+2) })));
 assert.equal(joined.filter(r=>r.status===200).length, 5);
 assert.equal(joined.filter(r=>r.status===409).length, 3);
 const snapshot = await request({ op:'sync', ...owner });
 assert.equal(snapshot.status,200); assert.equal(snapshot.data.peers.length,6);
 for(const secret of [owner.token, owner.invite, 'tokenHash','inviteHash']) assert.ok(!JSON.stringify(snapshot.data).includes(secret));
 assert.equal((await request({op:'sync',...owner,token:'x'.repeat(32)})).status,401);
 assert.equal((await request({op:'join',room:owner.room,invite:'x'.repeat(32),name:'Stranger',...identity(20)})).status,403);
 assert.equal((await request({op:'leave',...owner})).status,200);
 assert.equal((await request({op:'sync',...owner})).status,401);
});
test('Cloudflare admission limits are shared per IP but independent between callers', async () => {
 const data = {op:'create',name:'A',roomName:'Room',...identity(1)};
 for(let i=0;i<64;i++) assert.equal((await request(data,'198.51.100.8')).status,200);
 assert.equal((await request(data,'198.51.100.8')).status,429);
 assert.equal((await request(data,'198.51.100.9')).status,200);
});
test('Cloudflare durable storage preserves a selected capacity above six', async () => {
 const {data:owner,status}=await request({op:'create',name:'Host',roomName:'Larger room',maxMembers:8,...identity(40)},'192.0.2.70');
 assert.equal(status,200); assert.equal(owner.maxMembers,8);
 for(let i=0;i<7;i++) assert.equal((await request({op:'join',room:owner.room,invite:owner.invite,name:`Friend ${i}`,clientMaxMembers:32,...identity(41+i)},'192.0.2.70')).status,200);
 assert.equal((await request({op:'join',room:owner.room,invite:owner.invite,name:'Extra',clientMaxMembers:32,...identity(60)},'192.0.2.70')).status,409);
 const snapshot=await request({op:'sync',...owner},'192.0.2.70'); assert.equal(snapshot.data.maxMembers,8); assert.equal(snapshot.data.peers.length,8);
});

test('native executable exchanges encrypted Opus audio and chat through Cloudflare coordination', async (context) => {
 const exe = path.resolve('target/release/hush-native.exe');
 if(process.platform!=='win32') { context.skip('Windows native executable test'); return; }
 await new Promise<void>((resolve,reject)=>execFile(exe,['--self-test'],{cwd:process.cwd(),env:{...process.env,HUSH_TEST_SERVER_URL:origin},windowsHide:true,timeout:30000},(error,stdout,stderr)=>{ if(error)reject(Error(stderr||error.message));else resolve(); }));
});
