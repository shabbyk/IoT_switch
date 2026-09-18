#!/usr/bin/env bash
set -euo pipefail

# Cross-compile iot-switch for the Raspberry Pi from any dev machine.
#
# Prerequisites (one-time):
#   rustup target add aarch64-unknown-linux-gnu
#   zig  — download from https://ziglang.org, any stable version, put `zig` on PATH
#   cargo install cargo-zigbuild
#
# 64-bit DietPi (Pi Zero 2, Pi 3/4/5) → aarch64-unknown-linux-gnu (default)
# 32-bit ARMv7 DietPi (older Pis)     → TARGET=arm-unknown-linux-gnueabihf ./build-pi.sh

TARGET="${TARGET:-aarch64-unknown-linux-gnu}"

rustup target add "$TARGET"
cargo zigbuild --release --target "$TARGET"

mkdir -p dist
cp "target/$TARGET/release/iot-switch" dist/iot-switch

echo ""
echo "Built: dist/iot-switch ($(file -b dist/iot-switch | cut -d, -f1-2))"
echo "Copy it to the Pi together with config.json, then run: sudo bash setup.sh"