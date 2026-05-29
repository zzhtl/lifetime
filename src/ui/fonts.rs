// 中文字体注入
//
// 关键坑：
//   * NotoSansCJK-Regular.ttc 是 TrueType Collection，包含 JP/KR/SC/TC/HK 多个 face。
//     默认 index 0 是日文，不能覆盖简体中文（"喝/颈/腰"等会变方块）。
//   * 必须 face index 2（SC）才正确。
//
// 探测顺序：
//   1. 环境变量 LIFETIME_FONT="path[:index]" — 用户强制指定
//   2. fontconfig：`fc-match -f "%{file}|%{index}" sans-serif:lang=zh-cn`
//      —— Linux/macOS 上都最稳，能拿到正确的 ttc face index
//   3. 静态候选清单（兜底）

use eframe::egui::{self, FontData};

pub fn install_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidate = resolve_font();
    if let Some((bytes, index, source)) = candidate {
        let name = "cjk".to_owned();
        let mut data = FontData::from_owned(bytes);
        data.index = index;
        fonts.font_data.insert(name.clone(), data);
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, name.clone());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(name);
        log::info!("已加载中文字体: {source} (face={index})");
        ctx.set_fonts(fonts);
        return;
    }

    log::warn!("未找到系统中文字体，中文将显示为方块。请安装 fonts-noto-cjk 或设置 LIFETIME_FONT 环境变量");
    ctx.set_fonts(fonts);
}

// 提示：字体加载后由 main.rs 调用 theme::install 设置整体风格。

/// 返回 (字体字节, face index, 来源描述)
fn resolve_font() -> Option<(Vec<u8>, u32, String)> {
    // 1) 环境变量
    if let Ok(spec) = std::env::var("LIFETIME_FONT") {
        let (path, idx) = parse_spec(&spec);
        if let Ok(bytes) = std::fs::read(&path) {
            return Some((bytes, idx, format!("env:{path}")));
        } else {
            log::warn!("LIFETIME_FONT 指定的字体不可读: {path}");
        }
    }

    // 2) fontconfig（Linux / macOS 上 fc-match 通常都可用）
    if let Some((path, idx)) = via_fontconfig() {
        if let Ok(bytes) = std::fs::read(&path) {
            return Some((bytes, idx, format!("fc-match:{path}")));
        }
    }

    // 3) 静态候选清单
    for (path, idx) in static_candidates() {
        if let Ok(bytes) = std::fs::read(path) {
            return Some((bytes, *idx, format!("static:{path}")));
        }
    }

    None
}

fn parse_spec(spec: &str) -> (String, u32) {
    if let Some((p, i)) = spec.rsplit_once(':') {
        if let Ok(idx) = i.parse::<u32>() {
            return (p.to_owned(), idx);
        }
    }
    (spec.to_owned(), 0)
}

fn via_fontconfig() -> Option<(String, u32)> {
    // 优先指定 SC 变体，失败再回落到 zh-cn 通配
    for pattern in [
        "Noto Sans CJK SC",
        "Source Han Sans SC",
        "PingFang SC",
        "Microsoft YaHei",
        "sans-serif:lang=zh-cn",
    ] {
        if let Some(pair) = run_fc_match(pattern) {
            return Some(pair);
        }
    }
    None
}

fn run_fc_match(pattern: &str) -> Option<(String, u32)> {
    let out = std::process::Command::new("fc-match")
        .args(["-f", "%{file}|%{index}", pattern])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let (file, idx) = s.split_once('|')?;
    let file = file.trim();
    let idx: u32 = idx.trim().parse().unwrap_or(0);
    // fc-match 即便找不到也会回退一个非中文字体；过滤掉明显不含 CJK 的
    if file.is_empty() {
        return None;
    }
    // 拒绝 latin-only 字体（粗略：文件名含 dejavu / liberation / freeserif 且 pattern 是 lang=zh）
    let lower = file.to_lowercase();
    if pattern.contains("zh") {
        let bad = ["dejavu", "liberation", "freesans", "freeserif", "freemono"];
        if bad.iter().any(|b| lower.contains(b)) {
            return None;
        }
    }
    Some((file.to_owned(), idx))
}

/// 已知发行版默认安装的中文字体路径
/// (path, face_index) —— ttc 必须给对 index 才能渲染简体中文
fn static_candidates() -> &'static [(&'static str, u32)] {
    #[cfg(target_os = "linux")]
    {
        &[
            // Debian / Ubuntu fonts-noto-cjk
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Medium.ttc", 2),
            // Arch / Fedora noto-fonts-cjk
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/google-noto/NotoSansCJK-Regular.ttc", 2),
            // 文泉驿
            ("/usr/share/fonts/wqy-microhei/wqy-microhei.ttc", 0),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
            ("/usr/share/fonts/wenquanyi/wqy-zenhei/wqy-zenhei.ttc", 0),
            // 思源黑体
            ("/usr/share/fonts/adobe-source-han-sans/SourceHanSansSC-Regular.otf", 0),
            ("/usr/share/fonts/opentype/source-han-sans/SourceHanSansSC-Regular.otf", 0),
            // 用户级
            // (无法用绝对路径覆盖 ~ 展开 — 留给 env / fontconfig)
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            ("/System/Library/Fonts/PingFang.ttc", 0),
            ("/System/Library/Fonts/STHeiti Medium.ttc", 0),
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            ("/Library/Fonts/Arial Unicode.ttf", 0),
        ]
    }
    #[cfg(target_os = "windows")]
    {
        &[
            (r"C:\Windows\Fonts\msyh.ttc", 0),
            (r"C:\Windows\Fonts\msyh.ttf", 0),
            (r"C:\Windows\Fonts\msyhbd.ttc", 0),
            (r"C:\Windows\Fonts\simhei.ttf", 0),
            (r"C:\Windows\Fonts\simsun.ttc", 0),
        ]
    }
}
