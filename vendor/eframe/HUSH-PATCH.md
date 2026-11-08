# eframe 0.33.3 — Hush Windows tray patch

This is the crates.io eframe 0.33.3 source with one small change in `src/native/run.rs::check_redraw_requests`.

Windows can ignore `RedrawWindow` for hidden/minimized windows. Upstream switches the event loop to `ControlFlow::Poll` before requesting that redraw. With no paint arriving to restore `Wait`, a hidden Hush window consumed one full CPU core despite zero UI frames. A real Windows eframe/Win32 test reproduced 2,000 ms process CPU over 2 seconds.

The patch discards due redraw requests for hidden/minimized Windows windows and returns to `Wait` when no paint or timer is pending. Visible-window scheduling is preserved. Explicit native tray actions use Hush's separate posted HWND wake.

The Windows workflow runs `--desktop-self-test` with test-only Mesa outside shipping packages. It verifies hidden frame counts and process CPU, native Show/Mute/Quit, and restoration. Keep this patch until an upstream version provides equivalent behavior. Source provenance: https://crates.io/crates/eframe/0.33.3 and https://github.com/emilk/egui/tree/0.33.3/crates/eframe .
