Hush v0.4.1 adds configurable room capacity and GitHub updates to the native Windows app.

- Choose **2–32 people** in the lobby before creating a room. Six remains the default. Capacity is enforced by the room server; larger rooms use more upload and CPU.
- **Voice & settings → App updates** checks GitHub at startup and every six hours. Preview updates are included by default; both preferences can be changed and saved.
- Choose **Download update**, then **Install update & restart**. The app verifies GitHub's SHA-256 digest and release checksums, waits for Hush to close, updates the same folder and restarts. These controls are disabled during calls, microphone testing and server setup.
- Matching version tags now trigger tested GitHub release builds automatically.

**Upgrade once manually from v0.3.0** to get the updater. To use rooms above six, update every participant and run **Deploy server** again on the host (or redeploy Vercel). Existing room servers do not update merely by installing the app. Room size is fixed at creation; create a new room to change it. Unlimited rooms are not supported by this direct mesh architecture.

Download **Hush-0.4.1-Windows-Setup.exe**, or extract the complete **Hush-0.4.1-Windows-Portable.zip** and run `Hush/hush-native.exe`. Keep its `deployment` folder beside the executable. Source and SHA256SUMS.txt are included.

This is an unsigned preview with direct encrypted QUIC voice. CGNAT/restricted networks can still block calls; there is no ICE/TURN fallback. Physical multi-PC voice and gaming FPS are not benchmarked by CI.
