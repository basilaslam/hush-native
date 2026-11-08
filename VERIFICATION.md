# Hush native verification

The shipping application is native Rust. The browser/Tauri prototype and its React dependencies have been removed.

## Logo update — v0.2.1

- The conversation-H SVG master, transparent marks and outlined wordmark were visually inspected. PNG exports were rasterized at high density; native RGBA dimensions/transparency and all six Windows ICO sizes were verified.
- [Windows CI](https://github.com/basilaslam/hush-native/actions/runs/42445824727) passed at `0bd92e4f9b84403067a4c159b407a05e5674954f`: formatting, Clippy, seven Rust tests, five service tests/build, optimized compilation, static-runtime imports, synthetic QUIC/Opus/chat integration and NSIS packaging.
- Downloaded build artifacts matched CI SHA-256 checksums. The Windows x64 GUI executable contains the updated icon resource and imports no separate Microsoft C runtime. No physical Windows UI, microphone or gaming benchmark was performed for this artwork update.
- Cloudflare and relay recommendations in `docs/PRIVATE-USE.md` are a proposed architecture. The transport and deployed-service status remain unchanged by this release.

## Checks completed locally

- Native macOS development build compiled and its room/settings interface was inspected in a real native window.
- Optimized native macOS release compiled successfully. The stripped arm64 executable is approximately 7.9 MB. This is a macOS build size, not a Windows size or runtime-memory claim.
- Seven Rust unit tests passed: invitation and server URL validation; resampler duration at common audio-device rates; real Opus encode/decode and bounded jitter buffering; mute/deafen/PTT transmission rules; sequence loss/reordering accounting; mutually authenticated QUIC transport and rejection of an untrusted client certificate.
- Release Clippy checks compile all targets with warnings treated as errors.
- Five TypeScript room-service tests passed: private admission/roster authentication; six-person capacity under simultaneous joins; avoiding writes for unchanged polls; rejecting duplicate identities and malformed UDP endpoints; per-IP admission limits.
- TypeScript/server build passed. npm audit reported zero vulnerabilities in installed dependencies.
- Native end-to-end self-test passed against the local HTTP service. Two real per-session identities created/joined a room, read each other's certificate/address from the API, authenticated both sides of a QUIC connection, exchanged and decoded 12 synthetic 20 ms Opus frames, exchanged reliable chat, left, and verified the old membership token was rejected.
- The self-test used no microphones, speakers, browser, external STUN/TURN server or cloud storage. It confirms the native protocol and local coordination path; it is not a physical audio-quality or public-internet test.
- Formatting and Git whitespace checks passed.

During local builds the machine ran out of disk space. Only this project's generated Rust build artifacts and unused dependency downloads were removed; building resumed with bounded build parallelism. The native executable and source remain available.

## Limits and external checks

- After local development, [Windows CI](https://github.com/basilaslam/hush-native/actions/runs/42398005603) passed at commit `5a5391c9cc88caa7a1e7d91760375ed92795a791`: formatting, Clippy, seven Rust unit tests, five service tests/build, release compilation, static-runtime import verification, native synthetic audio/chat and NSIS packaging. Installing the generated setup program on a physical Windows desktop, WASAPI devices, native F8 and game privilege interactions remain unverified.
- The Windows workflow embeds an app icon, uses static Opus and packages a per-user installer without WebView2. Distribution signing is not configured.
- No Vercel deployment or private Blob store was provisioned. The SDK types, origin-read option and conditional-write interface were checked; actual deployed Blob contention, retention, quotas and latency still need a configured cloud test.
- UPnP mapping and renewal code is implemented but was not exercised against the user's router. No router mappings or firewall changes were made during these tests.
- LAN, cross-router/CGNAT behavior and six physical devices still require testing. This Vercel-only design intentionally has no media relay and cannot guarantee a connection through restricted NAT.
- Microphone capture/playback are implemented through CPAL but physical microphones, speakers and subjective audio quality were not tested in this session.
- Native FPS impact, 1% lows, CPU/RAM under a live Windows call and upload contention have not been benchmarked. Low overhead is an engineering goal, not a measured comparison or a zero-FPS-loss guarantee.

## Reproduce

```sh
npm ci
npm test
npm run build
cargo test --locked -p hush-native
cargo clippy --locked -p hush-native --all-targets -- -D warnings
cargo build --release --locked -p hush-native
```

Keep `npm run dev` running, then execute `target/release/hush-native --self-test` (append `.exe` on Windows) for the local native end-to-end test. The self-test creates disposable local room metadata and uses synthetic input only.
