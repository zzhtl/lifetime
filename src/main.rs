// Lifetime 健康助手 —— 主入口
// 启动 eframe 主窗口；Scheduler 线程随 AppState 一起拉起

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod db;
mod notify;
mod reminders;
mod scheduler;
mod stats;
mod tips;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cfg = config::load_or_default()?;
    log::info!("配置加载完成: {:?}", cfg.paths.data_dir);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lifetime 健康助手")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([800.0, 560.0]),
        ..Default::default()
    };

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
