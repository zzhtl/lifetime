// Lifetime 健康助手 —— 主入口
// 启动 eframe 主窗口；Scheduler 线程随 AppState 一起拉起

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod breathing;
mod config;
mod db;
mod notify;
mod practices;
mod reminders;
mod scheduler;
mod shortcut;
mod stats;
mod tips;
mod ui;

use anyhow::Result;

const MAIN_WINDOW_SIZE: [f32; 2] = [1280.0, 720.0];

fn enforce_main_window_size(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    viewport
        .with_inner_size(MAIN_WINDOW_SIZE)
        .with_min_inner_size(MAIN_WINDOW_SIZE)
        .with_max_inner_size(MAIN_WINDOW_SIZE)
        .with_resizable(false)
}

fn main_viewport() -> egui::ViewportBuilder {
    enforce_main_window_size(
        egui::ViewportBuilder::default().with_title("Lifetime 健康助手"),
    )
}

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: main_viewport(),
        persist_window: false,
        window_builder: Some(Box::new(enforce_main_window_size)),
        ..Default::default()
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg = config::load_or_default()?;
    log::info!("配置加载完成: {:?}", cfg.paths.data_dir);

    // 运行时确保桌面快捷方式存在（路径变化才覆盖）
    shortcut::ensure(&cfg.paths.data_dir);

    let native_options = native_options();

    eframe::run_native(
        "Lifetime",
        native_options,
        Box::new(move |cc| {
            // 注入中文字体（系统字体回退），再应用整体视觉风格
            ui::fonts::install_cjk_fonts(&cc.egui_ctx);
            ui::theme::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc, cfg).expect("初始化失败")))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe 启动失败: {e}"))?;

    Ok(())
}

pub use eframe::egui;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_window_has_fixed_dimensions() {
        let mut native_options = native_options();
        let size = Some(egui::vec2(MAIN_WINDOW_SIZE[0], MAIN_WINDOW_SIZE[1]));
        let assert_fixed = |viewport: &egui::ViewportBuilder| {
            assert_eq!(viewport.inner_size, size);
            assert_eq!(viewport.min_inner_size, size);
            assert_eq!(viewport.max_inner_size, size);
            assert_eq!(viewport.resizable, Some(false));
        };

        assert_fixed(&native_options.viewport);
        assert!(!native_options.persist_window);

        let restored_viewport =
            egui::ViewportBuilder::default().with_inner_size([1440.0, 640.0]);
        let window_builder = native_options
            .window_builder
            .take()
            .expect("固定窗口应覆盖历史尺寸");
        assert_fixed(&window_builder(restored_viewport));
    }
}
