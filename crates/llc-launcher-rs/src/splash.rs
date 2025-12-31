use eframe::{
    CreationContext, egui,
    egui::{CentralPanel, Color32, FontData, Frame, Layout, ProgressBar, ViewportCommand},
    emath::Align,
};
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use std::sync::{Arc, Mutex};

static STATE_TEXT: Mutex<String> = Mutex::new(String::new());
static ERROR_TEXT: Mutex<String> = Mutex::new(String::new());
static FONTS: &[&str] = &[
    "Microsoft YaHei",
    "PingFang SC",
    "Source Han Sans CN",
    "WenQuanYi Micro Hei",
    "Noto Sans CJK SC",
];

pub fn set_state_str(text: &str) {
    let mut state_text = STATE_TEXT.lock().unwrap();
    *state_text = text.to_string();
}

pub fn set_state_string(text: String) {
    let mut state_text = STATE_TEXT.lock().unwrap();
    *state_text = text;
}

pub fn set_error_string(text: String) {
    let mut error_text = ERROR_TEXT.lock().unwrap();
    *error_text = text;
}

pub struct SplashScreen {
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    should_quit: bool,
    display_progress: f32,
    show_animation: bool,
}

impl SplashScreen {
    pub fn new(
        cc: &CreationContext,
        is_tool: bool,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();

        let mut font_data = None;
        for name in FONTS {
            if let Ok(handle) = SystemSource::new()
                .select_best_match(&[FamilyName::Title(name.to_string())], &Properties::new())
            {
                if let Ok(data) = handle.load() {
                    font_data = data.copy_font_data();
                    break;
                }
            }
        }
        if let Some(data) = font_data {
            fonts.font_data.insert(
                "SystemChinese".to_owned(),
                Arc::new(FontData::from_owned((*data).clone())),
            );

            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "SystemChinese".to_owned());
            cc.egui_ctx.set_fonts(fonts);
        }

        Self {
            shutdown_rx,
            should_quit: false,
            display_progress: 0.0,
            show_animation: is_tool,
        }
    }
}

impl eframe::App for SplashScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.shutdown_rx.try_recv().is_ok() {
            self.should_quit = true;
        }
        // Close immediately if we're a launcher
        if !self.show_animation && self.should_quit {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        ctx.request_repaint();

        let state_text = {
            let guard = STATE_TEXT.lock().unwrap();
            guard.clone()
        };
        let error_text = {
            let guard = ERROR_TEXT.lock().unwrap();
            guard.clone()
        };
        let has_error = !error_text.is_empty();
        if !self.show_animation {
            ctx.send_viewport_cmd(ViewportCommand::Visible(has_error));
        }

        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        if !self.should_quit {
            if self.display_progress < 0.8 {
                self.display_progress += dt * 0.4;
            } else if self.display_progress < 0.99 {
                self.display_progress += dt * 0.02;
            }
        } else {
            if self.display_progress < 1.0 {
                self.display_progress += dt * 3.0;
            } else if !has_error {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        CentralPanel::default()
            .frame(Frame::new().inner_margin(100))
            .show(ctx, |ui| {
                let rect = ui.ctx().viewport_rect();
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 20));
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Limbus Company 启动器")
                            .color(Color32::WHITE)
                            .size(50.0),
                    );
                });

                if has_error {
                    ui.vertical_centered(|ui| {
                        ui.add_space(30.0);
                        egui::ScrollArea::vertical()
                            .max_height(500.0)
                            .show(ui, |ui| {
                                ui.set_min_width(800.0);
                                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                    ui.label(
                                        egui::RichText::new(error_text)
                                            .color(Color32::RED)
                                            .size(18.0),
                                    );
                                });
                            });
                        ui.add_space(20.0);
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("我知道了").size(30.0))
                                    .min_size(egui::vec2(200.0, 60.0)),
                            )
                            .clicked()
                        {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });
                    return;
                }

                ui.add_space(100.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(state_text)
                            .color(Color32::WHITE)
                            .size(30.0),
                    );
                });

                ui.centered_and_justified(|ui| {
                    ui.add(
                        ProgressBar::new(self.display_progress)
                            .desired_width(800.0)
                            .desired_height(50.0)
                            .animate(true),
                    );
                });
            });
    }
}
