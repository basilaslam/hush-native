# Hush native design system

The primary app is native Rust/egui, with no browser-rendered interface. It preserves the established quiet forest/cream identity and standard room navigation.

## Tokens

Main background #161c19; sidebar #1b231e; raised surface #27332a; divider #313f33; main text #e8ece5; secondary #a3b49b; leaf accent #c3e6a3; accent text #25331f; error #f1a9a0.

Embedded static Manrope: body at weight 450, headings at weight 650, derived from the SIL-OFL Google Fonts source in `native/assets/`. Native font families are assigned explicitly so variable-font default axes cannot render the app too thin. Default egui fonts remain as fallback.

The Hush mark joins an H with a speech tail. The leaf tile and dark forest mark are shared across the native interface, window/taskbar icon, executable, installer and favicon. Vector masters and outlined Manrope wordmark are in `assets/brand/`. The native UI uploads its embedded RGBA artwork once per egui context and reuses the texture; there is no per-frame decoding or logo animation.

Controls use 8–9px rounding, panels 12–14px. Icon strokes share a single 20-unit geometry and 1.6-unit stroke system. The application provides native accessibility semantics, focus styling and textual descriptions for icon buttons.

## Composition and behavior

Native sidebar, room or settings work area, optional peer chat, and an always-present voice dock. The sidebar disappears in compact mode; header buttons retain access to room, connection and settings. Chat is hidden when space is insufficient. Content scrolls inside the application; voice controls remain fixed.

Participant tiles show only actual room membership and measured state. Empty seats are explicit invitation actions. Failures describe recovery. No invented FPS, CPU, latency or peer data.

The renderer is event driven. No decorative animation or continuous frame loop is used. Gaming mode is on by default and disables live mic-meter repaints. The optional microphone meter updates at four frames per second only during testing or when enabled in a focused call. Periodic transport statistics refresh separately.

Native source of truth: `native/src/ui.rs`. The Vercel root page only explains how to configure the desktop client; it is not the client.
