Hush v0.5.3 removes the server deployment payload from the native Windows app.

- Desktop downloads contain the native Rust client, documentation and font license. Node, Wrangler and the in-app Deploy server flow are removed.
- The separate **setup-server.ps1** (Windows) and **setup-server.sh** (macOS/Linux) launchers prepare a private host runtime when needed, verify the server bundle, sign in through official Cloudflare Wrangler, deploy and verify the room server, then print its address.
- Only the host runs setup. Friends need just the native client and shared server address. GitHub distributes the files and updates; Cloudflare hosts room coordination.
- Updating from v0.3/v0.4 removes the old bundled deployment folder and preserves app preferences. Existing cloud servers and separately stored sign-in profiles are retained.
- Manual 2–32-person room sizes and verified GitHub app updates remain available.

**App:** download Hush-0.5.3-Windows-Setup.exe or Hush-0.5.3-Windows-Portable.zip. Existing v0.4.1 clients can install this version from App updates.

**Host setup:** download setup-server.ps1 or setup-server.sh from this release and run it in a terminal. It fetches Hush-Server-Setup.zip and checks SHA256SUMS.txt before installing host dependencies. Node 22+ with npm is reused when available, otherwise a checksum-verified private Node 24 runtime is installed. Browser approval is needed for Cloudflare sign-in. Linux additionally needs an unlocked libsecret keyring/secret-tool.

Source and checksums are included. This remains an unsigned Windows preview. Voice is direct encrypted QUIC, with no ICE/TURN fallback; physical multi-PC calls and game FPS remain outside automated CI checks.
