# Current implementation (v0.3.0)

Cloudflare coordination and the Windows in-app Deploy server flow are now implemented. The host signs in through the official Wrangler browser flow; Hush deploys a Worker with SQLite Durable Objects, verifies it and saves the address. Each friend still needs the same server address and room invitation.

The networking roadmap below contains proposals: WebSocket coordination, ICE/STUN and TURN relay fallback are not implemented. Current audio and chat use direct encrypted QUIC, with UPnP or manual UDP port forwarding where needed. Cloudflare deployment alone does not solve restricted NAT connectivity.

---

# Hosting Hush for a private squad

Recommendation checked on 8 November 2026. This is a proposed next architecture. The current Windows preview still uses Vercel coordination and direct QUIC; it does not implement Cloudflare, ICE or TURN.

## Recommended setup

Keep the native Rust interface, CPAL audio, Opus codec, push-to-talk and small invitation-only rooms. Use a Cloudflare Worker with one SQLite-backed Durable Object per room for admission and membership. Deliver roster changes over hibernating WebSockets rather than polling an object store. Do not send voice through those coordination WebSockets.

For internet connectivity, use a native WebRTC transport with ICE: try direct encrypted audio paths, then fall back to TURN when direct connectivity fails. This can remain a Rust desktop application without a browser or WebView. A QUIC-specific relay adapter is another possible implementation, but simply pasting a TURN address into the present Quinn client will not add relay support.

Issue expiring TURN credentials only after room admission. Keep Cloudflare API tokens and TURN credential-generation keys on the service. Do not put permanent relay credentials into a downloadable executable. See [Cloudflare's credential guidance](https://developers.cloudflare.com/realtime/turn/generate-credentials/).

Cloudflare offers free allowances for [SQLite-backed Durable Objects](https://developers.cloudflare.com/durable-objects/platform/pricing/) and [Realtime TURN](https://developers.cloudflare.com/realtime/sfu/platform/pricing/). A small voice-only group is likely to fit those allowances, but that is a usage estimate, not a free-service guarantee. Workers/Objects and Realtime have separate quotas. Realtime is metered beyond its shared SFU/TURN allowance; review account billing before enabling it. Idle WebSocket hibernation must be implemented correctly to avoid unnecessary compute duration.

## Keeping Vercel

Vercel is a reasonable host for HTTPS room admission and signaling. The limitation in the current implementation is its storage access pattern: frequent uncached Blob reads and periodic heartbeat writes. Blob's free operation allowance can be exhausted during regular gaming sessions, after which Hobby storage access stops until the allowance resets. See [Vercel Blob pricing and Hobby limits](https://vercel.com/docs/vercel-blob/usage-and-pricing/).

Keeping Vercel would therefore mean changing the room/presence store or accepting paid usage, and adding NAT traversal/relay support independently. Changing the coordinator alone does not make direct voice connections succeed through CGNAT.

## What needs testing before the group relies on it

Verify direct and forced-relay calls between separate home networks, including a CGNAT/mobile connection; test reconnects and expired credentials. On the actual Windows gaming PCs, test devices, F8 while the game has focus, CPU/RAM, upload traffic and frame-time percentiles while streaming. A native app reduces some software overhead, but an upload-saturated connection can still lose packets. No provider or transport change establishes a zero-FPS-loss claim without those measurements.
