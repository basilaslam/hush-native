import { get, put, BlobPreconditionFailedError, BlobError } from '@vercel/blob';
export type Document = { value: string; revision: string };
export interface NativeStore {
  read(path: string): Promise<Document | null>;
  write(path: string, value: string, revision: string | null): Promise<boolean>;
}
export class NativeMemoryStore implements NativeStore {
  private data = new Map<string, Document>();
  async read(path: string) {
    return this.data.get(path) ?? null;
  }
  async write(path: string, value: string, revision: string | null) {
    if ((this.data.get(path)?.revision ?? null) !== revision) return false;
    this.data.set(path, { value, revision: crypto.randomUUID() });
    return true;
  }
}
export class NativeBlobStore implements NativeStore {
  async read(path: string) {
    // Room membership must not come from the Blob CDN's stale cache.
    const document = await get(path, { access: 'private', useCache: false });
    if (!document?.stream) return null;
    return { value: await new Response(document.stream).text(), revision: document.blob.etag };
  }
  async write(path: string, value: string, revision: string | null) {
    try {
      await put(path, value, {
        access: 'private',
        addRandomSuffix: false,
        contentType: 'application/json',
        allowOverwrite: revision !== null,
        ...(revision ? { ifMatch: revision } : {}),
      });
      return true;
    } catch (error) {
      if (error instanceof BlobPreconditionFailedError) return false;
      if (revision === null && error instanceof BlobError && (await this.read(path))) return false;
      throw error;
    }
  }
}
