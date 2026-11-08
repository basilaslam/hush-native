# Hush
<!-- impeccable:product-schema 1 -->
## Platform
Native Rust desktop, targeting Windows. macOS is used for local development and compilation.
## Stack
User-required native Rust client; egui/eframe with a native Glow renderer, CPAL audio devices, Opus compression and Quinn QUIC transport. Room coordination can run on Cloudflare Workers with SQLite-backed Durable Objects, or on the existing Vercel Functions/private Blob backend. A separate terminal script prepares host-only tools, signs in through official Wrangler, deploys and verifies the server, then prints its address. The native client bundles no deployment runtime. The web/Tauri prototype has been replaced.
## Users and Purpose
Small squads playing and streaming games on Windows want independent voice chat with low CPU, memory and frame-time overhead.
## Capabilities and Constraints
Voice-first, creator-configured 2–32-person rooms, invitations, native mute/deafen/PTT, device selection, local mic test, gain/volume, peer chat and measured network health. No external STUN/TURN/Redis dependency. Direct IPv4 UDP requires reachable endpoints: LAN, successful UPnP mapping or manual port forwarding. CGNAT/restricted UDP can prevent calls; no relay fallback is available with the current direct-QUIC transport.
## Performance
One encode per frame shared across peers, bounded audio/network queues, isolated coordination task, event-driven native rendering, gaming mode enabled by default. Zero FPS loss and superiority over Discord have not been established on the user's Windows PC.
## Identity and Design
Hush is a working name. Preserve the established forest/leaf/cream palette and compact room/navigation/voice-dock structure. The application uses embedded static Manrope fonts and native controls rather than a WebView.
## Open Work Requiring External Environments
Deploy private Vercel Blob/Functions; compile and install on Windows; test WASAPI, global F8, firewall behavior, cross-router connectivity and game performance. Screen sharing, overlays and advanced speech processing remain outside this voice-first release.
