Hush v0.6.0 adds a Windows tray and configurable push-to-talk shortcuts.

- Close the window to keep voice connected in the tray. Double-click the icon or select Show Hush to reopen it. Right-click for microphone mute and Quit Hush.
- Hidden/minimized windows skip UI construction and periodic UI timers. Audio, network statistics, chat and push-to-talk state changes update in the background without requesting interface repaints. The native window and GPU allocations remain for fast restoration; this is not a zero-resource or zero-FPS-loss claim.
- In Voice & settings → Your way to talk, record a keyboard shortcut or type one (for example Ctrl+Shift+V), then save. F8 remains the default for existing installations. Modifiers are supported. Changing to an occupied shortcut during a call keeps the previous binding working.
- Push-to-talk stays active in the tray. Pointer-based hold-to-talk is released when hiding/minimizing, so the microphone cannot latch open. Mute and deafen continue to override transmission.
- Quit and Install update & restart fully exit the process. If the tray fails to initialize, closing exits normally so Hush cannot become inaccessible.
- Native-only small downloads and separate terminal server setup from v0.5.3 are retained. No Node, Wrangler, browser or webview runtime is bundled in the desktop client.

**Download:** Hush-0.6.0-Windows-Setup.exe or Hush-0.6.0-Windows-Portable.zip. Existing v0.4.1+ clients can install from App updates. Older clients require one manual upgrade. Server redeployment is not needed for the tray/shortcut changes.

Host scripts and Hush-Server-Setup.zip are separate downloads. GitHub hosts releases and updates; Cloudflare hosts room coordination. Only the host runs server setup.

This remains an unsigned Windows preview. Automated checks cover native tray menu messages, visibility/exit behavior, repaint suppression, shortcut conflicts, audio/chat, updates and packaging. Physical microphone/headphone calls, shortcuts inside elevated games, cross-router connectivity and gaming FPS still require testing on your gaming PCs.
