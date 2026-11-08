# Hush native design system

The primary app is native Rust/egui, with no browser-rendered interface. It preserves the established quiet forest/cream identity and standard room navigation.

## Tokens

Main background #161c19; sidebar #1b231e; raised surface #27332a; divider #3f4f42; input surface #202c24; main text #e8ece5; secondary #a3b49b; leaf accent #c3e6a3; accent text #25331f; error #f1a9a0.

Embedded static Manrope: body at weight 450, headings at weight 650, derived from the SIL-OFL Google Fonts source in `native/assets/`. Native font families are assigned explicitly so variable-font default axes cannot render the app too thin. Default egui fonts remain as fallback.

The Hush mark joins an H with a speech tail. The leaf tile and dark forest mark are shared across the native interface, window/taskbar icon, executable, installer and favicon. Vector masters and outlined Manrope wordmark are in `assets/brand/`. The native UI uploads its embedded RGBA artwork once per egui context and reuses the texture; there is no per-frame decoding or logo animation.

Controls use 8–9px rounding, panels 12–14px. Icon strokes share a single 20-unit geometry and 1.6-unit stroke system. The application provides native accessibility semantics, focus styling and textual descriptions for icon buttons.

## Composition and behavior

Native sidebar, room or settings work area, optional peer chat, and an always-present voice dock. The sidebar disappears in compact mode; header buttons retain access to room, connection and settings. Chat is hidden when space is insufficient. Content scrolls inside the application; voice controls remain fixed.

Participant tiles show only actual room membership and measured state. Empty seats are explicit invitation actions. Failures describe recovery. No invented FPS, CPU, latency or peer data.

The renderer is event driven. No decorative animation or continuous frame loop is used. Gaming mode is on by default and disables live mic-meter repaints. The optional microphone meter updates at four frames per second only during testing or when enabled in a focused call. Periodic transport statistics refresh separately.

Native source of truth: `native/src/ui.rs`. The Vercel root page only explains how to configure the desktop client; it is not the client.

## Windows form and control styling

Hush explicitly selects the dark egui theme so Windows light-mode defaults cannot replace navigation labels, fields or modal surfaces. Manrope text, dropdowns, checkboxes and disabled states share the same palette.

Settings are grouped into rounded cards for identity/server, audio, talk controls and direct connection. Single-line fields use a 42px minimum height, 12px horizontal padding, dark green fill, rounded borders and a leaf-colored focus state. Device dropdowns fill their column and truncate long device names. The lobby, invitation dialog and chat composer reuse the same field builder.

Labeled icon buttons reserve separate icon and text regions, with measured text width, accessible labels, visible keyboard focus and clipped overflow. Text is never positioned using leading spaces.

## Terminal server setup

Settings contains the coordination-address field and a link to terminal setup instructions. Cloudflare sign-in, account selection and deployment run only in the host's terminal. The client contains no deployment controls, Node runtime or Wrangler payload. Room-size selection and GitHub app updates remain native controls.

Windows tray lifetime is native and event-driven. Worker repaint requests pass through a shared visibility gate; hidden/minimized windows return before egui panels and timers. Native menu actions and call-end transitions may wake one lifecycle frame. Retain the window/context for fast restoration. Shortcut recording suspends the active global registration, and saving a changed binding registers it before releasing the old one. Quit/update bypass close-to-tray.

The pinned eframe source in `vendor/eframe` patches Windows hidden/minimized repaint scheduling to avoid a full-core idle spin. See its HUSH-PATCH.md for the reproduced failure and upstream provenance. This changes scheduling rather than adding a polling workaround.
