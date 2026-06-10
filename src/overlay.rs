use std::time::{Duration, Instant};

use chrono::Utc;
use eframe::egui;

use crate::config::Config;
use crate::model::WindowUsage;
use crate::{render, snapshot};

const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// Floating always-on-top bar. Transparent, frameless, and click-through,
/// so it can sit over any window without stealing input.
pub fn run(config: Config) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 64.0])
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_taskbar(false),
        ..Default::default()
    };

    eframe::run_native(
        "limitbar",
        options,
        Box::new(|cc| {
            // Mouse passthrough makes the bar click-through; releasing it
            // is a config/keybind concern for later versions.
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
            Ok(Box::new(OverlayApp::new(config)))
        }),
    )
    .map_err(|e| e.to_string())
}

struct OverlayApp {
    config: Config,
    usages: Vec<WindowUsage>,
    last_refresh: Instant,
}

impl OverlayApp {
    fn new(config: Config) -> Self {
        let (usages, _) = snapshot::collect(&config, Utc::now());
        Self {
            config,
            usages,
            last_refresh: Instant::now(),
        }
    }

    fn refresh_if_due(&mut self) {
        if self.last_refresh.elapsed() >= REFRESH_EVERY {
            let (usages, _) = snapshot::collect(&self.config, Utc::now());
            self.usages = usages;
            self.last_refresh = Instant::now();
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.refresh_if_due();
        ctx.request_repaint_after(REFRESH_EVERY);

        let panel_frame = egui::Frame::NONE
            .fill(egui::Color32::from_black_alpha(160))
            .corner_radius(6.0)
            .inner_margin(8.0);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let now = Utc::now();
                for usage in &self.usages {
                    ui.label(
                        egui::RichText::new(render::render_line(usage, now))
                            .monospace()
                            .size(12.0)
                            .color(egui::Color32::from_gray(230)),
                    );
                }
                if self.usages.is_empty() {
                    ui.label(
                        egui::RichText::new("limitbar: no usage data")
                            .monospace()
                            .color(egui::Color32::from_gray(200)),
                    );
                }
            });
    }
}
