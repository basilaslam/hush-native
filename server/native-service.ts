import { NativeRooms } from './native-rooms';
import { NativeBlobStore, NativeMemoryStore } from './native-store';
let rooms: NativeRooms | undefined;
export function getNativeRooms() {
  if (rooms) return rooms;
  if (
    !process.env.BLOB_READ_WRITE_TOKEN &&
    (process.env.VERCEL || process.env.NODE_ENV === 'production')
  )
    throw new Error('Attach a PRIVATE Vercel Blob store to enable native room coordination.');
  return (rooms = new NativeRooms(
    process.env.BLOB_READ_WRITE_TOKEN ? new NativeBlobStore() : new NativeMemoryStore(),
  ));
}
