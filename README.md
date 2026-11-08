# Hush — native Rust voice

<img src="assets/brand/hush-icon-512.png" alt="Hush speech H app icon" width="96" height="96">

[Download Windows preview](https://github.com/basilaslam/hush-native/releases) · [Source repository](https://github.com/basilaslam/hush-native)

A standalone desktop voice app for a small gaming squad. The shipping client is **Rust + egui/eframe + CPAL + Opus + Quinn**. The voice client has no browser, WebView, Tauri, Electron or Discord dependency. The Windows installer bundles Node and the official Wrangler CLI for optional server deployment.

**Windows is the target platform.** The source also compiles on macOS for development. The native executable talks to your configured coordination server and room participants. Deploy a Cloudflare Worker with SQLite Durable Objects directly from the app, or use Vercel Functions with a **private Vercel Blob store**. No Redis, external STUN service, or TURN relay is used.

## Run the native client

Install Rust stable (1.90+), CMake and the platform C/C++ build tools. On Windows install Visual Studio Build Tools with Desktop development with C++ and a Windows SDK. No WebView2 installation is required.

```sh
cargo run --release --locked -p hush-native
```

If Node/npm is installed, equivalent commands are available:

```sh
npm run native:dev
npm run native:build
```

`native:dev` uses an unoptimized development build. Use the release executable for actual gaming. The program is written to `target/release/hush-native.exe` on Windows, or `target/release/hush-native` on macOS/Linux.

Open **Voice & settings** in the app, click **Deploy server**, complete Cloudflare sign-in and consent in your browser, then choose your name and devices. Hush verifies the new server and saves its address automatically. You can also enter an existing Cloudflare or Vercel HTTPS origin and save. Create a room or join with a `hush1.…` invitation. Each participant needs the native app and the same coordination deployment. Browser-demo invitations from the earlier prototype are incompatible.

## Deploy a server from Hush (Windows v0.4.1)

Only the host needs a Cloudflare account. Click **Voice & settings → Deploy server** and complete the official Wrangler browser sign-in. Choose an account if you have several. Hush deploys a Worker with SQLite Durable Objects, checks its deployment identity and native protocol, then saves its HTTPS address. Share that server address and the room invitation with your friends; invitations currently do not include the server address.

The helper uses a dedicated Wrangler OAuth profile named `hush-native`. Credentials are encrypted with an encryption key held in Windows Credential Manager; Hush refuses plaintext fallback. No API token needs to be pasted into the app. Deployment runs separately from audio and rendering. Retry updates the same generated Worker for that account. Cancel stops local setup; resources already created remain in your account.

Cloudflare account usage limits and pricing apply. The app does not enable a paid plan automatically. New accounts may need Workers onboarding and a workers.dev subdomain in the dashboard. Voice and chat remain direct peer to peer; deploying coordination does not add a media relay.

## Deploy the optional Vercel service

1. Import this repository into Vercel. `vercel.json` builds the coordination service and a small information page, not a browser client.
2. In the same Vercel project, create and attach a **private Vercel Blob store**. Vercel injects `BLOB_READ_WRITE_TOKEN`. Public Blob storage is not supported for room metadata.
3. Set a server-side `CRON_SECRET` for the daily expiry cleanup route.
4. Deploy. Enter the resulting HTTPS origin in each native app’s Settings. Optionally set `HUSH_SERVER_URL` when compiling the executable to embed a default origin; end users can change it later.

Vercel service secrets do not belong in desktop preferences. `BLOB_READ_WRITE_TOKEN` and `CRON_SECRET` stay on Vercel.

The API uses `/api/native-room` for admission, roster refresh, status and leaving. Private Blob reads explicitly bypass CDN caching; conditional ETag writes prevent concurrent room joins from overwriting each other. Unchanged polls do not write to Blob. Live rooms refresh their expiry; inactive peers expire after two minutes. The cleanup job removes expired metadata. The bounded daily sweep inspects at most 1,000 documents, so a larger public service needs a scalable retention strategy. Logical room expiry is enforced even before physical cleanup.

See [Hosting Hush for a private squad](docs/PRIVATE-USE.md) for deployment and connectivity considerations.

## Direct P2P connectivity

Voice and chat travel directly over encrypted QUIC/TLS, independently of the coordination service. This is not an HTTP voice relay.

- On the same LAN, direct addresses can usually connect after allowing the app through the local firewall.
- For internet calls the app tries **UPnP** on your own router. It opens a temporary UDP mapping, renews it for long sessions and best-effort removes it on exit. Router mapping can be disabled in Settings. No existing mapping is removed when a new mapping attempt fails.
- If UPnP is unavailable, forward the configured UDP port (default **41642**) to your PC and enter your public **IPv4:port** in Settings. Configure Windows Firewall for that port/network as appropriate. The app does not silently alter firewall rules.
- Carrier-grade NAT, double NAT, restrictive corporate/mobile networks and blocked UDP can prevent connection. There is no universal NAT traversal or relay fallback in the current direct P2P architecture. Moving voice through short serverless HTTP calls would add buffering and is not implemented.
- IPv4 endpoints are currently supported. A pure IPv6-only network is not supported by this version.

The Connection screen reports the actual route/address, QUIC network round trip and a cumulative voice sequence-gap loss estimate. Failed peers keep retrying with a concrete explanation. Room-service outages do not block the media loop or close existing peer connections. If room membership itself expires, clients must rejoin.

## Native features

- Private rooms with a creator-selected size of 2–32 people (default six); capability invitations, no account registration.
- Native rendering with a dark forest/cream design, locally embedded Manrope fonts and native accessibility semantics.
- Native microphone capture and playback through CPAL (WASAPI on Windows).
- Mono Opus voice at 24, 32 or 64 kbps per peer; 20 ms frames; codec complexity 3; VBR, DTX and in-band FEC.
- One Opus encode per microphone frame for the entire squad, rather than one encode per peer. No encoding while no peers are connected.
- Bounded sample rings in device callbacks; no network I/O or mutex acquisition in the steady-state callbacks.
- Dedicated audio worker with continuous device-rate resampling, bounded jitter buffering, packet loss concealment/FEC and mixing for up to 31 remote streams.
- Mute, deafen (also blocks microphone transmission), microphone gain and output volume.
- Global **F8** push-to-talk, registered only during a PTT call; in-app hold-to-talk fallback if F8 is unavailable. Games with higher Windows privilege levels may intercept a global shortcut: test your actual setup.
- Device selection and local microphone level testing. Device changes require leaving the room. The microphone test is disabled during calls.
- Gaming mode on by default: no live microphone-meter repaints or decorative animation loop. The interface repaints for input, room events and periodic connection stats.
- Temporary chat over reliable peer streams. Messages are not stored on the coordination server; joining later does not retrieve history.
- A compact native window.

This release is voice-first. There is no screen sharing, overlay, recording, acoustic echo cancellation or neural noise suppression. Use headphones. Those features have distinct performance and implementation costs and are not represented as working controls.

## Room size and GitHub updates

Choose **Room size** in the lobby before creating a room. The server enforces 2–32 people, including the creator; six remains the default. Capacity is fixed for that room. Create a new room to change it. Unlimited mesh rooms are not supported: every participant sends voice to every other participant, so larger rooms increase CPU and upload traffic. The upper setting is not a performance guarantee.

To use rooms above six, **all participants need v0.4.1 or newer**, and the host must run **Deploy server** again (or redeploy Vercel) with the updated source. Older rooms and clients retain their six-person defaults. Updating the app does not automatically update an already deployed room server.

In **Voice & settings → App updates**, Hush checks this public GitHub repository at startup and every six hours. Disable automatic checks or exclude preview releases with the checkboxes and Save settings. Preview updates are included by default because this app currently ships preview releases. Checks do not need a GitHub account/token; API/network failures are recoverable and do not close calls.

On Windows, choose **Download update**, then **Install update & restart**. Download/install controls are disabled during calls, microphone testing and server setup. Updates are not downloaded or installed automatically. Installation waits for Hush to exit, verifies the installer again, runs it silently in the same writable app folder, and restarts Hush. Preferences remain in the normal user configuration folder. Portable users also receive the installer-based update in their existing folder. Read-only folders require a manual installer download. Failed installation details are surfaced at the next launch.

Downloads use bounded HTTPS requests to this repository and GitHub asset hosts. The app checks both GitHub's asset SHA-256 digest and `SHA256SUMS.txt`, rejects mismatches/oversized files, and refuses downgrades. These hashes check integrity; they are not independent publisher signatures. The trusted GitHub repository/account remains the update trust boundary. The Windows preview is still unsigned. Automatic installation is Windows-only.

**v0.3.0 has no updater:** install v0.4.1 manually once. Future releases can then be applied from the app. Maintainers bump the versions and release notes, commit, then push a matching `v…` tag; GitHub Actions tests/builds and publishes the preview. Manual workflow dispatch with `publish_release` remains available.

## Security and privacy

Each room session generates a fresh certificate/private key. HTTPS room membership publishes certificates to the squad; native QUIC clients trust only the expected peer certificate, and servers require certificates belonging to room participants. There is no accept-any-certificate verifier. Your configured server is the trusted coordinator for membership and certificate distribution; a compromised coordinator remains inside the trust boundary.

Audio datagrams and chat streams are encrypted by QUIC/TLS. Server-side storage contains expiring room names, display names, public certificates, UDP endpoint addresses, mute/deafen flags and token hashes. It does not contain audio, private keys or chat. Room session tokens and keys live in client memory; preferences contain only display/device/network settings. Deployment OAuth credentials are separately encrypted by Wrangler.

Anyone with an invitation can join, and direct P2P exposes participants’ network addresses to each other. Keep invitations among trusted friends. There is no host kick or invitation revocation in this release. Admission attempts, room size, payloads, stream sizes and queue capacities are bounded. Cloudflare usage or Vercel/Blob costs depend on activity; this is not promised to be free at scale.

## Windows installer

On Windows, prepare the deployment payload with `npm ci`, `npm ci --prefix deployment`, and `npm run cloudflare:build`, then build with `cargo build --release --locked -p hush-native`. Opus links statically by default on Windows, and `.cargo/config.toml` selects the static Microsoft C runtime for the x64 Windows build. For the optional per-user installer install NSIS, then run:

```powershell
makensis packaging/windows/installer.nsi
```

The executable and `packaging/windows/Hush-0.4.1-Windows-Setup.exe` require no browser runtime. The installer runs without administrator rights and preserves preferences when uninstalling. Distribution signing is not configured.

Alternatively, push this repository to GitHub and run **Actions → Build native Windows app**. Supply an optional default coordination origin. The workflow runs checks/tests, builds the native executable and NSIS installer, and packages a portable ZIP with the complete deployment helper, a source ZIP and checksums. Enable **publish_release** to publish a GitHub preview release only after all checks pass; otherwise it uploads the packages as a workflow artifact.

Windows x64 builds and installer packaging have passed [GitHub Actions for v0.4.1](https://github.com/basilaslam/hush-native/actions/runs/42455900480), including the Windows update-handoff test, Rust/service checks, native synthetic audio/chat through local Cloudflare coordination, the bundled deployment runtime, and verification that no separate Microsoft C runtime is imported by the Rust client. Download the Windows preview from [Releases](https://github.com/basilaslam/hush-native/releases). Physical audio, global F8 inside games, cross-router behavior and gaming performance require testing on your gaming PCs.

## Develop and verify

For local-only room coordination (Node 22+):

```sh
npm ci
npm run dev
```

Enter `http://127.0.0.1:5173` in the native app. Development uses in-memory metadata unless a private Blob token is configured. Two clients on one computer must listen on different UDP ports. No browser client is served. Production refuses to use in-memory metadata.

```sh
npm ci --prefix deployment
npm run cloudflare:build
cargo build --release --locked -p hush-native
npm test
npm run build
cargo fmt --all -- --check
cargo clippy --locked -p hush-native --all-targets -- -D warnings
cargo test --locked -p hush-native
```

For the opt-in native end-to-end test, keep `npm run dev` running and execute:

```sh
cargo run --release --locked -p hush-native -- --self-test
```

It creates two real native identities, creates/joins a room through the TypeScript HTTP service, authenticates both QUIC peers, sends and decodes synthetic Opus audio, sends reliable chat, and leaves. It does not use physical microphones or speakers. Set `HUSH_TEST_SERVER_URL` to test a specific reachable coordination server. The test suite also runs the native client through the local Cloudflare Worker runtime.

See `VERIFICATION.md` for actual results and limits. Low overhead is an implementation goal, not a zero-FPS-loss guarantee. Compare release builds using the same game, streaming settings and squad size; record frame-time percentiles, process CPU/RAM, upload traffic and packet loss. A six-person mesh sends voice to five other players from each computer.

## Source map

- `assets/brand/`: vector/PNG logo, transparent marks and outlined wordmark; see its README for regeneration.
- `native/src/brand.rs`: shared cached UI texture and native window/taskbar icon.
- `native/src/ui.rs`: native interface and global shortcut registration.
- `native/src/audio.rs`: device callbacks, Opus, resampling, jitter buffering and mixing.
- `native/src/transport.rs`: mutual TLS identities, QUIC transport, router mapping and loss accounting.
- `native/src/engine.rs`: session lifecycle, coordination isolation and peer management.
- `native/src/coordination.rs`: HTTPS room API client.
- `native/src/deploy.rs`, `deployment/helper.mjs`: native setup flow and bundled Wrangler deployment.
- `cloudflare/worker.ts`: Cloudflare Worker and SQLite Durable Objects coordination service.
- `server/native-*.ts`, `api/native-*.ts`: Vercel-only room service, private storage and cleanup.
- `packaging/windows/`, `.github/workflows/windows.yml`: native Windows packaging.
