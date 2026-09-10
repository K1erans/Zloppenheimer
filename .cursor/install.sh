#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# script/linux installs system libraries with apt-get. Configuring fuse3 shows an
# interactive conffile prompt for /etc/fuse.conf that hangs an unattended run, so
# force apt to keep existing config files and never prompt.
export DEBIAN_FRONTEND=noninteractive
sudo mkdir -p /etc/apt/apt.conf.d
sudo tee /etc/apt/apt.conf.d/90cursor-noninteractive >/dev/null <<'APT'
Dpkg::Options { "--force-confold"; "--force-confdef"; };
APT

# System libraries required to build Zed (Wayland/X11, Vulkan, fontconfig, etc.).
script/linux

# mesa-vulkan-drivers provides the llvmpipe/lavapipe software Vulkan device and
# Xvfb provides a virtual display, so the GUI can be exercised on a headless VM.
sudo -E apt-get install -y xvfb mesa-vulkan-drivers vulkan-tools

# Extra compilation targets pinned by rust-toolchain.toml (installing the active
# toolchain first if the base image does not already have it).
rustup show active-toolchain >/dev/null 2>&1 || rustup toolchain install
rustup target add wasm32-wasip2 wasm32-unknown-unknown x86_64-unknown-linux-musl

# Warm the crate/git dependency cache so the first build is faster.
cargo fetch --locked
