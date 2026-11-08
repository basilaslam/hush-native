//! Native tray lifetime and window visibility. No polling thread or repaint timer.
use crate::{
    engine::{Command, Controller},
    wake::Wake,
};
use eframe::egui::{self, ViewportCommand};
use std::sync::{atomic::Ordering, mpsc};

#[derive(Clone, Copy)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Action {
    Show,
    Mute,
    Quit,
}

pub struct Desktop {
    tray: Option<platform::Tray>,
    events: mpsc::Receiver<Action>,
    wake: Wake,
    hidden: bool,
    exiting: bool,
}
impl Desktop {
    pub fn unavailable(wake: Wake) -> Self {
        let (_, events) = mpsc::channel();
        Self {
            tray: None,
            events,
            wake,
            hidden: false,
            exiting: false,
        }
    }
    pub fn new(wake: Wake) -> anyhow::Result<Self> {
        let (tx, events) = mpsc::channel();
        let tray = platform::Tray::new(tx, wake.clone())?;
        Ok(Self {
            tray,
            events,
            wake,
            hidden: false,
            exiting: false,
        })
    }
    pub fn available(&self) -> bool {
        self.tray.is_some()
    }
    #[cfg(windows)]
    pub fn probe_commands(&self) -> (usize, [u32; 3]) {
        self.tray.as_ref().unwrap().probe_commands()
    }
    pub fn quit(&mut self, ctx: &egui::Context) {
        self.exiting = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
        self.wake.force();
    }
    pub fn update(&mut self, ctx: &egui::Context, engine: &Controller) -> bool {
        while let Ok(action) = self.events.try_recv() {
            match action {
                Action::Show => {
                    self.hidden = false;
                    self.wake.set_visible(true);
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                Action::Mute => {
                    engine.controls.muted.fetch_xor(true, Ordering::Relaxed);
                    engine.send(Command::ControlChanged);
                }
                Action::Quit => self.quit(ctx),
            }
        }
        if let Some(tray) = &self.tray {
            tray.set_muted(engine.controls.muted.load(Ordering::Relaxed));
        }
        if ctx.input(|input| input.viewport().close_requested())
            && !self.exiting
            && self.available()
        {
            self.hidden = true;
            self.wake.set_visible(false);
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        let visible = !self.hidden
            && !self.exiting
            && !ctx.input(|input| input.viewport().minimized.unwrap_or(false));
        self.wake.set_visible(visible);
        // A pointer release can occur outside a hidden/minimized window.
        if !visible
            && engine
                .controls
                .pointer_pressed
                .swap(false, Ordering::Relaxed)
        {
            engine.send(Command::ControlChanged);
        }
        visible
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use tray_icon::{
        menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
        MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub struct Tray {
        _icon: TrayIcon,
        menu: Menu,
        mute: CheckMenuItem,
    }
    impl Tray {
        pub fn new(tx: mpsc::Sender<Action>, wake: Wake) -> anyhow::Result<Option<Self>> {
            let menu = Menu::new();
            let show = MenuItem::new("Show Hush", true, None);
            let mute = CheckMenuItem::new("Mute microphone", true, false, None);
            let quit = MenuItem::new("Quit Hush", true, None);
            menu.append_items(&[&show, &mute, &PredefinedMenuItem::separator(), &quit])?;
            let pixels = crate::brand::window_icon();
            let icon = TrayIconBuilder::new()
                .with_id("hush")
                .with_guid(0x9d746937_25b6_4a91_9372_94c0b11330f5)
                .with_tooltip("Hush · Voice stays connected in the tray")
                .with_icon(tray_icon::Icon::from_rgba(
                    pixels.rgba,
                    pixels.width,
                    pixels.height,
                )?)
                .with_menu(Box::new(menu.clone()))
                .with_menu_on_left_click(false)
                .build()?;
            let show_id = show.id().clone();
            let mute_id = mute.id().clone();
            let quit_id = quit.id().clone();
            let menu_tx = tx.clone();
            let menu_wake = wake.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let action = if event.id == show_id {
                    Action::Show
                } else if event.id == mute_id {
                    Action::Mute
                } else if event.id == quit_id {
                    Action::Quit
                } else {
                    return;
                };
                let _ = menu_tx.send(action);
                menu_wake.force();
            }));
            TrayIconEvent::set_event_handler(Some(move |event| {
                if matches!(
                    event,
                    TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    }
                ) {
                    let _ = tx.send(Action::Show);
                    wake.force();
                }
            }));
            Ok(Some(Self {
                _icon: icon,
                mute,
                menu,
            }))
        }
        pub fn set_muted(&self, value: bool) {
            if self.mute.is_checked() != value {
                self.mute.set_checked(value);
            }
        }
        pub fn probe_commands(&self) -> (usize, [u32; 3]) {
            use tray_icon::menu::ContextMenu;
            use windows_sys::Win32::UI::WindowsAndMessaging::GetMenuItemID;
            let menu = self.menu.hpopupmenu() as _;
            (self._icon.window_handle() as usize, unsafe {
                [
                    GetMenuItemID(menu, 0),
                    GetMenuItemID(menu, 1),
                    GetMenuItemID(menu, 3),
                ]
            })
        }
    }
    impl Drop for Tray {
        fn drop(&mut self) {
            MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
            TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tray_icon::menu::ContextMenu;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetMenuItemID, SendMessageW, WM_COMMAND,
        };

        fn frame(
            ctx: &egui::Context,
            desktop: &mut Desktop,
            engine: &Controller,
            close: bool,
        ) -> (bool, egui::FullOutput) {
            let mut input = egui::RawInput::default();
            if close {
                input
                    .viewports
                    .get_mut(&egui::ViewportId::ROOT)
                    .unwrap()
                    .events
                    .push(egui::ViewportEvent::Close);
            }
            let mut foreground = false;
            let output = ctx.run(input, |ctx| {
                foreground = desktop.update(ctx, engine);
                if foreground {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Tray integration check");
                    });
                }
            });
            (foreground, output)
        }
        fn click(desktop: &Desktop, position: i32) {
            let tray = desktop.tray.as_ref().unwrap();
            // Exercise the real native menu subclass and registered event callback.
            unsafe {
                let command = GetMenuItemID(tray.menu.hpopupmenu() as _, position);
                assert_ne!(command, u32::MAX);
                SendMessageW(tray._icon.window_handle(), WM_COMMAND, command as usize, 0);
            }
        }
        #[test]
        fn native_tray_menu_hides_restores_mutes_and_quits() {
            let ctx = egui::Context::default();
            let wake = Wake::new(ctx.clone());
            let engine = Controller::new(wake.clone()).unwrap();
            let mut desktop = Desktop::new(wake).unwrap();
            assert!(desktop.available());
            frame(&ctx, &mut desktop, &engine, false);
            engine.controls.ptt.store(true, Ordering::Relaxed);
            engine
                .controls
                .pointer_pressed
                .store(true, Ordering::Relaxed);
            let (visible, output) = frame(&ctx, &mut desktop, &engine, true);
            assert!(!visible);
            assert!(output.shapes.is_empty());
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&ViewportCommand::CancelClose));
            assert!(commands.contains(&ViewportCommand::Visible(false)));
            assert!(!engine.controls.transmitting()); // Pointer cannot latch when hidden.
            engine.controls.pressed.store(true, Ordering::Relaxed);
            assert!(engine.controls.transmitting()); // Native PTT is independent of rendering.
            click(&desktop, 1);
            assert!(!frame(&ctx, &mut desktop, &engine, false).0);
            assert!(engine.controls.muted.load(Ordering::Relaxed));
            assert!(!engine.controls.transmitting());
            click(&desktop, 0);
            let (visible, output) = frame(&ctx, &mut desktop, &engine, false);
            assert!(visible);
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&ViewportCommand::Visible(true)));
            assert!(commands.contains(&ViewportCommand::Minimized(false)));
            click(&desktop, 3);
            let (visible, output) = frame(&ctx, &mut desktop, &engine, true);
            assert!(!visible);
            let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
            assert!(commands.contains(&ViewportCommand::Close));
            assert!(!commands.contains(&ViewportCommand::CancelClose));
            drop(desktop);
            let mut desktop = Desktop::new(Wake::new(ctx.clone())).unwrap();
            desktop.quit(&ctx);
            let (_, output) = frame(&ctx, &mut desktop, &engine, true);
            assert!(desktop.exiting);
            assert!(!output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&ViewportCommand::CancelClose));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_tray_does_not_hide_an_inaccessible_app() {
        let ctx = egui::Context::default();
        let wake = Wake::new(ctx.clone());
        let engine = Controller::new(wake.clone()).unwrap();
        let mut desktop = Desktop::unavailable(wake);
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| {
            desktop.update(ctx, &engine);
        });
        assert!(!output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .contains(&ViewportCommand::CancelClose));
        assert!(!desktop.hidden);
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    pub struct Tray;
    impl Tray {
        pub fn new(_: mpsc::Sender<Action>, _: Wake) -> anyhow::Result<Option<Self>> {
            Ok(None)
        }
        pub fn set_muted(&self, _: bool) {}
    }
}
