import { test } from 'node:test';
import assert from 'node:assert/strict';
import { NativeRooms } from '../server/native-rooms';
import { NativeMemoryStore } from '../server/native-store';
import { RoomError } from '../server/errors';
const identity = (seed: number) => ({
  certificate: Buffer.alloc(256, seed).toString('base64'),
  endpoints: ['127.0.0.1:41642'],
});
type Session = { room: string; peer: string; token: string; invite: string };
const create = (rooms: NativeRooms) =>
  rooms.handle({
    op: 'create',
    name: 'Basil',
    roomName: 'Native squad',
    ...identity(1),
  }) as Promise<Session>;
const join = (rooms: NativeRooms, session: Session, seed: number) =>
  rooms.handle({
    op: 'join',
    room: session.room,
    invite: session.invite,
    name: `Friend ${seed}`,
    ...identity(seed),
  }) as Promise<Session>;
test('native roster authenticates invitations and publishes certificates but never membership secrets', async () => {
  const rooms = new NativeRooms(new NativeMemoryStore());
  const owner = await create(rooms);
  const friend = await join(rooms, owner, 2);
  const snapshot = (await rooms.handle({ op: 'sync', ...friend })) as {
    peers: unknown[];
    version: number;
  };
  assert.equal(snapshot.version, 1);
  assert.equal(snapshot.peers.length, 2);
  assert.ok(JSON.stringify(snapshot).includes(identity(1).certificate));
  for (const secret of [owner.token, owner.invite, friend.token, 'tokenHash', 'inviteHash'])
    assert.ok(!JSON.stringify(snapshot).includes(secret));
  await assert.rejects(
    () => rooms.handle({ op: 'sync', ...friend, token: owner.token }),
    /session has ended/,
  );
  await assert.rejects(
    () =>
      rooms.handle({
        op: 'join',
        room: owner.room,
        invite: 'x'.repeat(32),
        name: 'Stranger',
        ...identity(3),
      }),
    (error: unknown) => error instanceof RoomError && error.status === 403,
  );
});
test('native room admission is atomic under simultaneous joins and bounded to six peers', async () => {
  const rooms = new NativeRooms(new NativeMemoryStore());
  const owner = await create(rooms);
  const results = await Promise.allSettled(
    Array.from({ length: 8 }, (_, index) => join(rooms, owner, index + 2)),
  );
  assert.equal(results.filter((result) => result.status === 'fulfilled').length, 5);
  assert.equal(
    ((await rooms.handle({ op: 'sync', ...owner })) as { peers: unknown[] }).peers.length,
    6,
  );
});
test('native sync avoids writes for unchanged polling and preserves tokens across mute changes', async () => {
  class CountingStore extends NativeMemoryStore {
    writes = 0;
    override async write(path: string, value: string, revision: string | null) {
      this.writes++;
      return super.write(path, value, revision);
    }
  }
  const store = new CountingStore();
  const rooms = new NativeRooms(store);
  const owner = await create(rooms);
  await rooms.handle({ op: 'sync', ...owner });
  await rooms.handle({ op: 'sync', ...owner });
  assert.equal(store.writes, 1);
  await rooms.handle({ op: 'sync', ...owner, muted: false });
  assert.equal(store.writes, 2);
  await rooms.handle({ op: 'leave', ...owner });
  await assert.rejects(() => rooms.handle({ op: 'sync', ...owner }), /session has ended/);
});
test('native API rejects duplicate identities and invalid UDP endpoints', async () => {
  const rooms = new NativeRooms(new NativeMemoryStore());
  const owner = await create(rooms);
  await assert.rejects(() => join(rooms, owner, 1), /already in the room/);
  for (const address of [
    '0.0.0.0:41642',
    '127.0.0.1:0',
    '256.0.0.1:1234',
    'https://example.com',
    '127.0.0.1:99999',
  ])
    await assert.rejects(
      () =>
        rooms.handle({
          op: 'create',
          name: 'Test',
          roomName: 'Room',
          ...identity(2),
          endpoints: [address],
        }),
      /address/,
    );
});
test('native admission limiter enforces the shared per-IP cap', async () => {
  const rooms = new NativeRooms(new NativeMemoryStore());
  const results = [];
  for (let index = 0; index < 21; index++) results.push(await rooms.admit('198.51.100.1'));
  assert.equal(results.filter(Boolean).length, 20);
  assert.equal(results[20], false);
  assert.equal(await rooms.admit('198.51.100.2'), true);
});
