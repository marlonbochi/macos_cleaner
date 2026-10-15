#!/usr/bin/env bash
# System libraries Tauri needs to build on Debian/Ubuntu (WebKitGTK and friends).
# https://v2.tauri.app/start/prerequisites/#linux
set -euo pipefail
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
