import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { getNativeRooms } from '../server/native-service';
import { RoomError } from '../server/errors';
const port = Number(process.env.PORT ?? 5173);
createServer(async (req, res) => {
  res.setHeader('Cache-Control', 'no-store');
  if (req.url === '/' && req.method === 'GET') {
    res.setHeader('Content-Type', 'text/html');
    res.end(await readFile('public/index.html'));
    return;
  }
  if (req.url === '/favicon.svg' && req.method === 'GET') {
    res.setHeader('Content-Type', 'image/svg+xml');
    res.end(await readFile('public/favicon.svg'));
    return;
  }
  if (req.url !== '/api/native-room' || req.method !== 'POST') {
    res.writeHead(404);
    res.end();
    return;
  }
  res.setHeader('Content-Type', 'application/json');
  try {
    let data = '';
    for await (const chunk of req) {
      data += chunk;
      if (Buffer.byteLength(data) > 16_000) throw new RoomError('Request exceeds 16 KB.', 413);
    }
    const input = JSON.parse(data);
    const rooms = getNativeRooms();
    if (
      (input.op === 'create' || input.op === 'join') &&
      !(await rooms.admit(req.socket.remoteAddress ?? 'local'))
    )
      throw new RoomError('Too many attempts. Wait a minute.', 429);
    res.end(JSON.stringify(await rooms.handle(input)));
  } catch (error) {
    res.statusCode =
      error instanceof RoomError ? error.status : error instanceof SyntaxError ? 400 : 503;
    res.end(
      JSON.stringify({ error: error instanceof Error ? error.message : 'Coordination failed' }),
    );
  }
}).listen(port, '127.0.0.1', () =>
  console.log(`Native Hush coordination: http://127.0.0.1:${port} (no browser client)`),
);
