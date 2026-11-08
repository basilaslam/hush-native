Hush v0.3.0 adds Cloudflare server deployment from the native app and refreshes the lobby, settings and connection interface.

- **Deploy server** opens the official Wrangler browser sign-in, supports account selection, deploys your private room coordinator and saves its verified HTTPS address.
- Deployment runs separately from the native voice/audio loop. Windows credential storage protects the Wrangler sign-in profile.
- Vercel coordination remains available. Voice and chat still use direct encrypted QUIC; this release does not add ICE or TURN relay fallback.

Download **Hush-0.3.0-Windows-Setup.exe**, or extract the **complete** Windows Portable ZIP and run `Hush/hush-native.exe`. Keep its `deployment` directory beside the executable; it contains Node, Wrangler and the server payload needed by Deploy server. Normal voice processing remains native Rust. The source ZIP is also included for development on Mac or Windows.

Validation runs Windows formatting, Clippy, native unit tests, service/deployment tests, local Cloudflare Worker coordination with native Opus/chat integration, static-runtime import inspection, release compilation, bundled deployment-runtime startup and NSIS packaging. Download hashes are listed in SHA256SUMS.txt.

This is an unsigned preview. A new live Cloudflare account deployment, cross-router calls, physical audio devices, global F8 inside games and gaming FPS require testing on your own setup. Cloudflare quotas apply; restrictive NAT can still block direct connections.
