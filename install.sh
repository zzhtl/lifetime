#!/bin/sh
# Lifetime 一键安装脚本（Linux x86_64）
#
#   curl -fsSL https://raw.githubusercontent.com/zzhtl/lifetime/main/install.sh | sh
#
# 可用环境变量：
#   LIFETIME_VERSION      指定版本 tag（如 v0.1.0），默认最新 release
#   LIFETIME_INSTALL_DIR  安装目录，默认 ~/.local/bin
set -eu

REPO="zzhtl/lifetime"
BIN="lifetime"
TARGET="x86_64-unknown-linux-gnu"
INSTALL_DIR="${LIFETIME_INSTALL_DIR:-$HOME/.local/bin}"

err() { printf '错误: %s\n' "$1" >&2; exit 1; }

# ---- 平台检查 --------------------------------------------------------------
os=$(uname -s)
arch=$(uname -m)
[ "$os" = "Linux" ] || err "此脚本仅支持 Linux（当前: $os）。Windows 请用 install.ps1，macOS 请从源码构建：cargo install --git https://github.com/$REPO"
[ "$arch" = "x86_64" ] || err "暂无 $arch 预编译包（仅 x86_64）。请从源码构建：cargo install --git https://github.com/$REPO"

command -v curl >/dev/null 2>&1 || err "需要 curl，请先安装"

# ---- 解析版本 --------------------------------------------------------------
if [ -n "${LIFETIME_VERSION:-}" ]; then
    tag="$LIFETIME_VERSION"
else
    # 跟随 releases/latest 的重定向拿最新 tag，避免 GitHub API 限流
    location=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") \
        || err "获取最新版本失败，请检查网络或稍后重试"
    tag="${location##*/}"
    case "$tag" in
        v*) ;;
        *) err "解析最新版本失败（得到: $tag），可用 LIFETIME_VERSION=v0.1.0 指定版本重试" ;;
    esac
fi
printf '安装 %s %s ...\n' "$BIN" "$tag"

# ---- 下载并校验 ------------------------------------------------------------
archive="$BIN-$tag-$TARGET.tar.gz"
base_url="https://github.com/$REPO/releases/download/$tag"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

curl -fsSL -o "$tmp/$archive" "$base_url/$archive" \
    || err "下载失败: $base_url/$archive"

if command -v sha256sum >/dev/null 2>&1; then
    if curl -fsSL -o "$tmp/sha256sums.txt" "$base_url/sha256sums.txt"; then
        (cd "$tmp" && grep " $archive\$" sha256sums.txt | sha256sum -c - >/dev/null) \
            || err "sha256 校验失败，下载可能不完整或被篡改"
        printf 'sha256 校验通过\n'
    else
        printf '警告: 未获取到 sha256sums.txt，跳过校验\n' >&2
    fi
fi

# ---- 安装 ------------------------------------------------------------------
tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"
printf '已安装到 %s/%s\n' "$INSTALL_DIR" "$BIN"

# ---- 运行时依赖与 PATH 提示 ------------------------------------------------
if command -v ldconfig >/dev/null 2>&1 && ! ldconfig -p 2>/dev/null | grep -q libasound.so.2; then
    printf '提示: 未检测到 ALSA 运行库，音效提醒需要它:\n'
    printf '  sudo apt install libasound2t64   # Ubuntu 24.04+\n'
    printf '  sudo apt install libasound2      # Ubuntu 22.04 / Debian 12\n'
fi

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) printf '提示: %s 不在 PATH 中，可执行:\n  echo '\''export PATH="%s:$PATH"'\'' >> ~/.bashrc && . ~/.bashrc\n' "$INSTALL_DIR" "$INSTALL_DIR" ;;
esac

printf '完成。首次运行 `%s` 会自动创建应用菜单快捷方式。\n' "$BIN"
