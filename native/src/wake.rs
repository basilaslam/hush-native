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
}
impl Wake {
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            ctx,
            visible: Arc::new(AtomicBool::new(true)),
        }
    }
    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Release);
    }
    pub fn request_repaint(&self) {
        if self.visible.load(Ordering::Acquire) {
            self.force();
        }
    }
    // Only explicit desktop actions and engine lifecycle transitions bypass the gate.
    pub fn force(&self) {
        self.ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

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
