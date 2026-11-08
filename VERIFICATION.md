# Hush native verification

The shipping application is native Rust. The browser/Tauri prototype and its React dependencies have been removed.

## Small client and standalone terminal setup — v0.5.0

- Removed the native deployment module, in-app Cloudflare setup controls and bundled Node/Wrangler payload. Voice, room-size selection and the GitHub updater remain native. New installers remove the old app-owned deployment folder; host state and desktop preferences live outside it.
- Local Clippy, formatting, service build and 13 native tests passed. Eighteen service/setup tests ran locally: 17 passed and the Windows-only native/Cloudflare integration case was skipped. Setup tests cover secure environment isolation, URL/protocol/identity verification, selected-account deployment, saved-worker reuse and a dry run that invokes no sign-in or cloud writes.
- The macOS launcher downloaded and checksum-verified a private Node 24 runtime. A fresh host-tool installation with that runtime passed the Worker dry run. No Cloudflare sign-in or actual cloud deployment was performed during these checks.
- CI now validates the shipped PowerShell/Bash launchers against the separate server bundle, enforces a 12 MB desktop-download budget, and tests silent installation over a simulated old payload while checking preserved preferences/server state. Actual end-user browser OAuth and network calls remain subject to the existing physical-device limits below.

## Configurable rooms and GitHub updates — v0.4.1

- [Windows CI](https://github.com/basilaslam/hush-native/actions/runs/42455900480) passed its build job at `698ae63514f3220b7d05d9900bb2d834f4314f34`: 15 Rust tests, 17 service/deployment tests, optimized compilation, native audio/chat against local Vercel-compatible and Cloudflare coordination, static-runtime inspection, bundled runtime startup and NSIS/portable packaging. The earlier unpublished v0.4.0 tag caught an inherited PowerShell module-path issue; the handoff now uses built-in .NET SHA-256 and a clean module path.
- The tag-triggered publication job passed. Published installer/portable/source/license checksums were verified after download. The portable archive contains the complete deployment payload; the source archive matches the release commit byte-for-byte. Live GitHub installer metadata (digest, URL and size) matches the downloaded file and is compatible with the updater's validation.
- Local release Clippy, formatting and TypeScript/server builds passed. Fourteen native tests passed, including real Opus decoding across all 31 remote-stream slots, old-preference defaults, update version/channel selection, downgrade rejection, metadata bounds, asset URL/digest validation and checksum matching.
- Sixteen Node/service/deployment tests passed locally; the Windows native executable/Cloudflare integration case is skipped on macOS and runs in Windows CI. Added tests cover concurrent admission above six, exact selected capacity, invalid capacity, preventing joiners from enlarging rooms and requiring current clients for larger rooms.
- Windows CI additionally exercises the real PowerShell handoff with compiled process/installer stubs: wait for exit, paths containing spaces/apostrophes, restart and refusal of a modified installer. This is distinct from a full user-desktop upgrade test.
- The new maximum is 32 total participants, default six. Per-IP admission allows 64 attempts/minute so a squad behind one NAT can join. Higher room sizes increase mesh work; physical 32-PC calls and gaming performance have not been measured.
- Automatic checks contact the fixed public GitHub repository at startup/every six hours. Downloads and installation require the app controls. SHA-256 metadata and release-manifest matching validate integrity against the trusted GitHub account; independent code signing is not configured.
- Existing v0.3.0 installations require one manual upgrade. Existing room coordinators must be redeployed for capacities above six. Client update installation does not redeploy the cloud service.

## Downloaded v0.3.0 source and release validation

- Imported `Hush-v0.3.0-source-for-Mac.zip` from Downloads, SHA-256 `ee03e7560b33eebb8926aa53c0a52c5bf6f1b0036d622727057fc9782d3454f4`. Downloaded application source files were preserved exactly. Release preparation aligned the root package version, updated the workflow and documented complete portable packaging.
- [Windows build and validation](https://github.com/basilaslam/hush-native/actions/runs/42452903265) passed at `96ec7de2a52a07a16b5852412b9b6d1417b4e92a`: Worker/server builds, formatting, Clippy, native tests, optimized compilation, service/deployment tests, native QUIC/Opus/chat through local Cloudflare coordination, static-runtime imports, deployment-runtime startup and NSIS packaging.
- The portable package now includes the complete Node/Wrangler deployment helper beside the native executable. The source ZIP is generated from the exact build commit. Release publication is a separate job gated on successful build/tests and checksum verification.
- Published installer/portable checksums, Windows x64 GUI/static-runtime imports and the complete portable deployment payload were verified after download. The source ZIP was normalized to LF line endings for Mac and checked byte-for-byte against the release commit; its updated checksum is published with the other downloads.
- The local Windows and live-account verification records below were retained from the downloaded source. This publication run uses CI and does not perform a new live Cloudflare deployment or game-performance benchmark.

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

## Windows form refresh — local v0.2.2 (8 November 2026)

- Built an optimized Windows x64 application locally using Rust 1.99.1 and Visual Studio 2022 C++ Build Tools. Installed the generated per-user NSIS package and checked that the installed executable matches the local build byte-for-byte.
- Seven native tests and five room-service tests passed. TypeScript checking and service bundling passed. Release Clippy with all targets and warnings denied passed, as did Rust formatting.
- The native self-test authenticated two local QUIC peers, exchanged and decoded 12 synthetic Opus frames, exchanged reliable chat, left the room and rejected the expired membership token. The log contains an older hardcoded “this Mac” phrase; this run was on Windows and that phrase is not a platform measurement.
- The installed native microphone test started successfully, displayed a live input level and stopped cleanly. This confirms device initialization and capture; no subjective playback quality or acoustic processing claim is made.
- Reviewed the lobby, settings cards, focused fields, invitation modal, connection/help pages, normal window and compact window. The application pins its dark theme, uses shared padded fields, measures icon/label spacing, constrains cards to the available panel and preserves independent scroll positions per page.
- Production coordination, internet calls between separate routers, global F8 inside games, multiple-monitor/DPI combinations and game-performance benchmarks remain outside these local checks. No cloud deployment or router/firewall configuration was changed.

## Cloudflare deployment — local v0.3.0 (8 November 2026)

- Added the installed Windows Deploy server flow, browser OAuth through official Wrangler, a dedicated encrypted credential profile, account selection, automatic Worker deployment, health/identity validation and saving the verified server address.
- Eight Rust tests passed. Release build, formatting, Clippy across all targets with warnings denied, root TypeScript and Cloudflare Worker type checks passed.
- Cloudflare runtime tests exercise room admission/authentication, concurrent six-person capacity, expiry/leave behavior, malformed requests and per-IP admission limits. The native executable also authenticates QUIC peers and exchanges synthetic Opus audio and chat using that local Worker for coordination.
- Deployment helper tests validate URL filtering, deployment identity/protocol checks, redirect rejection, OAuth scope compatibility with the bundled CLI and Windows configuration resolution when APPDATA is absent.
- Built and installed the v0.3.0 Windows application and bundled Node/Wrangler payload. Actual browser OAuth testing caught and fixed explicit offline_access scope rejection and Windows APPDATA fallback for the secure keyring module. Credentials never fell back to plaintext.
- Live cloud deployment is being verified after user browser consent; its result is recorded below when available. Cross-router physical calls, gaming performance and multi-device audio remain unverified.

- Final result: all 14 Node/service/deployment tests and 8 Rust tests passed (22 total). Browser consent completed, encrypted credentials were saved, and the installed button deployed a live Cloudflare Worker. Its `/health` endpoint matched the expected application, protocol and deployment nonce. Hush populated and saved the server address automatically.
- The installed Windows executable passed its native end-to-end self-test against that live Worker: real room creation/join, certificate-based QUIC authentication, 12 synthetic Opus frames decoded, reliable chat and leave/membership checks. This uses two peers on this PC and does not establish cross-router connectivity.
