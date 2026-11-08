import { createHash, randomBytes, timingSafeEqual } from 'node:crypto';
import { isIP } from 'node:net';
import { RoomError } from './errors';
import type { NativeStore } from './native-store';
export type NativePeer = {
  id: string;
  name: string;
  certificate: string;
  endpoints: string[];
  muted: boolean;
  deafened: boolean;
  seen: number;
};
type Member = NativePeer & { tokenHash: string };
type Room = { version: 1; name: string; maxMembers?: number; inviteHash: string; members: Member[]; expires: number };
export const MAX_ROOM_MEMBERS = 32;
function roomCapacity(value: unknown = 6): number {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 2 || value > MAX_ROOM_MEMBERS)
    throw new RoomError(`Room size must be a whole number from 2 to ${MAX_ROOM_MEMBERS}.`);
  return value;
}
export type NativeInput = {
  op?: string;
  room?: string;
  peer?: string;
  token?: string;
  invite?: string;
  name?: string;
  roomName?: string;
  maxMembers?: number;
  clientMaxMembers?: number;
  certificate?: string;
  endpoints?: string[];
  muted?: boolean;
  deafened?: boolean;
};
const TTL = 4 * 60 * 60 * 1000;
const hash = (value: string) => createHash('sha256').update(value).digest('hex');
const secret = () => randomBytes(24).toString('base64url');
function same(value: string | undefined, digest: string) {
  if (!value || !/^[\w-]{32}$/.test(value)) return false;
  return timingSafeEqual(Buffer.from(hash(value)), Buffer.from(digest));
}
function name(value: unknown, label: string, max: number) {
  if (typeof value !== 'string' || !value.trim() || [...value.trim()].length > max)
    throw new RoomError(`${label} must be 1–${max} characters.`);
  return value.trim();
}
function endpoints(values: unknown): string[] {
  if (!Array.isArray(values) || values.length > 8)
    throw new RoomError('Publish at most eight UDP addresses.');
  return values.map((value) => {
    if (typeof value !== 'string') throw new RoomError('Invalid UDP address.');
    const match = /^(\d{1,3}(?:\.\d{1,3}){3}):(\d{1,5})$/.exec(value);
    if (
      !match ||
      isIP(match[1]) !== 4 ||
      Number(match[2]) < 1 ||
      Number(match[2]) > 65535 ||
      match[1] === '0.0.0.0' ||
      match[1].startsWith('224.')
    )
      throw new RoomError('Use IPv4:port for the UDP address.');
    return value;
  });
}
function identity(input: NativeInput) {
  if (
    typeof input.certificate !== 'string' ||
    input.certificate.length > 4096 ||
    !/^[A-Za-z0-9+/]+={0,2}$/.test(input.certificate) ||
    Buffer.from(input.certificate, 'base64').length < 128
  )
    throw new RoomError('A native TLS identity is required.');
  return { certificate: input.certificate, endpoints: endpoints(input.endpoints) };
}
export class NativeRooms {
  constructor(
    private store: NativeStore,
    private createRoomId: () => string = () => randomBytes(9).toString('base64url'),
  ) {}
  async handle(input: NativeInput) {
    if (!input || typeof input !== 'object') throw new RoomError('Invalid native room request.');
    const now = Date.now();
    if (input.op === 'create') {
      const displayName = name(input.name, 'Your name', 24);
      const roomName = name(input.roomName, 'Room name', 48);
      const maxMembers = roomCapacity(input.maxMembers);
      const network = identity(input);
      const room = this.createRoomId();
      const peer = randomBytes(9).toString('base64url');
      const token = secret();
      const invite = secret();
      const data: Room = {
        version: 1,
        name: roomName,
        maxMembers,
        inviteHash: hash(invite),
        expires: now + TTL,
        members: [
          {
            id: peer,
            name: displayName,
            tokenHash: hash(token),
            ...network,
            muted: true,
            deafened: false,
            seen: now,
          },
        ],
      };
      if (!(await this.store.write(`native/rooms/${room}.json`, JSON.stringify(data), null)))
        throw new RoomError('Please try creating the room again.', 503);
      return { room, peer, token, invite, roomName, maxMembers };
    }
    if (!input.room || !/^[\w-]{12}$/.test(input.room))
      throw new RoomError('Invalid native room code.');
    const path = `native/rooms/${input.room}.json`;
    for (let retry = 0; retry < MAX_ROOM_MEMBERS; retry++) {
      const document = await this.store.read(path);
      if (!document) throw new RoomError('Room not found. Ask for a new invitation.', 404);
      const room: Room = JSON.parse(document.value);
      if (room.expires < now) throw new RoomError('This room has expired. Create a new one.', 404);
      const previousMembers = room.members.length;
      room.members = room.members.filter((peer) => now - peer.seen < 120_000);
      let changed = previousMembers !== room.members.length;
      let result: unknown;
      if (input.op === 'join') {
        if (!same(input.invite, room.inviteHash))
          throw new RoomError('This invitation is invalid.', 403);
        const maxMembers = roomCapacity(room.maxMembers);
        if ((input.clientMaxMembers ?? 6) < maxMembers)
          throw new RoomError('Update Hush to join a room larger than six people.', 409);
        if (room.members.length >= maxMembers) throw new RoomError(`This room is full (${maxMembers} people).`, 409);
        const displayName = name(input.name, 'Your name', 24);
        const network = identity(input);
        if (room.members.some((member) => member.certificate === network.certificate))
          throw new RoomError('This identity is already in the room.', 409);
        const peer = randomBytes(9).toString('base64url');
        const token = secret();
        room.members.push({
          id: peer,
          name: displayName,
          tokenHash: hash(token),
          ...network,
          muted: true,
          deafened: false,
          seen: now,
        });
        result = { room: input.room, peer, token, invite: input.invite, roomName: room.name, maxMembers };
        changed = true;
      } else {
        const member = room.members.find((member) => member.id === input.peer);
        if (!member || !same(input.token, member.tokenHash))
          throw new RoomError('Your room session has ended. Join again.', 401);
        if (input.op === 'leave') {
          room.members = room.members.filter((peer) => peer.id !== input.peer);
          changed = true;
          result = { ok: true };
        } else if (input.op === 'sync') {
          const muted = typeof input.muted === 'boolean' ? input.muted : member.muted;
          const deafened = typeof input.deafened === 'boolean' ? input.deafened : member.deafened;
          const addresses = input.endpoints ? endpoints(input.endpoints) : member.endpoints;
          if (
            now - member.seen > 20_000 ||
            muted !== member.muted ||
            deafened !== member.deafened ||
            JSON.stringify(addresses) !== JSON.stringify(member.endpoints)
          ) {
            member.seen = now;
            member.muted = muted;
            member.deafened = deafened;
            member.endpoints = addresses;
            changed = true;
          }
          result = {
            version: 1,
            roomName: room.name,
            maxMembers: roomCapacity(room.maxMembers),
            peers: room.members.map(({ tokenHash: _tokenHash, ...peer }) => peer),
          };
        } else throw new RoomError('Unknown native room action.');
      }
      if (!changed) return result;
      room.expires = now + TTL;
      if (await this.store.write(path, JSON.stringify(room), document.revision)) return result;
      await new Promise((resolve) => setTimeout(resolve, 10 + Math.random() * 30));
    }
    throw new RoomError('Room updates are busy. Please retry.', 503);
  }
  async admit(ip: string) {
    const minute = Math.floor(Date.now() / 60_000);
    const path = `native/admission/${hash(ip).slice(0, 32)}-${minute}.json`;
    for (let retry = 0; retry < 8; retry++) {
      const before = await this.store.read(path);
      const value = before
        ? JSON.parse(before.value)
        : { count: 0, expires: (minute + 2) * 60_000 };
      if (value.count >= 64) return false;
      value.count++;
      if (await this.store.write(path, JSON.stringify(value), before?.revision ?? null))
        return true;
    }
    return false;
  }
}
