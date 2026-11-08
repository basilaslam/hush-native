//! Background workers can update state without waking the renderer.
use eframe::egui;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct Wake {
    ctx: egui::Context,
    visible: Arc<AtomicBool>,
    #[cfg(windows)]
    window: Arc<std::sync::atomic::AtomicUsize>,
}
impl Wake {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            ctx,
            visible: Arc::new(AtomicBool::new(true)),
            #[cfg(windows)]
            window: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    #[cfg(windows)]
    pub fn attach_window(&self, window: usize) {
        self.window.store(window, Ordering::Release);
    }
    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Release);
    }
    pub fn request_repaint(&self) {
        if self.visible.load(Ordering::Acquire) {
            self.ctx.request_repaint();
        }
    }
    // Only explicit desktop actions and engine lifecycle transitions bypass the gate.
    pub fn force(&self) {
        self.ctx.request_repaint();
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_PAINT};
            let window = self.window.load(Ordering::Acquire);
            if window != 0 {
                // Winit's ordinary RedrawWindow request may be ignored for hidden windows.
                // Post one native paint event for explicit tray/lifecycle actions instead.
                // The handle belongs to eframe and remains alive while the app is running.
                unsafe {
                    PostMessageW(window as _, WM_PAINT, 0, 0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[cfg(windows)]
    #[test]
    fn explicit_actions_wake_a_real_hidden_windows_window() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        unsafe {
            let window = CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                100,
                100,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );
            assert!(!window.is_null());
            let wake = Wake::new(egui::Context::default());
            wake.attach_window(window as usize);
            wake.set_visible(false);
            let mut message = std::mem::zeroed();
            while PeekMessageW(&mut message, window, WM_PAINT, WM_PAINT, PM_REMOVE) != 0 {}
            wake.request_repaint();
            assert_eq!(
                PeekMessageW(&mut message, window, WM_PAINT, WM_PAINT, PM_REMOVE),
                0
            );
            wake.force();
            assert_ne!(
                PeekMessageW(&mut message, window, WM_PAINT, WM_PAINT, PM_REMOVE),
                0
            );
            assert_eq!(message.message, WM_PAINT);
            assert_eq!(IsWindowVisible(window), 0);
            DestroyWindow(window);
        }
    }

    #[test]
    fn hidden_worker_events_do_not_wake_the_event_loop() {
        let ctx = egui::Context::default();
        let count = Arc::new(AtomicUsize::new(0));
        let output = count.clone();
        ctx.set_request_repaint_callback(move |_| {
            output.fetch_add(1, Ordering::Relaxed);
        });
        let wake = Wake::new(ctx.clone());
        for _ in 0..3 {
            let _ = ctx.run(Default::default(), |_| {});
        }
        wake.set_visible(false);
        let prior = count.load(Ordering::Relaxed);
        let worker = wake.clone();
        std::thread::spawn(move || {
            for _ in 0..10_000 {
                worker.request_repaint();
            }
        })
        .join()
        .unwrap();
        assert_eq!(count.load(Ordering::Relaxed), prior);
        wake.set_visible(true);
        wake.request_repaint();
        assert!(count.load(Ordering::Relaxed) > prior);
    }
}
