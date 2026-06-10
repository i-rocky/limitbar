use std::time::{Duration, Instant};

use chrono::Utc;
use device_query::{DeviceQuery, DeviceState, Keycode};
use eframe::egui;

use crate::config::{self, Config};
use crate::model::WindowUsage;
use crate::{render, snapshot};

const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// Floating always-on-top bar. Click-through by default; holding
/// Ctrl+Alt while the pointer is over it unlocks the window so it can
/// be dragged. The position survives restarts via the state file.
pub fn run(config: Config) -> Result<(), String> {
    let state = config::load_state();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([420.0, 72.0])
        .with_always_on_top()
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(false);
    if let Some([x, y]) = state.overlay_position {
        viewport = viewport.with_position([x, y]);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "limitbar",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
            Ok(Box::new(OverlayApp::new(config)))
        }),
    )
    .map_err(|e| e.to_string())
}

fn parse_color(value: &str, fallback: egui::Color32) -> egui::Color32 {
    let hex = value.trim().trim_start_matches('#');
    let parse = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        6 => match (parse(&hex[0..2]), parse(&hex[2..4]), parse(&hex[4..6])) {
            (Some(r), Some(g), Some(b)) => egui::Color32::from_rgb(r, g, b),
            _ => fallback,
        },
        8 => match (
            parse(&hex[0..2]),
            parse(&hex[2..4]),
            parse(&hex[4..6]),
            parse(&hex[6..8]),
        ) {
            (Some(r), Some(g), Some(b), Some(a)) => {
                egui::Color32::from_rgba_unmultiplied(r, g, b, a)
            }
            _ => fallback,
        },
        _ => fallback,
    }
}

struct OverlayApp {
    config: Config,
    usages: Vec<WindowUsage>,
    last_refresh: Instant,
    device: DeviceState,
    unlocked: bool,
    background: egui::Color32,
    text_color: egui::Color32,
    font_size: f32,
}

impl OverlayApp {
    fn new(config: Config) -> Self {
        let (usages, _) = snapshot::collect(&config, Utc::now());

        let overlay = &config.overlay;
        let mut background = parse_color(
            overlay.background.as_deref().unwrap_or(""),
            egui::Color32::from_black_alpha(160),
        );
        if let Some(opacity) = overlay.opacity {
            let a = (background.a() as f32 * opacity.clamp(0.0, 1.0)) as u8;
            background = egui::Color32::from_rgba_unmultiplied(
                background.r(),
                background.g(),
                background.b(),
                a,
            );
        }
        let text_color = parse_color(
            overlay.text.as_deref().unwrap_or(""),
            egui::Color32::from_gray(230),
        );
        let font_size = overlay.font_size.unwrap_or(12.0).clamp(6.0, 48.0);

        Self {
            config,
            usages,
            last_refresh: Instant::now(),
            device: DeviceState::new(),
            unlocked: false,
            background,
            text_color,
            font_size,
        }
    }

    fn refresh_if_due(&mut self) {
        if self.last_refresh.elapsed() >= REFRESH_EVERY {
            let (usages, _) = snapshot::collect(&self.config, Utc::now());
            self.usages = usages;
            self.last_refresh = Instant::now();
        }
    }

    /// Ctrl+Alt held with the pointer over the window unlocks it.
    /// Global polling is required because a click-through window never
    /// receives input events of its own.
    fn wants_unlock(&self, ctx: &egui::Context) -> bool {
        let keys = self.device.get_keys();
        let ctrl = keys.contains(&Keycode::LControl) || keys.contains(&Keycode::RControl);
        let alt = keys.contains(&Keycode::LAlt) || keys.contains(&Keycode::RAlt);
        if !(ctrl && alt) {
            return false;
        }

        let mouse = self.device.get_mouse();
        let (mx, my) = (mouse.coords.0 as f32, mouse.coords.1 as f32);
        let scale = ctx.pixels_per_point();
        ctx.input(|i| i.viewport().outer_rect)
            .map(|rect| {
                let px = egui::Rect::from_min_max(
                    egui::pos2(rect.min.x * scale, rect.min.y * scale),
                    egui::pos2(rect.max.x * scale, rect.max.y * scale),
                );
                px.contains(egui::pos2(mx, my))
            })
            .unwrap_or(false)
    }

    fn persist_position(&self, ctx: &egui::Context) {
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            config::save_state(&config::State {
                overlay_position: Some([rect.min.x, rect.min.y]),
            });
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_if_due();

        let unlock = self.wants_unlock(ctx);
        if unlock != self.unlocked {
            self.unlocked = unlock;
            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(!unlock));
            if !unlock {
                // Re-locking marks the end of a drag: remember where it landed.
                self.persist_position(ctx);
            }
        }

        // Poll fast while interactive (or near-interactive); idle otherwise.
        ctx.request_repaint_after(if self.unlocked {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(250)
        });

        let stroke = if self.unlocked {
            egui::Stroke::new(1.5, egui::Color32::from_rgb(88, 166, 255))
        } else {
            egui::Stroke::NONE
        };
        let panel_frame = egui::Frame::NONE
            .fill(self.background)
            .stroke(stroke)
            .corner_radius(6.0)
            .inner_margin(8.0);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                if self.unlocked {
                    // Any press-and-move while unlocked drags the window.
                    let response = ui.interact(
                        ui.max_rect(),
                        ui.id().with("drag-region"),
                        egui::Sense::drag(),
                    );
                    if response.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                }

                let now = Utc::now();
                for usage in &self.usages {
                    ui.label(
                        egui::RichText::new(render::render_line(usage, now))
                            .monospace()
                            .size(self.font_size)
                            .color(self.text_color),
                    );
                }
                if self.usages.is_empty() {
                    ui.label(
                        egui::RichText::new("limitbar: no usage data")
                            .monospace()
                            .size(self.font_size)
                            .color(self.text_color),
                    );
                }
                if self.unlocked {
                    ui.label(
                        egui::RichText::new("unlocked — drag to move, release Ctrl+Alt to lock")
                            .size(self.font_size * 0.85)
                            .color(egui::Color32::from_rgb(88, 166, 255)),
                    );
                }
            });
    }
}
