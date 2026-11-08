import { DurableObject } from 'cloudflare:workers';
import { createHash, randomBytes } from 'node:crypto';
import { NativeRooms, type NativeInput } from '../server/native-rooms';
import { RoomError } from '../server/errors';
import type { Document, NativeStore } from '../server/native-store';
interface Env { ROOMS: DurableObjectNamespace; DEPLOYMENT_ID: string }
const json = (body: unknown, status = 200) => Response.json(body, { status, headers: { 'Cache-Control': 'no-store' } });
export class DurableStore implements NativeStore {
  constructor(private storage: DurableObjectStorage) {}
  async read(path: string) { return (await this.storage.get<Document>(path)) ?? null; }
  async write(path: string, value: string, revision: string | null) {
    const written = await this.storage.transaction(async (tx) => {
      const before = await tx.get<Document>(path);
      if ((before?.revision ?? null) !== revision) return false;
      await tx.put(path, { value, revision: crypto.randomUUID() });
      return true;
    });
    if (written) {
      const expires = JSON.parse(value).expires;
      const alarm = await this.storage.getAlarm();
      if (Number.isFinite(expires) && (!alarm || expires < alarm)) await this.storage.setAlarm(expires);
    }
    return written;
  }
}
export class HushRoom extends DurableObject<Env> {
  async fetch(request: Request) {
    try {
      const data = await request.json() as NativeInput & { admission?: string };
      const rooms = new NativeRooms(new DurableStore(this.ctx.storage), () => data.room!);
      if (new URL(request.url).pathname === '/admit') return json({ allowed: await rooms.admit(data.admission!) });
      return json(await rooms.handle(data));
    } catch (error) {
      return json({ error: error instanceof RoomError ? error.message : 'Room coordination is unavailable. Please retry.' }, error instanceof RoomError ? error.status : 503);
    }
  }
  async alarm() {
    await this.ctx.storage.transaction(async tx => {
      const documents = await tx.list<Document>();
      let next = Infinity;
      for (const [key, document] of documents) {
        const expires = JSON.parse(document.value).expires;
        if (!Number.isFinite(expires) || expires <= Date.now()) await tx.delete(key);
        else next = Math.min(next, expires);
      }
      if (Number.isFinite(next)) await tx.setAlarm(next);
    });
  }
}
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === '/health' && request.method === 'GET') return json({ app: 'hush-native', protocol: 1, provider: 'cloudflare', deploymentId: env.DEPLOYMENT_ID });
    if (url.pathname === '/' && request.method === 'GET') return new Response('Hush room server is ready. Connect with the Hush desktop app.', { headers: { 'Content-Type': 'text/plain; charset=utf-8', 'Cache-Control': 'no-store' } });
    if (url.pathname !== '/api/native-room') return json({ error: 'Not found.' }, 404);
    if (request.method !== 'POST') return json({ error: 'Use POST.' }, 405);
    try {
      if (Number(request.headers.get('content-length')) > 16000) return json({ error: 'Request exceeds 16 KB.' }, 413);
      const reader = request.body?.getReader();
      let size = 0;
      const chunks: Uint8Array[] = [];
      if (reader) { for (;;) { const { value, done } = await reader.read(); if (done) break; size += value.byteLength; if (size > 16000) { await reader.cancel(); return json({ error: 'Request exceeds 16 KB.' }, 413); } chunks.push(value); } }
      const bytes = new Uint8Array(size); let offset = 0; for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
      const input = JSON.parse(new TextDecoder().decode(bytes)) as NativeInput;
      if (!input || typeof input !== 'object' || Array.isArray(input)) return json({ error: 'Invalid room request.' }, 400);
      if (input.op === 'create' || input.op === 'join') {
        const ip = request.headers.get('CF-Connecting-IP') ?? 'local';
        const admission = createHash('sha256').update(ip).digest('hex');
        const limiter = env.ROOMS.get(env.ROOMS.idFromName('admission:' + admission));
        const limit = await limiter.fetch(new Request('https://internal/admit', { method: 'POST', body: JSON.stringify({ admission }) }));
        if (!limit.ok) return json({ error: 'Room admission is temporarily unavailable.' }, 503);
        if (!(await limit.json() as { allowed: boolean }).allowed) return json({ error: 'Too many attempts. Wait a minute and retry.' }, 429);
      }
      const room = input.op === 'create' ? randomBytes(9).toString('base64url') : input.room;
      if (typeof room !== 'string' || !/^[\w-]{12}$/.test(room)) return json({ error: 'Invalid native room code.' }, 400);
      const object = env.ROOMS.get(env.ROOMS.idFromName('room:' + room));
      return object.fetch(new Request('https://internal/room', { method: 'POST', body: JSON.stringify({ ...input, room }) }));
    } catch (error) { return json({ error: error instanceof SyntaxError ? 'Invalid JSON.' : 'Room coordination is unavailable. Please retry.' }, error instanceof SyntaxError ? 400 : 503); }
  }
} satisfies ExportedHandler<Env>;
