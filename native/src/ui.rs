use crate::{
    audio,
    config::Config,
    engine::{Command, Controller},
    model::ViewState,
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Vec2,
};
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
const BG: Color32 = Color32::from_rgb(22, 28, 25);
const SIDE: Color32 = Color32::from_rgb(27, 35, 30);
const PANEL: Color32 = Color32::from_rgb(39, 51, 42);
const LINE: Color32 = Color32::from_rgb(63, 79, 66);
const FIELD: Color32 = Color32::from_rgb(32, 44, 36);
const TEXT: Color32 = Color32::from_rgb(232, 236, 229);
const MUTED: Color32 = Color32::from_rgb(163, 180, 155);
const GREEN: Color32 = Color32::from_rgb(195, 230, 163);
const INK: Color32 = Color32::from_rgb(37, 51, 31);
const RED: Color32 = Color32::from_rgb(241, 169, 160);
#[derive(Clone, Copy)]
enum Icon {
    Mic,
    Headphones,
    Plus,
    Link,
    Settings,
    Signal,
    Chat,
    Power,
    Leaf,
    Arrow,
    Lock,
    Close,
    Monitor,
}
fn icon(painter: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 20.;
    let p = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let stroke = Stroke::new(1.6 * s, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    match kind {
        Icon::Mic => {
            painter.rect_stroke(
                Rect::from_min_max(p(-3., -8.), p(3., 3.)),
                4.,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.line_segment([p(-6., -1.), p(-6., 3.)], stroke);
            painter.line_segment([p(6., -1.), p(6., 3.)], stroke);
            painter.add(egui::Shape::line(
                vec![p(-6., 3.), p(-4., 6.), p(4., 6.), p(6., 3.)],
                stroke,
            ));
            line((0., 6.), (0., 9.));
            line((-4., 9.), (4., 9.));
        }
        Icon::Headphones => {
            painter.add(egui::Shape::line(
                vec![
                    p(-8., 4.),
                    p(-8., -2.),
                    p(-5., -7.),
                    p(0., -9.),
                    p(5., -7.),
                    p(8., -2.),
                    p(8., 4.),
                ],
                stroke,
            ));
            painter.rect_stroke(
                Rect::from_min_max(p(-9., 0.), p(-5., 8.)),
                2.,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(5., 0.), p(9., 8.)),
                2.,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Icon::Plus => {
            line((-7., 0.), (7., 0.));
            line((0., -7.), (0., 7.));
        }
        Icon::Close => {
            line((-5., -5.), (5., 5.));
            line((-5., 5.), (5., -5.));
        }
        Icon::Signal => {
            for i in 0..4 {
                let x = -7. + i as f32 * 4.;
                line((x, 7.), (x, 5. - i as f32 * 4.));
            }
        }
        Icon::Arrow => {
            line((-7., 0.), (7., 0.));
            line((2., -5.), (7., 0.));
            line((2., 5.), (7., 0.));
        }
        Icon::Link => {
            painter.rect_stroke(
                Rect::from_center_size(p(-4., 2.), Vec2::new(10. * s, 7. * s)),
                3.,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_center_size(p(4., -2.), Vec2::new(10. * s, 7. * s)),
                3.,
                stroke,
                egui::StrokeKind::Middle,
            );
        }
        Icon::Chat => {
            painter.rect_stroke(
                Rect::from_min_max(p(-8., -7.), p(8., 5.)),
                3.,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((-4., 5.), (-7., 9.));
            line((-7., 9.), (-7., 5.));
        }
        Icon::Settings => {
            for (y, x) in [(-6., -3.), (0., 4.), (6., -1.)] {
                line((-8., y), (8., y));
                painter.circle_filled(p(x, y), 2.5 * s, color);
            }
        }
        Icon::Power => {
            line((0., -9.), (0., 1.));
            painter.add(egui::Shape::line(
                vec![
                    p(-4., -5.),
                    p(-8., -1.),
                    p(-7., 5.),
                    p(-3., 8.),
                    p(3., 8.),
                    p(7., 5.),
                    p(8., -1.),
                    p(4., -5.),
                ],
                stroke,
            ));
        }
        Icon::Leaf => {
            painter.add(egui::Shape::line(
                vec![
                    p(-6., 6.),
                    p(-8., 0.),
                    p(-5., -5.),
                    p(7., -8.),
                    p(7., 2.),
                    p(2., 7.),
                    p(-6., 6.),
                ],
                stroke,
            ));
            line((-8., 9.), (4., -3.));
        }
        Icon::Lock => {
            painter.rect_stroke(
                Rect::from_min_max(p(-6., -1.), p(6., 8.)),
                2.,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.add(egui::Shape::line(
                vec![p(-4., -1.), p(-4., -6.), p(0., -8.), p(4., -6.), p(4., -1.)],
                stroke,
            ));
        }
        Icon::Monitor => {
            painter.rect_stroke(
                Rect::from_min_max(p(-8., -7.), p(8., 4.)),
                2.,
                stroke,
                egui::StrokeKind::Middle,
            );
            line((0., 4.), (0., 8.));
            line((-4., 8.), (4., 8.));
        }
    }
}
fn logo(painter: &egui::Painter, rect: Rect) {
    let texture = crate::brand::texture(painter.ctx());
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
        Color32::WHITE,
    );
}
fn icon_text_button(
    ui: &mut egui::Ui,
    label: &str,
    kind: Icon,
    fill: Color32,
    foreground: Color32,
    full_width: bool,
) -> egui::Response {
    let font = FontId::proportional(13.);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, foreground);
    let width = if full_width {
        ui.available_width()
    } else {
        galley.size().x + 58.
    };
    let response = ui.add(
        egui::Button::new("")
            .fill(fill)
            .stroke(Stroke::new(
                1.0_f32,
                if fill == GREEN { GREEN } else { LINE },
            ))
            .min_size(Vec2::new(width.min(ui.available_width()), 44.))
            .corner_radius(9.),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            9.,
            Stroke::new(1.0_f32, GREEN),
            egui::StrokeKind::Inside,
        );
    }
    icon(
        ui.painter(),
        Rect::from_center_size(
            Pos2::new(response.rect.left() + 21., response.rect.center().y),
            Vec2::splat(17.),
        ),
        kind,
        foreground,
    );
    let text_rect = Rect::from_min_max(
        Pos2::new(response.rect.left() + 40., response.rect.top()),
        Pos2::new(response.rect.right() - 12., response.rect.bottom()),
    );
    ui.painter()
        .with_clip_rect(text_rect.intersect(ui.clip_rect()))
        .galley(
            Pos2::new(
                text_rect.left(),
                response.rect.center().y - galley.size().y / 2.,
            ),
            galley,
            foreground,
        );
    response
}
fn action(ui: &mut egui::Ui, label: &str, kind: Icon, primary: bool) -> egui::Response {
    icon_text_button(
        ui,
        label,
        kind,
        if primary { GREEN } else { PANEL },
        if primary { INK } else { TEXT },
        false,
    )
}
fn text_field<'a>(value: &'a mut String, hint: &str, limit: usize) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(value)
        .hint_text(RichText::new(hint).color(MUTED))
        .char_limit(limit)
        .text_color(TEXT)
        .background_color(FIELD)
        .margin(Margin::symmetric(12, 11))
        .min_size(Vec2::new(0., 42.))
        .desired_width(f32::INFINITY)
}
fn form_card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(SIDE)
        .stroke(Stroke::new(1.0_f32, LINE))
        .corner_radius(14.)
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui);
        });
}
fn section_heading(ui: &mut egui::Ui, name: &str, description: &str, kind: Icon) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(20.), Sense::hover());
        icon(ui.painter(), rect, kind, GREEN);
        title(ui, name, 17.);
    });
    ui.add_space(3.);
    small(ui, description);
    ui.add_space(15.);
}
fn icon_button(ui: &mut egui::Ui, label: &str, kind: Icon, selected: bool) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .fill(if selected {
                Color32::from_rgb(72, 49, 43)
            } else {
                PANEL
            })
            .min_size(Vec2::splat(39.))
            .corner_radius(9.),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    icon(
        ui.painter(),
        response.rect.shrink(10.),
        kind,
        if selected { RED } else { TEXT },
    );
    response.on_hover_text(label)
}
fn small(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).size(13.).color(MUTED));
}
fn label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(13.).color(TEXT));
    ui.add_space(5.);
}
fn title(ui: &mut egui::Ui, text: &str, size: f32) {
    ui.label(
        RichText::new(text)
            .font(FontId::new(
                size,
                egui::FontFamily::Name("HushDisplay".into()),
            ))
            .color(TEXT),
    );
}
#[derive(PartialEq)]
enum Page {
    Room,
    Connection,
    Settings,
    Help,
}
#[derive(PartialEq)]
enum Modal {
    Join,
    Invite,
}
pub struct HushApp {
    engine: Controller,
    deployment: crate::deploy::Deployment,
    updater: crate::updater::Updater,
    last_update_check: std::time::Instant,
    config: Config,
    draft: Config,
    page: Page,
    modal: Option<Modal>,
    invitation: String,
    chat: String,
    chat_open: bool,
    compact: bool,
    inputs: Vec<String>,
    outputs: Vec<String>,
    hotkey: Option<GlobalHotKeyManager>,
    hotkey_registered: bool,
    copied: bool,
}
impl HushApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        let ctx = &cc.egui_ctx;
        let _ = crate::brand::texture(ctx);
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Manrope".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/Manrope-Body.ttf"
            ))),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Manrope".into());
        fonts.font_data.insert(
            "HushDisplay".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/Manrope-Display.ttf"
            ))),
        );
        fonts.families.insert(
            egui::FontFamily::Name("HushDisplay".into()),
            vec!["HushDisplay".into(), "Manrope".into()],
        );
        ctx.set_fonts(fonts);
        // Hush has one authored palette, independent of the Windows theme.
        ctx.set_theme(egui::Theme::Dark);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = SIDE;
        visuals.override_text_color = Some(TEXT);
        visuals.extreme_bg_color = Color32::from_rgb(18, 26, 21);
        visuals.faint_bg_color = PANEL;
        visuals.selection.bg_fill = Color32::from_rgb(65, 89, 48);
        visuals.selection.stroke = Stroke::new(1.0_f32, GREEN);
        visuals.widgets.inactive.bg_fill = PANEL;
        visuals.widgets.inactive.weak_bg_fill = PANEL;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 65, 50);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 65, 50);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, GREEN);
        visuals.widgets.active.bg_fill = Color32::from_rgb(61, 79, 54);
        visuals.weak_text_color = Some(MUTED);
        visuals.text_edit_bg_color = Some(FIELD);
        visuals.hyperlink_color = GREEN;
        visuals.window_stroke = Stroke::new(1.0_f32, LINE);
        visuals.window_corner_radius = CornerRadius::same(14);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LINE);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, GREEN);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
        visuals.widgets.active.corner_radius = CornerRadius::same(8);
        visuals.widgets.open.corner_radius = CornerRadius::same(8);
        visuals.widgets.open.bg_fill = FIELD;
        visuals.widgets.open.weak_bg_fill = FIELD;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
        ctx.set_visuals_of(egui::Theme::Dark, visuals);
        ctx.style_mut(|style| {
            style.spacing.item_spacing = Vec2::new(10., 9.);
            style.spacing.button_padding = Vec2::new(15., 9.);
            style.spacing.interact_size = Vec2::new(40., 42.);
            style.spacing.combo_height = 240.;
            style.spacing.slider_width = 115.;
            style.animation_time = 0.;
            style
                .text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(14.));
            style
                .text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(13.));
        });
        let config = Config::load();
        let engine = Controller::new(ctx.clone())?;
        if let Some(error) = crate::updater::previous_install_error() {
            engine.state.lock().unwrap().error = error;
        }
        engine.configure(&config);
        let controls = engine.controls.clone();
        let repaint = ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            controls
                .pressed
                .store(event.state == HotKeyState::Pressed, Ordering::Relaxed);
            repaint.request_repaint();
        }));
        let hotkey = GlobalHotKeyManager::new().ok();
        let updater = crate::updater::Updater::default();
        if config.check_updates {
            updater.check(ctx.clone(), config.preview_updates);
        }
        Ok(Self {
            engine,
            deployment: crate::deploy::Deployment::default(),
            updater,
            last_update_check: std::time::Instant::now(),
            draft: config.clone(),
            config,
            page: Page::Room,
            modal: None,
            invitation: String::new(),
            chat: String::new(),
            chat_open: true,
            compact: false,
            inputs: vec![],
            outputs: vec![],
            hotkey,
            hotkey_registered: false,
            copied: false,
        })
    }
    fn save(&mut self) {
        self.engine.configure(&self.draft);
        if let Err(error) = self.draft.save() {
            self.engine.state.lock().unwrap().error = error.to_string();
            return;
        }
        self.config = self.draft.clone();
        self.engine.state.lock().unwrap().notice = "Settings saved on this device.".into();
    }
    fn begin(&mut self, invitation: Option<String>) {
        if self.deployment.snapshot().running {
            self.engine.state.lock().unwrap().error =
                "Finish or cancel server setup before joining a room.".into();
            return;
        }
        if let Err(error) = self.config.validate() {
            self.engine.state.lock().unwrap().error = error.to_string();
            if self.config.server.trim().is_empty() {
                self.page = Page::Settings;
                self.draft = self.config.clone();
            }
            return;
        }
        self.engine.configure(&self.config);
        self.engine.send(Command::Enter {
            config: self.config.clone(),
            invitation,
        });
        self.modal = None;
        self.page = Page::Room;
    }
    fn sync_hotkey(&mut self, active: bool) {
        let wanted = active && self.config.push_to_talk;
        if wanted == self.hotkey_registered {
            return;
        }
        if let Some(manager) = &self.hotkey {
            let hotkey = HotKey::new(None, Code::F8);
            if wanted {
                match manager.register(hotkey) {
                    Ok(()) => self.hotkey_registered = true,
                    Err(error) => {
                        self.engine.state.lock().unwrap().notice=format!("Global F8 could not register: {error}. Use the in-app hold-to-talk button.");
                    }
                }
            } else {
                let _ = manager.unregister(hotkey);
                self.hotkey_registered = false;
                self.engine.ptt(false);
            }
        }
    }
    fn settings(&mut self, ui: &mut egui::Ui, state: &ViewState) {
        ui.set_max_width(ui.available_width().min(820.));
        title(ui, "Sound like yourself.", 32.);
        small(ui, "A few thoughtful settings. A better session.");
        ui.add_space(24.);
        let deployment = self.deployment.snapshot();
        let locked = state.session.is_some() || state.busy || state.testing || deployment.running;
        form_card(ui, |ui| {
            section_heading(
                ui,
                "Host your squad",
                "Your own room server, set up in a few clicks.",
                Icon::Signal,
            );
            small(ui, "Sign in to Cloudflare in your browser. Hush deploys the server and fills in its address for you.");
            ui.add_space(12.);
            if deployment.running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(&deployment.message).color(TEXT));
                });
                if !deployment.accounts.is_empty() {
                    ui.add_space(10.);
                    label(ui, "Cloudflare account");
                    let mut selected = deployment.selected;
                    egui::ComboBox::from_id_salt("cloudflare_account")
                        .selected_text(
                            deployment
                                .accounts
                                .get(selected)
                                .map_or("Choose an account", |account| account.name.as_str()),
                        )
                        .width(ui.available_width())
                        .truncate()
                        .show_ui(ui, |ui| {
                            for (index, account) in deployment.accounts.iter().enumerate() {
                                ui.selectable_value(&mut selected, index, &account.name);
                            }
                        });
                    self.deployment.select(selected);
                    if action(ui, "Use this account", Icon::Arrow, true).clicked() {
                        self.deployment.use_account();
                    }
                }
                ui.add_space(10.);
                if action(ui, "Cancel deployment", Icon::Close, false).clicked() {
                    self.deployment.cancel();
                }
            } else {
                ui.add_enabled_ui(
                    state.session.is_none()
                        && !state.busy
                        && !state.testing
                        && !self.deployment.snapshot().running,
                    |ui| {
                        if action(ui, "Deploy server", Icon::Plus, true).clicked() {
                            self.deployment.start(ui.ctx().clone());
                        }
                    },
                );
                if !deployment.message.is_empty() {
                    ui.add_space(10.);
                    ui.label(
                        RichText::new(&deployment.message).color(if deployment.error {
                            RED
                        } else {
                            GREEN
                        }),
                    );
                }
            }
            ui.add_space(10.);
            small(ui, "Uses your Cloudflare account and its usage limits. Voice and chat stay peer to peer.");
        });
        ui.add_space(16.);
        form_card(ui, |ui| {
            section_heading(
                ui,
                "Your home base",
                "Choose your name and the server your squad uses.",
                Icon::Leaf,
            );
            ui.add_enabled_ui(!locked, |ui| {
                label(ui, "Coordination server");
                ui.add(text_field(
                    &mut self.draft.server,
                    "https://your-server.workers.dev",
                    2048,
                ));
                small(
                    ui,
                    "Deploy above, or use an existing Cloudflare or Vercel server address.",
                );
                ui.add_space(12.);
                label(ui, "Your display name");
                ui.add(text_field(
                    &mut self.draft.name,
                    "What should your friends call you?",
                    24,
                ));
            });
        });
        ui.add_space(16.);
        form_card(ui, |ui| {
            section_heading(
                ui,
                "Make yourself heard",
                "Pick your devices and find a comfortable level.",
                Icon::Headphones,
            );
            ui.add_enabled_ui(!locked, |ui| {
                ui.columns(2, |columns| {
                    label(&mut columns[0], "Microphone");
                    egui::ComboBox::from_id_salt("input")
                        .selected_text(if self.draft.input.is_empty() {
                            "System default"
                        } else {
                            &self.draft.input
                        })
                        .width(columns[0].available_width())
                        .truncate()
                        .show_ui(&mut columns[0], |ui| {
                            ui.selectable_value(
                                &mut self.draft.input,
                                String::new(),
                                "System default",
                            );
                            for name in &self.inputs {
                                ui.selectable_value(&mut self.draft.input, name.clone(), name);
                            }
                        });
                    label(&mut columns[1], "Playback");
                    egui::ComboBox::from_id_salt("output")
                        .selected_text(if self.draft.output.is_empty() {
                            "System default"
                        } else {
                            &self.draft.output
                        })
                        .width(columns[1].available_width())
                        .truncate()
                        .show_ui(&mut columns[1], |ui| {
                            ui.selectable_value(
                                &mut self.draft.output,
                                String::new(),
                                "System default",
                            );
                            for name in &self.outputs {
                                ui.selectable_value(&mut self.draft.output, name.clone(), name);
                            }
                        });
                });
                ui.add_space(8.);
                if action(ui, "Refresh devices", Icon::Arrow, false).clicked() {
                    self.inputs = audio::devices(true);
                    self.outputs = audio::devices(false);
                }
                ui.add_space(12.);
                label(ui, "Voice quality per peer");
                egui::ComboBox::from_id_salt("bitrate")
                    .selected_text(format!("{} kbps", self.draft.bitrate / 1000))
                    .width(ui.available_width())
                    .truncate()
                    .show_ui(ui, |ui| {
                        for bitrate in [24000, 32000, 64000] {
                            ui.selectable_value(
                                &mut self.draft.bitrate,
                                bitrate,
                                format!("{} kbps", bitrate / 1000),
                            );
                        }
                    });
            });
            ui.add_space(15.);
            ui.columns(2, |columns| {
                label(&mut columns[0], "Microphone gain");
                columns[0]
                    .add(egui::Slider::new(&mut self.draft.input_gain, 0.0..=2.0).suffix("×"));
                label(&mut columns[1], "Playback volume");
                columns[1]
                    .add(egui::Slider::new(&mut self.draft.output_volume, 0.0..=1.5).suffix("×"));
            });
            ui.add_space(14.);
            ui.add_enabled_ui(
                !state.busy && state.session.is_none() && !deployment.running,
                |ui| {
                    if action(
                        ui,
                        if state.testing {
                            "Stop microphone test"
                        } else {
                            "Test microphone"
                        },
                        Icon::Mic,
                        false,
                    )
                    .clicked()
                    {
                        if state.testing {
                            self.engine.send(Command::StopTest);
                        } else {
                            self.engine.configure(&self.draft);
                            self.engine.send(Command::Test(self.draft.clone()));
                        }
                    }
                },
            );
            if state.testing {
                ui.add_space(12.);
                let level = f32::from_bits(self.engine.controls.level.load(Ordering::Relaxed));
                ui.add(
                    egui::ProgressBar::new((level * 4.).min(1.))
                        .fill(GREEN)
                        .text("Microphone test stays on this device"),
                );
            }
            ui.add_space(8.);
            small(
                ui,
                "Use headphones. Echo cancellation and noise suppression are not included.",
            );
        });
        ui.add_space(16.);
        form_card(ui, |ui| {
            section_heading(
                ui,
                "Your way to talk",
                "Simple controls that keep the game in focus.",
                Icon::Mic,
            );
            ui.checkbox(&mut self.draft.push_to_talk, "Push to talk · global F8");
            small(ui, "F8 is active only during calls. The in-app hold-to-talk button is always available.");
            ui.add_space(10.);
            ui.checkbox(&mut self.draft.quiet, "Gaming mode");
            small(
                ui,
                "Keep microphone-meter repaints off during calls for a quieter interface.",
            );
        });
        ui.add_space(16.);
        form_card(ui, |ui| {
            section_heading(
                ui,
                "Direct connection",
                "For LAN calls or a reachable internet connection.",
                Icon::Signal,
            );
            ui.add_enabled_ui(!locked, |ui| {
                ui.columns(2, |columns| {
                    label(&mut columns[0], "Listening UDP port");
                    columns[0].add_sized(Vec2::new(columns[0].available_width(), 42.),
                        egui::DragValue::new(&mut self.draft.port).range(1024..=65535));
                    label(&mut columns[1], "Public address (optional)");
                    columns[1].add(text_field(&mut self.draft.public_address, "203.0.113.1:41642", 64));
                });
                ui.add_space(12.);
                ui.checkbox(&mut self.draft.auto_map, "Automatic router mapping (UPnP)")
                    .on_hover_text("Opens a temporary UDP mapping for the active room, renews it during long sessions, and removes it when leaving.");
                small(ui, "No relay is available. Some carrier or restricted networks may block direct calls.");
            });
        });
        if locked {
            ui.add_space(12.);
            small(ui, "Finish server setup, leave the room and stop the microphone test before changing connection settings.");
        }
        ui.add_space(16.);
        form_card(ui, |ui| {
            section_heading(
                ui,
                "App updates",
                concat!(
                    "Hush ",
                    env!("CARGO_PKG_VERSION"),
                    " · Updates from your GitHub releases"
                ),
                Icon::Arrow,
            );
            ui.checkbox(
                &mut self.draft.check_updates,
                "Check automatically at startup and every six hours",
            );
            ui.checkbox(&mut self.draft.preview_updates, "Include preview releases");
            let update = self.updater.snapshot();
            ui.add_space(8.);
            if !update.message.is_empty() {
                ui.label(RichText::new(&update.message).color(if update.error {
                    RED
                } else {
                    MUTED
                }));
            }
            if update.busy && update.release.is_some() {
                ui.add(egui::ProgressBar::new(update.progress).show_percentage());
            }
            ui.add_enabled_ui(!update.busy, |ui| {
                if action(ui, "Check for updates", Icon::Arrow, false).clicked() {
                    self.last_update_check = std::time::Instant::now();
                    self.updater
                        .check(ui.ctx().clone(), self.draft.preview_updates);
                }
            });
            if let Some(release) = &update.release {
                ui.hyperlink_to("View release on GitHub", &release.page);
                ui.add_enabled_ui(
                    cfg!(target_os = "windows") && !update.busy && !locked,
                    |ui| {
                        if update.installer.is_some() {
                            if action(ui, "Install update & restart", Icon::Arrow, true).clicked() {
                                match self.config.save().and_then(|()| self.updater.install()) {
                                    Ok(()) => {
                                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close)
                                    }
                                    Err(error) => {
                                        self.engine.state.lock().unwrap().error = error.to_string()
                                    }
                                }
                            }
                        } else if action(ui, "Download update", Icon::Arrow, true).clicked() {
                            self.updater.download(ui.ctx().clone());
                        }
                    },
                );
            }
            ui.hyperlink_to(
                "All GitHub releases",
                "https://github.com/basilaslam/hush-native/releases",
            );
            small(ui, "Save settings to keep your update preferences. Leave calls and stop microphone/server setup before downloading or installing. Installation updates this Windows folder and restarts Hush.");
        });
        ui.add_space(20.);
        if action(ui, "Save settings", Icon::Settings, true).clicked() {
            self.save();
        }
        ui.add_space(8.);
        small(ui, "Your preferences are saved only on this device.");
    }
    fn sidebar(&mut self, ctx: &egui::Context, state: &ViewState) {
        egui::SidePanel::left("navigation")
            .exact_width(218.)
            .resizable(false)
            .frame(Frame::new().fill(SIDE).inner_margin(Margin::same(18)))
            .show(ctx, |ui| {
                ui.add_space(11.);
                title(ui, "My little corner", 14.);
                ui.add_space(29.);
                small(ui, "Your space");
                ui.add_space(8.);
                let room = state
                    .session
                    .as_ref()
                    .map_or("The hangout", |session| session.room_name.as_str());
                let response = icon_text_button(
                    ui,
                    room,
                    Icon::Headphones,
                    if self.page == Page::Room { PANEL } else { SIDE },
                    TEXT,
                    true,
                );
                if response.clicked() {
                    self.page = Page::Room;
                }
                for (page, text, kind) in [
                    (Page::Connection, "Connection", Icon::Signal),
                    (Page::Settings, "Voice & settings", Icon::Settings),
                    (Page::Help, "A little help", Icon::Chat),
                ] {
                    let selected = self.page == page;
                    let response = icon_text_button(
                        ui,
                        text,
                        kind,
                        if selected { PANEL } else { SIDE },
                        TEXT,
                        true,
                    );
                    if response.clicked() {
                        self.page = page;
                        if self.page == Page::Settings {
                            self.draft = self.config.clone();
                        }
                    }
                }
                ui.add_space(35.);
                ui.separator();
                ui.add_space(22.);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(23.), Sense::hover());
                icon(ui.painter(), rect, Icon::Leaf, GREEN);
                ui.add_space(14.);
                title(ui, "A little less app.\nA little more game.", 16.);
                ui.add_space(9.);
                small(ui, "Native pixels. Native audio.\nMore room for the game.");
                ui.add_space(14.);
                if ui.checkbox(&mut self.config.quiet, "Gaming mode").changed() {
                    self.draft.quiet = self.config.quiet;
                    let _ = self.config.save();
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    small(
                        ui,
                        concat!("Hush ", env!("CARGO_PKG_VERSION"), " · native Rust"),
                    );
                    ui.add_space(9.);
                    ui.separator();
                    ui.add_space(10.);
                    ui.horizontal(|ui| {
                        avatar(ui, &self.config.name, 36., GREEN);
                        ui.vertical(|ui| {
                            title(
                                ui,
                                if self.config.name.is_empty() {
                                    "Your profile"
                                } else {
                                    &self.config.name
                                },
                                12.,
                            );
                            small(
                                ui,
                                if state.session.is_some() {
                                    "In the room"
                                } else {
                                    "Ready to connect"
                                },
                            );
                        });
                    });
                });
            });
    }
    fn footer(&mut self, ctx: &egui::Context, state: &ViewState) {
        let active = state.session.is_some();
        let connected = state
            .links
            .values()
            .filter(|link| link.state == "Connected")
            .count();
        let muted = self.engine.controls.muted.load(Ordering::Relaxed);
        let deafened = self.engine.controls.deafened.load(Ordering::Relaxed);
        egui::TopBottomPanel::bottom("voice_dock")
            .exact_height(83.)
            .frame(
                Frame::new()
                    .fill(SIDE)
                    .inner_margin(Margin::symmetric(24, 18)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.available_width() > 620. {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(38.), Sense::hover());
                        icon(
                            ui.painter(),
                            rect.shrink(8.),
                            if active {
                                Icon::Signal
                            } else {
                                Icon::Headphones
                            },
                            GREEN,
                        );
                    }
                    ui.vertical(|ui| {
                        title(
                            ui,
                            if state.busy {
                                "Getting your room ready…"
                            } else if active && connected > 0 {
                                "Voice connected"
                            } else if active {
                                "Waiting for your squad"
                            } else {
                                "A good session starts here"
                            },
                            12.,
                        );
                        small(
                            ui,
                            state
                                .session
                                .as_ref()
                                .map_or("Create a room. Bring your people.", |room| {
                                    room.room_name.as_str()
                                }),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if active {
                            if icon_button(ui, "Leave room", Icon::Power, true).clicked() {
                                self.engine.send(Command::Leave);
                            }
                        } else if state.busy && action(ui, "Cancel", Icon::Close, false).clicked() {
                            self.engine.send(Command::Leave);
                        }
                        if icon_button(ui, "Voice & settings", Icon::Settings, false).clicked() {
                            self.page = Page::Settings;
                            self.draft = self.config.clone();
                        }
                        ui.add_enabled_ui(active, |ui| {
                            if icon_button(
                                ui,
                                if deafened {
                                    "Undeafen audio"
                                } else {
                                    "Deafen audio"
                                },
                                Icon::Headphones,
                                deafened,
                            )
                            .clicked()
                            {
                                self.engine
                                    .controls
                                    .deafened
                                    .store(!deafened, Ordering::Relaxed);
                                self.engine.send(Command::ControlChanged);
                            }
                            if icon_button(
                                ui,
                                if muted {
                                    "Unmute microphone"
                                } else {
                                    "Mute microphone"
                                },
                                Icon::Mic,
                                muted,
                            )
                            .clicked()
                            {
                                self.engine.controls.muted.store(!muted, Ordering::Relaxed);
                                self.engine.send(Command::ControlChanged);
                            }
                        });
                        if active && self.config.push_to_talk {
                            let response = ui.add(
                                egui::Button::new(RichText::new("Hold to talk · F8").color(
                                    if self.engine.controls.transmitting() {
                                        INK
                                    } else {
                                        TEXT
                                    },
                                ))
                                .fill(if self.engine.controls.transmitting() {
                                    GREEN
                                } else {
                                    PANEL
                                })
                                .min_size(Vec2::new(130., 38.)),
                            );
                            let pressed = response.is_pointer_button_down_on();
                            let prior = ui.data(|data| {
                                data.get_temp::<bool>(egui::Id::new("pointer_ptt"))
                                    .unwrap_or(false)
                            });
                            if prior != pressed {
                                self.engine.ptt(pressed);
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("pointer_ptt"), pressed)
                                });
                            }
                        }
                    });
                });
            });
    }
    fn lobby(&mut self, ui: &mut egui::Ui, state: &ViewState) {
        let narrow = ui.available_width() < 600.;
        title(
            ui,
            "Your people.\nYour space.",
            if narrow { 36. } else { 43. },
        );
        ui.add_space(8.);
        small(ui, "A quiet place to talk, play, and stay a little longer.");
        ui.add_space(16.);
        let height = if narrow { 110. } else { 130. };
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        let center = rect.center();
        let painter = ui.painter();
        painter.circle_stroke(
            center,
            if narrow { 57. } else { 70. },
            Stroke::new(1.0_f32, LINE),
        );
        painter.line_segment(
            [center - Vec2::new(130., 0.), center + Vec2::new(130., 0.)],
            Stroke::new(1.0_f32, Color32::from_rgb(65, 84, 56)),
        );
        logo(painter, Rect::from_center_size(center, Vec2::splat(78.)));
        for (offset, kind, fill) in [
            (if narrow { -110. } else { -130. }, Icon::Headphones, PANEL),
            (
                if narrow { 110. } else { 130. },
                Icon::Mic,
                Color32::from_rgb(184, 172, 150),
            ),
        ] {
            let rectangle =
                Rect::from_center_size(center + Vec2::new(offset, 0.), Vec2::splat(55.));
            painter.rect_filled(rectangle, 18., fill);
            icon(
                painter,
                rectangle.shrink(15.),
                kind,
                if offset > 0. { INK } else { GREEN },
            );
        }
        ui.vertical_centered(|ui| {
            title(ui, "There’s a seat for everyone.", 23.);
            ui.add_space(7.);
            small(
                ui,
                "Start a private voice room and send your friends an invite.",
            );
            small(ui, "Just your squad, without the noise.");
            ui.add_space(21.);
            ui.scope(|ui| {
                ui.set_max_width(440.);
                form_card(ui, |ui| {
                    ui.columns(2, |columns| {
                        label(&mut columns[0], "Your name");
                        if columns[0]
                            .add(text_field(&mut self.config.name, "Your name", 24))
                            .changed()
                        {
                            self.draft.name = self.config.name.clone();
                        }
                        label(&mut columns[1], "Room name");
                        if columns[1]
                            .add(text_field(
                                &mut self.config.room_name,
                                "Give your room a name",
                                48,
                            ))
                            .changed()
                        {
                            self.draft.room_name = self.config.room_name.clone();
                        }
                    });
                    ui.add_space(8.);
                    ui.horizontal(|ui| {
                        label(ui, "Room size");
                        if ui.add(egui::DragValue::new(&mut self.config.max_members)
                            .range(2..=crate::config::MAX_ROOM_MEMBERS).suffix(" people")).changed() {
                            self.draft.max_members = self.config.max_members;
                        }
                    });
                    small(ui, "Choose 2–32 people before creating the room. Larger rooms use more CPU and upload.");
                    ui.add_space(8.);
                    ui.add_enabled_ui(!state.busy && !state.testing, |ui| {
                        ui.horizontal(|ui| {
                            if action(ui, "Create a room", Icon::Plus, true).clicked() {
                                let _ = self.config.save();
                                self.begin(None);
                            }
                            if action(ui, "Join with an invite", Icon::Link, false).clicked() {
                                self.modal = Some(Modal::Join);
                            }
                        });
                    });
                });
            });
            ui.add_space(15.);
            small(ui, "No account needed. Invitations stay between friends.");
        });
        ui.add_space(28.);
        ui.separator();
        ui.add_space(13.);
        ui.horizontal(|ui| {
            small(ui, "Native Opus audio · encrypted peer connections");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.link("Check your sound").clicked() {
                    self.page = Page::Settings;
                    self.draft = self.config.clone();
                }
            });
        });
        if self.config.server.is_empty() {
            ui.add_space(12.);
            small(
                ui,
                "First time here? Deploy your server in Voice & settings.",
            );
        }
    }
    fn room(&mut self, ui: &mut egui::Ui, state: &ViewState) {
        let Some(session) = &state.session else {
            self.lobby(ui, state);
            return;
        };
        title(ui, &session.room_name, 35.);
        small(ui, "Settle in. You’re in good company.");
        ui.add_space(24.);
        ui.horizontal(|ui| {
            title(
                ui,
                &format!(
                    "In the room  ·  {} / {}",
                    state.peers.len() + 1,
                    session.max_members
                ),
                14.,
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if action(
                    ui,
                    if self.copied {
                        "Copied"
                    } else {
                        "Invite a friend"
                    },
                    Icon::Link,
                    false,
                )
                .clicked()
                {
                    ui.ctx().copy_text(session.invitation());
                    self.copied = true;
                    self.modal = Some(Modal::Invite);
                }
            });
        });
        ui.add_space(15.);
        let mut people = vec![(
            self.config.name.clone(),
            true,
            !self.engine.controls.transmitting(),
            self.engine.controls.deafened.load(Ordering::Relaxed),
            "You".to_owned(),
        )];
        for peer in &state.peers {
            people.push((
                peer.name.clone(),
                false,
                peer.muted,
                peer.deafened,
                state
                    .links
                    .get(&peer.id)
                    .map_or("Connecting".to_owned(), |link| link.state.clone()),
            ));
        }
        let columns = if ui.available_width() > 700. {
            3
        } else if ui.available_width() > 360. {
            2
        } else {
            1
        };
        let count = people.len() + usize::from(people.len() < session.max_members);
        for row in (0..count).step_by(columns) {
            ui.columns(columns, |uis| {
                for (column, ui) in uis.iter_mut().enumerate() {
                    let index = row + column;
                    if let Some((name, self_peer, muted, deafened, connection)) = people.get(index)
                    {
                        let color = match index % 3 {
                            0 => GREEN,
                            1 => Color32::from_rgb(207, 188, 164),
                            _ => Color32::from_rgb(176, 198, 202),
                        };
                        Frame::new()
                            .fill(PANEL)
                            .corner_radius(14.)
                            .inner_margin(Margin::same(18))
                            .show(ui, |ui| {
                                ui.set_min_width((ui.available_width() - 1.).max(0.));
                                ui.set_min_height(172.);
                                ui.horizontal(|ui| {
                                    small(ui, if *self_peer { "Your seat" } else { "Squadmate" });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let (rect, _) = ui
                                            .allocate_exact_size(Vec2::splat(16.), Sense::hover());
                                        icon(
                                            ui.painter(),
                                            rect,
                                            Icon::Mic,
                                            if *muted { MUTED } else { GREEN },
                                        );
                                    });
                                });
                                ui.add_space(17.);
                                ui.vertical_centered(|ui| {
                                    avatar(ui, name, 64., color);
                                    ui.add_space(14.);
                                    title(ui, name, 16.);
                                    ui.add_space(4.);
                                    small(
                                        ui,
                                        if *deafened {
                                            "Deafened"
                                        } else if *muted {
                                            "Microphone off"
                                        } else if *self_peer {
                                            "Ready to talk"
                                        } else {
                                            connection.as_str()
                                        },
                                    );
                                });
                            });
                    } else if index == people.len() && people.len() < 6 {
                        let response = ui.add(
                            egui::Button::new(
                                RichText::new("+\n\nSave a seat\n\nInvite a friend")
                                    .size(15.)
                                    .color(GREEN),
                            )
                            .fill(Color32::from_rgb(28, 38, 29))
                            .stroke(Stroke::new(1.0_f32, LINE))
                            .min_size(Vec2::new(ui.available_width(), 208.))
                            .corner_radius(14.),
                        );
                        if response.clicked() {
                            ui.ctx().copy_text(session.invitation());
                            self.modal = Some(Modal::Invite);
                        }
                    }
                }
            });
            ui.add_space(14.);
        }
        ui.add_space(8.);
        small(
            ui,
            "Audio stays between peers. Your server only keeps the room connected.",
        );
        if !self.config.quiet {
            let level = f32::from_bits(self.engine.controls.level.load(Ordering::Relaxed));
            ui.add_space(10.);
            ui.add(
                egui::ProgressBar::new((level * 4.).min(1.))
                    .fill(GREEN)
                    .text("Your microphone"),
            );
        }
    }
    fn diagnostics(&mut self, ui: &mut egui::Ui, state: &ViewState) {
        ui.set_max_width(ui.available_width().min(820.));
        title(ui, "A clearer connection.", 32.);
        small(ui, "Real measurements from your native peer connections.");
        ui.add_space(25.);
        Frame::new()
            .fill(PANEL)
            .corner_radius(12.)
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                title(
                    ui,
                    if state.session.is_none() {
                        "Join a room to measure your connection"
                    } else if state.coordination_online {
                        "Vercel coordination is online"
                    } else {
                        "Coordination is reconnecting"
                    },
                    15.,
                );
                ui.add_space(7.);
                small(
                    ui,
                    if state.network.is_empty() {
                        "No media server, external STUN, or TURN relay is used."
                    } else {
                        &state.network
                    },
                );
            });
        ui.add_space(20.);
        for peer in &state.peers {
            let stats = state.links.get(&peer.id);
            ui.horizontal(|ui| {
                avatar(ui, &peer.name, 36., GREEN);
                ui.vertical(|ui| {
                    title(ui, &peer.name, 14.);
                    small(ui, stats.map_or("Connecting", |s| s.state.as_str()));
                });
            });
            if let Some(stats) = stats {
                if stats.state == "Connected" {
                    ui.add_space(10.);
                    ui.columns(3, |columns| {
                        small(&mut columns[0], "Round trip");
                        title(&mut columns[0], &format!("{:.0} ms", stats.rtt_ms), 20.);
                        small(&mut columns[1], "Voice packets missing");
                        let total = stats.received + stats.lost;
                        title(
                            &mut columns[1],
                            &if total == 0 {
                                "—".into()
                            } else {
                                format!("{:.1}%", stats.lost as f64 / total as f64 * 100.)
                            },
                            20.,
                        );
                        small(&mut columns[2], "Sent / received");
                        title(
                            &mut columns[2],
                            &format!("{} / {}", stats.sent, stats.received),
                            20.,
                        );
                    });
                    ui.add_space(10.);
                    small(ui, format!("Direct QUIC/TLS · {}", stats.address));
                } else if !stats.last_error.is_empty() {
                    small(ui, &stats.last_error);
                }
            }
            ui.add_space(15.);
            ui.separator();
            ui.add_space(15.);
        }
        if !state.local_address.is_empty() {
            small(ui, format!("Listening: {}", state.local_address));
            for address in &state.published_addresses {
                small(ui, format!("Published: {address}"));
            }
        }
        ui.add_space(18.);
        small(ui,"Sampled every two seconds. Loss is a cumulative sequence-gap estimate and can decrease when reordered packets arrive. Network round trip is not end-to-end voice latency.");
        ui.add_space(12.);
        small(ui,"If your carrier or router blocks direct UDP, calls may not connect. Use a reachable network or forward the configured port. Deploying a room server does not add a voice relay.");
    }
    fn help(&mut self, ui: &mut egui::Ui) {
        ui.set_max_width(ui.available_width().min(820.));
        title(ui, "A little help, before you play.", 30.);
        ui.add_space(25.);
        for(title_text,body)in[("Bring your people","Give yourself a name, create a room and copy its hush1 invitation. Friends paste it into Join with an invite in their native app. Web demo invitations are a different protocol."),("A small, native audio path","The microphone feeds a bounded sample ring. One worker encodes 20 ms mono Opus frames once, then shares those bytes with the squad. Incoming frames use a bounded jitter buffer and mix directly to the sound device."),("Your own home base","Use Deploy server in Voice & settings to sign in to Cloudflare and automatically create your room server. Its address is saved for you. Existing Vercel servers are also supported."),("Router access matters","Allow Hush on private Windows networks. Automatic UPnP opens a temporary UDP mapping only for the room. If mapping is unavailable, forward the chosen UDP port and enter the public IPv4:port. Carrier-grade NAT and some restricted networks will still block direct calls."),("Keep the game first","Gaming mode is on by default. It stops live microphone-meter repaints. F8 is registered globally only during push-to-talk calls. A game running with elevated privileges may block a shortcut; test your setup."),("Measure before promising","This is a native implementation designed for low overhead. Compare game frame times, CPU, memory and network loss on your gaming PC before claiming an improvement. No zero-FPS-loss promise is made.")]{title(ui,title_text,16.);ui.add_space(7.);ui.label(RichText::new(body).size(13.).color(MUTED));ui.add_space(23.);}
    }
    fn chat_panel(&mut self, ctx: &egui::Context, state: &ViewState) {
        egui::SidePanel::right("chat")
            .exact_width(270.)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(24, 32, 27))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                title(ui, "Room chat", 14.);
                ui.add_space(6.);
                small(ui, "Direct between friends.");
                ui.add_space(18.);
                let height = (ui.available_height() - 156.).max(80.);
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.set_min_height(height);
                        if state.chat.is_empty() {
                            ui.add_space(65.);
                            ui.vertical_centered(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(35.), Sense::hover());
                                icon(ui.painter(), rect, Icon::Chat, GREEN);
                                ui.add_space(17.);
                                title(ui, "The little things\nbetween games.", 18.);
                                ui.add_space(12.);
                                small(ui, "A link. A hello.\nOne more game?");
                            });
                        } else {
                            for message in &state.chat {
                                title(ui, &message.name, 12.);
                                ui.label(RichText::new(&message.text).size(13.).color(TEXT));
                                ui.add_space(17.);
                            }
                        }
                    });
                ui.add_space(14.);
                ui.add_enabled_ui(state.session.is_some(), |ui| {
                    let response = ui.add(text_field(&mut self.chat, "Message your room…", 500));
                    if ((response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                        || action(ui, "Send message", Icon::Arrow, false).clicked())
                        && !self.chat.trim().is_empty()
                    {
                        self.engine
                            .send(Command::Chat(std::mem::take(&mut self.chat)));
                        response.request_focus();
                    }
                });
                ui.add_space(8.);
                small(ui, "Temporary. Never stored on your server.");
            });
    }
}
fn avatar(ui: &mut egui::Ui, name: &str, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same((size * 0.32) as u8), color);
    let initials: String = if name.is_empty() {
        "Y".into()
    } else {
        name.chars().flat_map(char::to_uppercase).take(2).collect()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initials,
        FontId::proportional(size * 0.33),
        INK,
    );
}
impl eframe::App for HushApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.config.check_updates {
            let interval = std::time::Duration::from_secs(6 * 60 * 60);
            if self.last_update_check.elapsed() >= interval && !self.updater.snapshot().busy {
                self.last_update_check = std::time::Instant::now();
                self.updater.check(ctx.clone(), self.config.preview_updates);
            }
            ctx.request_repaint_after(interval.saturating_sub(self.last_update_check.elapsed()));
        }
        if let Some(server) = self.deployment.take_ready() {
            self.config.server = server.clone();
            self.draft.server = server;
            if let Err(error) = self.config.save() {
                self.engine.state.lock().unwrap().error = format!("Your server is deployed, but its address could not be saved: {error}. Save it in Settings.");
            } else {
                self.engine.state.lock().unwrap().notice =
                    "Your Cloudflare server is ready. Choose your name, then create a room.".into();
            }
        }
        let state = self.engine.state.lock().unwrap().clone();
        self.sync_hotkey(state.session.is_some());
        // The hotkey callback updates the atomic immediately, independent of UI focus.
        let pressed = self.engine.controls.pressed.load(Ordering::Relaxed);
        let prior = ctx.data(|data| {
            data.get_temp::<bool>(egui::Id::new("native_ptt"))
                .unwrap_or(false)
        });
        if pressed != prior {
            self.engine.send(Command::ControlChanged);
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("native_ptt"), pressed));
        }
        let width = ctx.content_rect().width();
        if width > 850. {
            self.sidebar(ctx, &state);
        }
        egui::TopBottomPanel::top("header")
            .exact_height(68.)
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(24, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.), Sense::hover());
                    logo(ui.painter(), rect);
                    ui.add_space(7.);
                    title(ui, "hush", 21.);
                    small(ui, "native");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, "Show or hide room chat", Icon::Chat, false).clicked() {
                            self.chat_open = !self.chat_open;
                        }
                        if icon_button(ui, "Compact window", Icon::Monitor, false).clicked() {
                            self.compact = !self.compact;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                if self.compact {
                                    Vec2::new(520., 760.)
                                } else {
                                    Vec2::new(1300., 880.)
                                },
                            ));
                        }
                        if width <= 850. {
                            if icon_button(ui, "Voice & settings", Icon::Settings, false).clicked()
                            {
                                self.page = Page::Settings;
                                self.draft = self.config.clone();
                            }
                            if icon_button(ui, "Connection diagnostics", Icon::Signal, false)
                                .clicked()
                            {
                                self.page = Page::Connection;
                            }
                            if icon_button(ui, "Room", Icon::Headphones, false).clicked() {
                                self.page = Page::Room;
                            }
                        } else {
                            small(ui, "Private by invitation");
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(14.), Sense::hover());
                            icon(ui.painter(), rect, Icon::Lock, MUTED);
                        }
                    });
                });
            });
        self.footer(ctx, &state);
        if self.chat_open && width > 1100. {
            self.chat_panel(ctx, &state);
        }
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::same(if width < 700. { 22 } else { 34 })),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(match self.page {
                        Page::Room => "room_content",
                        Page::Settings => "settings_content",
                        Page::Connection => "connection_content",
                        Page::Help => "help_content",
                    })
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !state.error.is_empty() {
                            Frame::new()
                                .fill(Color32::from_rgb(61, 42, 36))
                                .corner_radius(8.)
                                .inner_margin(Margin::same(12))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(RichText::new(&state.error).size(13.).color(RED));
                                        if ui.small_button("Dismiss").clicked() {
                                            self.engine.state.lock().unwrap().error.clear();
                                        }
                                    });
                                });
                            ui.add_space(16.);
                        }
                        if !state.coordination_online || !state.notice.is_empty() {
                            small(ui, &state.notice);
                            ui.add_space(12.);
                        }
                        match self.page {
                            Page::Room => self.room(ui, &state),
                            Page::Settings => self.settings(ui, &state),
                            Page::Connection => self.diagnostics(ui, &state),
                            Page::Help => self.help(ui),
                        }
                        if state.busy {
                            ui.add_space(15.);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                small(ui, &state.network);
                            });
                        }
                    });
            });
        if let Some(kind) = self.modal.as_ref() {
            let join = *kind == Modal::Join;
            let modal = egui::Modal::new(egui::Id::new("room_modal"))
                .frame(Frame::new().fill(SIDE).stroke(Stroke::new(1.0_f32, LINE))
                    .corner_radius(16.).inner_margin(Margin::same(24)))
                .show(ctx, |ui| {
                ui.set_width(400f32.min((width - 80.).max(240.)));
                ui.spacing_mut().item_spacing.y = 10.;
                title(ui, if join { "Your people are waiting." } else { "Save a seat for a friend." }, 24.);
                ui.add_space(4.);
                small(ui, if join { "A name and an invitation. You’re almost there." } else { "Make a little room for your squad." });
                ui.add_space(16.);
                if join {
                    label(ui, "Your name");
                    ui.add(text_field(&mut self.config.name, "Your display name", 24));
                    ui.add_space(8.);
                    label(ui, "Native room invitation");
                    ui.add(text_field(&mut self.invitation, "hush1.…", 100));
                    ui.add_space(16.);
                    if action(ui, "Join room & connect", Icon::Arrow, true).clicked() {
                        self.begin(Some(self.invitation.clone()));
                    }
                } else if let Some(session) = &state.session {
                    let mut code = session.invitation();
                    ui.add(text_field(&mut code, "", 100).interactive(false));
                    ui.add_space(12.);
                    if action(ui, "Copy invitation", Icon::Link, true).clicked() {
                        ctx.copy_text(session.invitation()); self.copied = true;
                    }
                    small(ui, "Anyone with the invitation can join until the room reaches its selected size. Keep it between friends.");
                }
                ui.add_space(10.);
                if action(ui, "Close", Icon::Close, false).clicked() { self.modal = None; }
            });
            if modal.should_close() {
                self.modal = None;
            }
        }
        if state.testing
            || (state.session.is_some()
                && !self.config.quiet
                && ctx.input(|input| input.viewport().focused.unwrap_or(false)))
        {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine.send(Command::Shutdown);
        if self.hotkey_registered {
            if let Some(manager) = &self.hotkey {
                let _ = manager.unregister(HotKey::new(None, Code::F8));
            }
        }
    }
}
