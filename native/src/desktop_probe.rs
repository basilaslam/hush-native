//! Runs only with --desktop-self-test. Exercises the real eframe/Win32 event loop in CI.
use crate::{desktop::Desktop, engine::Controller, wake::Wake};
use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::FILETIME,
    System::Threading::{GetCurrentProcess, GetProcessTimes},
    UI::WindowsAndMessaging::{IsWindowVisible, PostMessageW, SendMessageW, WM_CLOSE, WM_COMMAND},
};

type Outcome = Arc<Mutex<Option<Result<String, String>>>>;
struct Probe {
    desktop: Desktop,
    engine: Controller,
    frames: Arc<AtomicUsize>,
}
fn cpu_time() -> u64 {
    unsafe {
        let mut created = std::mem::zeroed::<FILETIME>();
        let mut exited = std::mem::zeroed::<FILETIME>();
        let mut kernel = std::mem::zeroed::<FILETIME>();
        let mut user = std::mem::zeroed::<FILETIME>();
        assert_ne!(
            GetProcessTimes(
                GetCurrentProcess(),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user
            ),
            0
        );
        [kernel, user]
            .iter()
            .map(|value| ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64)
            .sum()
    }
}
impl Probe {
    fn new(cc: &eframe::CreationContext<'_>, outcome: Outcome) -> anyhow::Result<Self> {
        let RawWindowHandle::Win32(handle) = cc.window_handle()?.as_raw() else {
            anyhow::bail!("Expected Windows handle");
        };
        let window = handle.hwnd.get() as usize;
        let wake = Wake::new(cc.egui_ctx.clone());
        wake.attach_window(window);
        let desktop = Desktop::new(wake.clone())?;
        let (tray, commands) = desktop.probe_commands();
        let engine = Controller::new(wake.clone())?;
        let controls = engine.controls.clone();
        let frames = Arc::new(AtomicUsize::new(0));
        let observed = frames.clone();
        std::thread::spawn(move || {
            let sleep = |ms| std::thread::sleep(Duration::from_millis(ms));
            let click = |index: usize| unsafe {
                SendMessageW(tray as _, WM_COMMAND, commands[index] as usize, 0);
            };
            let result = (|| -> anyhow::Result<String> {
                sleep(1500);
                anyhow::ensure!(observed.load(Ordering::Relaxed) > 0, "GUI never rendered");
                unsafe { PostMessageW(window as _, WM_CLOSE, 0, 0); }
                sleep(700);
                anyhow::ensure!(unsafe { IsWindowVisible(window as _) } == 0, "Close did not hide the window");
                let before = observed.load(Ordering::Relaxed);
                let cpu_before = cpu_time();
                for _ in 0..10_000 { wake.request_repaint(); }
                sleep(2000);
                let extra_frames = observed.load(Ordering::Relaxed) - before;
                let cpu_ms = (cpu_time() - cpu_before) / 10_000;
                anyhow::ensure!(extra_frames <= 2, "Hidden renderer kept running: {extra_frames} frames");
                anyhow::ensure!(cpu_ms < 250, "Hidden event loop used {cpu_ms} ms CPU in 2 seconds");
                click(1);
                sleep(400);
                anyhow::ensure!(controls.muted.load(Ordering::Relaxed), "Hidden tray mute was not processed");
                click(0);
                sleep(700);
                anyhow::ensure!(unsafe { IsWindowVisible(window as _) } != 0, "Tray Show did not restore the window");
                anyhow::ensure!(observed.load(Ordering::Relaxed) > before, "Restored renderer did not resume");
                unsafe { PostMessageW(window as _, WM_CLOSE, 0, 0); }
                sleep(400);
                Ok(format!("Real Windows event loop passed: {extra_frames} hidden frames, {cpu_ms} ms process CPU over 2 seconds; native Show/Mute/Quit."))
            })().map_err(|error| format!("{error:#}"));
            *outcome.lock().unwrap() = Some(result);
            click(2);
        });
        Ok(Self {
            desktop,
            engine,
            frames,
        })
    }
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.frames.fetch_add(1, Ordering::Relaxed);
        if !self.desktop.update(ctx, &self.engine) {
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hush native tray check");
        });
        // Also verify that hiding drains a previously scheduled visible UI timer.
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
pub fn run(options: eframe::NativeOptions) -> eframe::Result {
    let outcome = Outcome::default();
    let result = outcome.clone();
    eframe::run_native(
        "Hush desktop check",
        options,
        Box::new(move |cc| Ok(Box::new(Probe::new(cc, result)?))),
    )?;
    let result = outcome
        .lock()
        .unwrap()
        .take()
        .unwrap_or_else(|| Err("Desktop probe closed before finishing".into()));
    match result {
        Ok(detail) => {
            eprintln!("{detail}");
            Ok(())
        }
        Err(detail) => Err(eframe::Error::AppCreation(Box::new(std::io::Error::other(
            detail,
        )))),
    }
}
