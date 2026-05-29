// 桌面快捷方式 —— 运行时自动在应用菜单生成启动项
//
// 逻辑：取当前可执行文件路径，与已存在快捷方式中记录的路径比较。
// 路径不变则跳过（不修改），路径变化或快捷方式缺失则覆盖写入。
// 跨平台落点：
//   Linux   -> ~/.local/share/applications/lifetime.desktop
//   Windows -> %APPDATA%\Microsoft\Windows\Start Menu\Programs\Lifetime 健康助手.lnk
//   macOS   -> ~/Applications/Lifetime.app
//
// 任何失败只记日志，绝不影响程序启动。

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const APP_NAME: &str = "Lifetime 健康助手";
const APP_COMMENT: &str = "程序员/久坐人群的科学健康提醒工具";

/// 入口：尽力创建/更新桌面快捷方式。
pub fn ensure(data_dir: &Path) {
    if let Err(e) = run(data_dir) {
        log::warn!("生成桌面快捷方式失败: {e:#}");
    }
}

fn run(data_dir: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("获取可执行文件路径失败")?;
    // 规范化以便与已记录路径稳定比较；失败则退回原始路径
    let exe = exe.canonicalize().unwrap_or(exe);
    platform::ensure(&exe, data_dir)
}

/// 把内嵌资源写到数据目录（仅在缺失时写）。返回目标绝对路径。
#[allow(dead_code)]
fn write_if_missing(data_dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf> {
    let path = data_dir.join(name);
    if !path.exists() {
        std::fs::write(&path, bytes).with_context(|| format!("写入资源失败: {path:?}"))?;
    }
    Ok(path)
}

// ---------------------------------------------------------------- Linux

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

    pub fn ensure(exe: &Path, data_dir: &Path) -> Result<()> {
        let icon = write_if_missing(data_dir, "icon.png", ICON_PNG)?;

        let dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("无法定位 ~/.local/share"))?
            .join("applications");
        std::fs::create_dir_all(&dir).with_context(|| format!("创建目录失败: {dir:?}"))?;
        let file = dir.join("lifetime.desktop");

        let exec = exe.display().to_string();
        if let Ok(existing) = std::fs::read_to_string(&file) {
            if exec_matches(&existing, &exec) {
                return Ok(()); // 运行路径未变，保持不动
            }
        }

        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={APP_NAME}\n\
             Comment={APP_COMMENT}\n\
             Exec=\"{exec}\"\n\
             Icon={icon}\n\
             Terminal=false\n\
             Categories=Utility;\n\
             StartupWMClass=Lifetime\n",
            icon = icon.display(),
        );
        std::fs::write(&file, content).with_context(|| format!("写入 .desktop 失败: {file:?}"))?;
        log::info!("已更新桌面快捷方式: {file:?}");
        Ok(())
    }

    /// 解析 .desktop 里的 Exec= 行，判断是否指向同一可执行文件。
    fn exec_matches(content: &str, exec: &str) -> bool {
        content.lines().any(|line| {
            line.strip_prefix("Exec=")
                .map(|v| v.trim().trim_matches('"') == exec)
                .unwrap_or(false)
        })
    }
}

// ---------------------------------------------------------------- Windows

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::process::Command;

    const ICON_ICO: &[u8] = include_bytes!("../assets/icon.ico");

    pub fn ensure(exe: &Path, data_dir: &Path) -> Result<()> {
        let icon = write_if_missing(data_dir, "icon.ico", ICON_ICO)?;

        let dir = dirs::data_dir() // %APPDATA%\Roaming
            .ok_or_else(|| anyhow::anyhow!("无法定位 APPDATA"))?
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        std::fs::create_dir_all(&dir).with_context(|| format!("创建目录失败: {dir:?}"))?;
        let lnk = dir.join(format!("{APP_NAME}.lnk"));

        let exec = exe.display().to_string();
        if let Some(existing) = read_target(&lnk) {
            if existing.trim().eq_ignore_ascii_case(exec.trim()) {
                return Ok(()); // 运行路径未变，保持不动
            }
        }

        create_lnk(&lnk, &exec, &icon.display().to_string())?;
        log::info!("已更新开始菜单快捷方式: {lnk:?}");
        Ok(())
    }

    /// 通过 WScript.Shell 读取已存在 .lnk 的 TargetPath。
    fn read_target(lnk: &Path) -> Option<String> {
        if !lnk.exists() {
            return None;
        }
        let script = format!(
            "$s=(New-Object -COM WScript.Shell).CreateShortcut('{}'); $s.TargetPath",
            lnk.display()
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .ok()?;
        let target = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (!target.is_empty()).then_some(target)
    }

    fn create_lnk(lnk: &Path, exec: &str, icon: &str) -> Result<()> {
        let workdir = Path::new(exec)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let script = format!(
            "$s=(New-Object -COM WScript.Shell).CreateShortcut('{lnk}'); \
             $s.TargetPath='{exec}'; \
             $s.WorkingDirectory='{workdir}'; \
             $s.IconLocation='{icon}'; \
             $s.Description='{desc}'; \
             $s.Save()",
            lnk = lnk.display(),
            desc = APP_COMMENT,
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .context("调用 PowerShell 创建快捷方式失败")?;
        anyhow::ensure!(status.success(), "PowerShell 创建快捷方式返回非零状态");
        Ok(())
    }
}

// ---------------------------------------------------------------- macOS

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

    pub fn ensure(exe: &Path, data_dir: &Path) -> Result<()> {
        let png = write_if_missing(data_dir, "icon.png", ICON_PNG)?;

        let app = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("无法定位 home 目录"))?
            .join("Applications")
            .join("Lifetime.app");
        let macos_dir = app.join("Contents/MacOS");
        let res_dir = app.join("Contents/Resources");
        let launcher = macos_dir.join("Lifetime");

        let exec = exe.display().to_string();
        if let Ok(existing) = std::fs::read_to_string(&launcher) {
            if existing.contains(&exec) {
                return Ok(()); // 运行路径未变，保持不动
            }
        }

        std::fs::create_dir_all(&macos_dir).with_context(|| format!("创建目录失败: {macos_dir:?}"))?;
        std::fs::create_dir_all(&res_dir)?;

        // 启动器脚本转发到真实二进制
        let script = format!("#!/bin/sh\nexec \"{exec}\" \"$@\"\n");
        std::fs::write(&launcher, script).context("写入启动器脚本失败")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))?;
        }

        // 尽力把 PNG 转成 .icns（需要系统自带的 sips / iconutil），失败则无图标
        let icon_file = make_icns(&png, &res_dir).ok();
        let icon_plist = icon_file
            .as_ref()
            .map(|_| "    <key>CFBundleIconFile</key>\n    <string>icon</string>\n")
            .unwrap_or("");

        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n<dict>\n\
             \x20   <key>CFBundleName</key>\n    <string>Lifetime</string>\n\
             \x20   <key>CFBundleDisplayName</key>\n    <string>{APP_NAME}</string>\n\
             \x20   <key>CFBundleExecutable</key>\n    <string>Lifetime</string>\n\
             \x20   <key>CFBundleIdentifier</key>\n    <string>dev.zzhtl.lifetime</string>\n\
             \x20   <key>CFBundlePackageType</key>\n    <string>APPL</string>\n\
             {icon_plist}\
             </dict>\n</plist>\n",
        );
        std::fs::write(app.join("Contents/Info.plist"), plist).context("写入 Info.plist 失败")?;
        log::info!("已更新启动器: {app:?}");
        Ok(())
    }

    /// 借助 sips + iconutil 把 PNG 转 icns；任一缺失即返回 Err。
    fn make_icns(png: &Path, res_dir: &Path) -> Result<PathBuf> {
        use std::process::Command;
        let iconset = res_dir.join("icon.iconset");
        std::fs::create_dir_all(&iconset)?;
        for size in [16, 32, 128, 256, 512] {
            let out = iconset.join(format!("icon_{size}x{size}.png"));
            let status = Command::new("sips")
                .args(["-z", &size.to_string(), &size.to_string()])
                .arg(png)
                .arg("--out")
                .arg(&out)
                .status()
                .context("sips 调用失败")?;
            anyhow::ensure!(status.success(), "sips 缩放失败");
        }
        let icns = res_dir.join("icon.icns");
        let status = Command::new("iconutil")
            .args(["-c", "icns"])
            .arg(&iconset)
            .arg("-o")
            .arg(&icns)
            .status()
            .context("iconutil 调用失败")?;
        anyhow::ensure!(status.success(), "iconutil 转换失败");
        let _ = std::fs::remove_dir_all(&iconset);
        Ok(icns)
    }
}
