# Hush terminal server setup

Only the host runs these tools. The Rust desktop app does not contain Node, Wrangler or deployment code.

On Windows, download `setup-server.ps1` from the matching GitHub release, then run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\setup-server.ps1
```

On macOS/Linux, download `setup-server.sh`, then run:

```sh
bash setup-server.sh
```

The launchers reuse Node 22+ with npm when available, or install a user-local Node 24 runtime verified against nodejs.org's SHA-256 manifest. They download the matching `Hush-Server-Setup.zip` from GitHub and verify its checksum before running it. Neither Git, Rust, global npm packages nor administrator access is required. Unix needs curl/tar and unzip (macOS can use its built-in tar). Linux sign-in also requires an unlocked libsecret keyring and `secret-tool`; Windows uses Credential Manager and macOS uses Keychain.

The CLI installs pinned dependencies locally, opens official Cloudflare browser sign-in if needed, selects an account, deploys a Worker with SQLite Durable Objects, verifies its deployment identity and prints the server HTTPS address. Paste that address in Hush Settings, save it, and share it with your friends along with a room invitation. A Cloudflare account and its usage limits apply. A new account may need Workers onboarding / a workers.dev subdomain first. No paid plan is enabled automatically.

Rerunning setup reuses the worker recorded in `servers.json`. Windows uses the same host state folder as the old in-app deployer, preserving its worker mapping and encrypted `hush-native` OAuth profile. Existing Cloudflare resources are retained on cancellation or failure. Use `--worker existing-worker-name` on Unix, or `-WorkerName existing-worker-name` with PowerShell, to explicitly update a different existing worker in your chosen account.

The launcher tools live outside the app, under `%LOCALAPPDATA%\Hush\server-tools` on Windows or `${XDG_DATA_HOME:-~/.local/share}/hush/server-tools` on Unix. Host state is `%LOCALAPPDATA%\Hush\Hush\data\cloudflare` on Windows and `${XDG_DATA_HOME:-~/.local/share}/hush/cloudflare` on Unix. `server-address.txt` contains the verified URL. Credentials stay in Wrangler's secure local profile, not GitHub or the app.

For a validation without sign-in or cloud writes, use `-DryRun` with PowerShell or `--dry-run` with Bash. This still installs local host dependencies. Advanced users with the extracted setup ZIP and Node/npm can run `node setup.mjs` directly, with `--help` for options.

GitHub hosts scripts, source and app releases. Cloudflare runs the dynamic room coordinator. Voice/chat remain direct QUIC; this setup does not add an ICE/TURN relay or resolve CGNAT.
