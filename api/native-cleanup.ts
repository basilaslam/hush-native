import type { IncomingMessage, ServerResponse } from 'node:http';
import { del, get, list } from '@vercel/blob';
export default async function handler(req: IncomingMessage, res: ServerResponse) {
  const expected = process.env.CRON_SECRET;
  if (!expected || req.headers.authorization !== `Bearer ${expected}`) {
    res.writeHead(401);
    res.end();
    return;
  }
  try {
    let cursor: string | undefined;
    let removed = 0;
    let inspected = 0;
    do {
      const page = await list({ prefix: 'native/', limit: 100, cursor });
      for (const blob of page.blobs) {
        if (++inspected > 1000) break;
        const object = await get(blob.pathname, { access: 'private', useCache: false });
        if (!object?.stream) continue;
        const value = JSON.parse(await new Response(object.stream).text());
        if (typeof value.expires === 'number' && value.expires < Date.now()) {
          await del(blob.url, { ifMatch: object.blob.etag });
          removed++;
        }
      }
      cursor = page.hasMore ? page.cursor : undefined;
    } while (cursor && inspected < 1000);
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({ removed, inspected }));
  } catch (error) {
    console.error(
      'Native cleanup failed',
      error instanceof Error ? error.message : 'Unknown error',
    );
    res.writeHead(503);
    res.end();
  }
}
