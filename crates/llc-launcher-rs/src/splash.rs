use crate::utils::{consume_next_error, next_error};
use eframe::{
    CreationContext, egui,
    egui::{CentralPanel, Color32, Frame, Layout, ViewportCommand},
    emath::Align,
    epaint::StrokeKind,
};
use egui::{
    Align2, CornerRadius, FontFamily, FontId, Label, Pos2, Rect, Response, RichText, ScrollArea,
    Stroke, UiBuilder, Vec2, Widget, pos2, vec2,
};
use std::{f32::consts::PI, time::Instant};

mod color;
mod font;
mod style;

const TARGET_SIZE: Vec2 = vec2(1280.0, 800.0);
const CENTER: Pos2 = pos2(640.0, 400.0);

pub struct SplashScreen {
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    should_quit: bool,
    progress: f32,
    show_animation: bool,
    scale: f32,

    start_time: Instant,
    glitch_offset_logic: Vec2,
}

impl SplashScreen {
    pub fn new(
        cc: &CreationContext,
        is_tool: bool,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        font::load(&cc.egui_ctx);
        style::setup(&cc.egui_ctx);

        Self {
            shutdown_rx,
            should_quit: false,
            progress: 0.0,
            show_animation: is_tool,
            scale: 1.0,

            start_time: Instant::now(),
            glitch_offset_logic: Vec2::ZERO,
        }
    }

    fn update_resolution(&mut self, ctx: &egui::Context) {
        let monitor_size = ctx.input(|i| i.viewport().monitor_size).unwrap_or(TARGET_SIZE);

        let viewport = ctx.viewport_rect().size();
        let target_size = {
            const RATIO: f32 = 0.4;
            let target_width = monitor_size.x * RATIO;
            let target_height = TARGET_SIZE.y / TARGET_SIZE.x * target_width;

            vec2(target_width, target_height)
        };

        if viewport == target_size {
            return;
        }

        ctx.send_viewport_cmd(ViewportCommand::InnerSize(target_size));
        self.scale = viewport.x / TARGET_SIZE.x;

        if let Some(current_pos) = ctx.input(|i| i.viewport().outer_rect) {
            let current_pos = current_pos.left_top();
            let center_x = (monitor_size.x - target_size.x) / 2.0;
            let center_y = (monitor_size.y - target_size.y) / 2.0;
            let target_pos = pos2(center_x, center_y);

            if (current_pos - target_pos).length() > 2.0 {
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(target_pos));
            }
        }
    }

    fn update_fake_progress(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        if !self.should_quit {
            if self.progress < 0.8 {
                self.progress += dt * 0.4;
            } else if self.progress < 0.99 {
                self.progress += dt * 0.02;
            }
        } else if self.progress < 1.0 {
            self.progress += dt * 3.0;
        }
        if self.should_quit && self.progress >= 1.0 {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_button(
        &self,
        ui: &mut egui::Ui,
        // text
        text: &str,
        color: Color32,
        color_hovered: Color32,
        // fill
        fill: Color32,
        fill_hovered: Color32,
        // stroke
        stroked: Option<Color32>,
    ) -> Response {
        let btn_width = 180.0 * self.scale;
        let btn_height = 45.0 * self.scale;

        let (id, rect) = ui.allocate_space(vec2(btn_width, btn_height));

        let response = ui.interact(rect, id, egui::Sense::click());

        let is_hovered = response.hovered();

        let skew_x = 15.0 * self.scale;
        let text_color = if is_hovered { color_hovered } else { color };

        let painter = ui.painter();

        let points = vec![
            rect.left_top() + vec2(skew_x, 0.0),
            rect.right_top(),
            rect.right_bottom() - vec2(skew_x, 0.0),
            rect.left_bottom(),
        ];
        painter.add(egui::Shape::convex_polygon(
            points,
            if is_hovered { fill_hovered } else { fill },
            if let Some(stroke_color) = stroked {
                Stroke::new(2.0 * self.scale, stroke_color)
            } else {
                Stroke::NONE
            },
        ));

        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::new(16.0 * self.scale, font::SANS_SERIF_BOLD.clone()),
            text_color,
        );

        response
    }

    fn paint_clock(&mut self, ui: &mut egui::Ui, is_error: bool, elapsed: f32) {
        let radius = 160.0 * self.scale;
        let primary_color = if is_error { color::RED } else { color::GOLD };

        let rect = ui.max_rect();
        let center = rect.left_top() + vec2(40.0 * self.scale, 30.0 * self.scale);

        let painter = ui.painter();

        painter.circle_filled(center, radius, Color32::from_black_alpha(200));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(5.0 * self.scale, primary_color.linear_multiply(0.3)),
        );

        for i in 0..12 {
            let angle = (i as f32) * (PI / 6.0);
            let is_major = i % 3 == 0;
            let dir = vec2(angle.sin(), -angle.cos());
            let tick_len = (if is_major { 16.0 } else { 12.0 }) * self.scale;
            let tick_width = (if is_major { 5.0 } else { 2.0 }) * self.scale;

            painter.line_segment(
                [
                    center + dir * (radius - 4.0 * self.scale),
                    center + dir * (radius - tick_len),
                ],
                Stroke::new(
                    tick_width,
                    primary_color.linear_multiply(if is_major { 0.8 } else { 0.4 }),
                ),
            );
        }

        let rotation_speed = if is_error {
            -elapsed * 100.0
        } else {
            elapsed * 30.0
        };
        let hands = [
            (
                rotation_speed * 1.2,
                radius * 0.8,
                2.0 * self.scale,
                Color32::WHITE.linear_multiply(0.4),
            ),
            (
                rotation_speed * 0.3,
                radius * 0.7,
                4.0 * self.scale,
                Color32::DARK_RED,
            ),
            (
                rotation_speed * 0.05,
                radius * 0.5,
                6.0 * self.scale,
                primary_color,
            ),
        ];

        for (angle, len, width, color) in hands {
            let dir = vec2(angle.sin(), -angle.cos());
            painter.line_segment([center, center + dir * len], Stroke::new(width, color));
        }

        painter.circle_filled(center, 2.5 * self.scale, Color32::from_rgb(10, 10, 10));
        painter.circle_stroke(
            center,
            2.5 * self.scale,
            Stroke::new(1.0 * self.scale, primary_color),
        );
    }

    fn paint_logo(&mut self, ui: &mut egui::Ui, is_error: bool) {
        let color = if is_error {
            color::RED
        } else {
            color::DARK_RED
        };

        let rect = ui.max_rect();

        let logo_pos_phys =
            rect.left_top() + (CENTER.to_vec2() + self.glitch_offset_logic) * self.scale;
        let logo_size = vec2(800.0 * self.scale, 300.0 * self.scale);
        ui.scope_builder(
            UiBuilder::new().max_rect(Rect::from_center_size(logo_pos_phys, logo_size)),
            |ui| {
                ui.vertical_centered(|ui| {
                    Label::new(
                        RichText::new("LIMBUS COMPANY")
                            .size(80.0 * self.scale)
                            .family(font::SERIF.clone())
                            .strong()
                            .italics()
                            .color(color),
                    )
                    .selectable(false)
                    .ui(ui);
                    Label::new(
                        RichText::new("LAUNCHER")
                            .size(28.0 * self.scale)
                            .family(font::SERIF.clone())
                            .extra_letter_spacing(25.0 * self.scale)
                            .color(color::GOLD),
                    )
                    .selectable(false)
                    .ui(ui);
                });
            },
        );
    }

    fn paint_error_window(&mut self, ui: &mut egui::Ui, error: String) {
        let rect = ui.max_rect();
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from_black_alpha(110));

        let modal_pos_phys = rect.left_top() + CENTER.to_vec2() * self.scale;
        let modal_size = vec2(1000.0 * self.scale, 600.0 * self.scale);
        let modal_rect = Rect::from_center_size(modal_pos_phys, modal_size);

        let shadow_color = Color32::from_rgba_premultiplied(255, 0, 0, 40);
        for i in 1..=30 {
            let alpha = (1.0 - (i as f32 / 30.0)).powi(2) * 0.4;
            let expansion = i as f32 * 3.0 * self.scale;
            ui.painter().rect_filled(
                modal_rect.expand(expansion),
                50.0 * self.scale,
                shadow_color.linear_multiply(alpha),
            );
        }

        ui.painter()
            .rect_filled(modal_rect, 0.0, Color32::from_black_alpha(240));
        ui.painter().rect_stroke(
            modal_rect,
            0.0,
            Stroke::new(2.0 * self.scale, color::RED),
            StrokeKind::Inside,
        );

        // top red bar
        let top_bar_height = 60.0 * self.scale;
        let top_bar_rect = Rect::from_min_size(
            pos2(modal_rect.min.x, modal_rect.min.y),
            vec2(modal_rect.width(), top_bar_height),
        );
        ui.painter().rect_filled(top_bar_rect, 0.0, color::RED);
        // top bar left text
        let title_rect = top_bar_rect.shrink2(vec2(20.0 * self.scale, 0.0));
        ui.scope_builder(UiBuilder::new().max_rect(title_rect), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(
                    Label::new(
                        RichText::new("严重错误")
                            .size(30.0 * self.scale)
                            .family(font::SANS_SERIF_BOLD.clone())
                            .extra_letter_spacing(5.0 * self.scale)
                            .color(color::BLACK)
                            .italics(),
                    )
                    .selectable(false),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(
                        Label::new(
                            RichText::new("CODE: 0x000000FF")
                                .size(16.0 * self.scale)
                                .family(FontFamily::Monospace)
                                .color(color::BLACK)
                                .italics()
                                .strong(),
                        )
                        .selectable(false),
                    );
                });
            });
        });

        let below_rect = Rect::from_min_size(
            pos2(top_bar_rect.min.x, top_bar_rect.max.y),
            vec2(modal_rect.width(), modal_rect.height() - top_bar_height),
        );
        let error_container_rect = Rect::from_min_size(
            pos2(
                below_rect.min.x + 40.0 * self.scale,
                below_rect.min.y + 45.0 * self.scale,
            ),
            vec2(
                below_rect.width() - 80.0 * self.scale,
                below_rect.height() - 190.0 * self.scale,
            ),
        );
        ui.scope_builder(UiBuilder::new().max_rect(error_container_rect), |ui| {
            ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Label::new(
                        RichText::new(error)
                            .size(16.0 * self.scale)
                            .family(FontFamily::Monospace)
                            .color(color::DARK_RED),
                    )
                    .extend()
                    .selectable(false)
                    .ui(ui)
                })
        });

        // draw lines to separate buttons
        let line_y = below_rect.max.y - 100.0 * self.scale;
        ui.painter().line_segment(
            [
                pos2(below_rect.min.x + 10.0 * self.scale, line_y),
                pos2(below_rect.max.x - 10.0 * self.scale, line_y),
            ],
            Stroke::new(1.0 * self.scale, color::RED.linear_multiply(0.5)),
        );

        let bottom_rect = Rect::from_min_size(
            pos2(below_rect.min.x, below_rect.max.y - 100.0 * self.scale),
            vec2(below_rect.width(), 100.0 * self.scale),
        )
        .shrink(10.0 * self.scale);
        ui.scope_builder(UiBuilder::new().max_rect(bottom_rect), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(10.0 * self.scale); // 右边距

                if self
                    .draw_button(
                        ui,
                        "我知道了",
                        color::WHITE,
                        color::WHITE,
                        color::DARK_RED,
                        color::RED,
                        None,
                    )
                    .clicked()
                {
                    consume_next_error();
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
                // ui.add_space(20.0 * self.scale);
                // self.draw_button(ui, "复制崩溃日志", color::RED, color::WHITE, color::BLACK, color::RED, Some(color::RED));
            });
        });
    }

    fn paint_progress_bar(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let bar_width = 1000.0 * self.scale;
        let bar_height = 20.0 * self.scale;
        let bar_rect = Rect::from_center_size(
            rect.center() + vec2(0.0, 150.0 * self.scale),
            vec2(bar_width, bar_height),
        );

        ui.painter()
            .rect_filled(bar_rect, 10.0 * self.scale, Color32::from_black_alpha(150));
        ui.painter().rect_filled(
            Rect::from_min_size(bar_rect.min, vec2(bar_width * self.progress, bar_height)),
            10.0 * self.scale,
            color::GOLD,
        );
    }
}

impl eframe::App for SplashScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // poll for shutdown signal
        if self.shutdown_rx.try_recv().is_ok() {
            self.should_quit = true;
        }
        // Close immediately if we don't need to show animation
        if !self.show_animation && self.should_quit {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        let next_error = next_error();
        if !self.show_animation {
            ctx.send_viewport_cmd(ViewportCommand::Visible(next_error.is_some()));
        }

        // let height = ctx.content_rect().height();

        // let font_title = height * 0.07;
        // let font_body = height * 0.03;
        // let font_small = height * 0.02;

        self.update_resolution(ctx);
        self.update_fake_progress(ctx);

        let elapsed = self.start_time.elapsed().as_secs_f32();

        if (elapsed * 10.0).sin() > 0.95 {
            self.glitch_offset_logic = vec2(elapsed.cos() * 20.0 * self.scale, 0.0);
        } else {
            self.glitch_offset_logic = Vec2::ZERO;
        }

        CentralPanel::default()
            .frame(Frame {
                fill: color::COLOR_BG,
                corner_radius: CornerRadius::ZERO,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ctx.request_repaint();

                if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_down()) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }

                self.paint_clock(ui, next_error.is_some(), elapsed);
                self.paint_logo(ui, next_error.is_some());

                if let Some(error) = next_error {
                    self.paint_error_window(ui, error);
                } else {
                    self.paint_progress_bar(ui);
                }
            });
    }
}
