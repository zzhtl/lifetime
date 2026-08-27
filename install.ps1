# Lifetime 一键安装脚本（Windows x86_64）
#
#   powershell -ExecutionPolicy Bypass -c "irm https://raw.githubusercontent.com/zzhtl/lifetime/main/install.ps1 | iex"
#
# 可用环境变量：
#   LIFETIME_VERSION      指定版本 tag（如 v0.1.0），默认最新 release
#   LIFETIME_INSTALL_DIR  安装目录，默认 %LOCALAPPDATA%\Programs\Lifetime

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 默认不启用 TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = 'zzhtl/lifetime'
$Bin = 'lifetime'
$Target = 'x86_64-pc-windows-msvc'
$InstallDir = if ($env:LIFETIME_INSTALL_DIR) { $env:LIFETIME_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\Lifetime' }

if (-not [Environment]::Is64BitOperatingSystem) {
    throw '暂无 32 位预编译包，请从源码构建'
}

# ---- 解析版本 --------------------------------------------------------------
if ($env:LIFETIME_VERSION) {
    $Tag = $env:LIFETIME_VERSION
} else {
    $Tag = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name
}
Write-Host "安装 $Bin $Tag ..."

# ---- 下载解压 --------------------------------------------------------------
$Archive = "$Bin-$Tag-$Target.zip"
$Tmp = Join-Path ([IO.Path]::GetTempPath()) "lifetime-install-$([IO.Path]::GetRandomFileName())"
New-Item -ItemType Directory -Path $Tmp | Out-Null
try {
    Invoke-WebRequest "https://github.com/$Repo/releases/download/$Tag/$Archive" -OutFile (Join-Path $Tmp $Archive)
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -Path (Join-Path $Tmp $Archive) -DestinationPath $InstallDir -Force
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
Write-Host "已安装到 $InstallDir\$Bin.exe"

# ---- PATH ------------------------------------------------------------------
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($UserPath -split ';') -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable('Path', "$UserPath;$InstallDir", 'User')
    Write-Host "已将 $InstallDir 加入用户 PATH（新开终端生效）"
}

Write-Host "完成。首次运行 $Bin 会自动创建开始菜单快捷方式。"
