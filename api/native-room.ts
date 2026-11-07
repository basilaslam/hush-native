import type { IncomingMessage, ServerResponse } from 'node:http';
import type { NativeInput } from '../server/native-rooms';
import { getNativeRooms } from '../server/native-service';
import { RoomError } from '../server/errors';
type Request = IncomingMessage & { body?: NativeInput };
type Response = ServerResponse & { status(code: number): Response; json(body: unknown): Response };
export default async function handler(req: Request, res: Response) {
  res.setHeader('Cache-Control', 'no-store');
  if (req.method !== 'POST') return res.status(405).json({ error: 'Use POST.' });
  try {
    if (Buffer.byteLength(JSON.stringify(req.body ?? {})) > 16_000)
      return res.status(413).json({ error: 'Request exceeds 16 KB.' });
    const rooms = getNativeRooms();
    if (
      (req.body?.op === 'create' || req.body?.op === 'join') &&
      !(await rooms.admit(
        String(req.headers['x-forwarded-for'] ?? req.socket.remoteAddress ?? 'unknown')
          .split(',')[0]
          .trim(),
      ))
    )
      return res.status(429).json({ error: 'Too many attempts. Wait a minute and retry.' });
    return res.status(200).json(await rooms.handle(req.body!));
  } catch (error) {
    if (error instanceof RoomError) return res.status(error.status).json({ error: error.message });
    console.error(
      'Native coordination failed',
      error instanceof Error ? error.message : 'Unknown error',
    );
    return res
      .status(503)
      .json({
        error: 'Room coordination is unavailable. Verify the private Vercel Blob configuration.',
      });
  }
}
