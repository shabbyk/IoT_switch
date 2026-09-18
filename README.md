# IoT Switch

A general-purpose programmable smart switch with a web UI — written in **Rust**. Controls any electrical load (pump, light, motor, sprinkler) via a Raspberry Pi + relay + contactor. Ships as a single ~1.2 MB ARM binary; the web UI is embedded inside it.

- **axum** (HTTP server + API) — same endpoints as before, same `config.json` format
- **rppal** — toggles the relay on GPIO (active-low)
- **tokio** — background scheduler ticks every 15s (schedules + manual override expiry)
- **rust-embed** — HTML/CSS compiled into the binary
- Gates on hardware: without a Pi (or with `IOT_SWITCH_SIMULATE=1`) it runs in simulation mode.

## Local dev (no Pi needed)

```bash
cargo run                 # simulation mode automatically (no /dev/gpiomem)
curl localhost:5000/api/status
```

## Build the Pi binary (cross-compile)

On any machine (aarch64 for 64-bit DietPi on a Pi Zero 2):

```bash
# one-time toolchain
rustup target add aarch64-unknown-linux-gnu
# install zig (https://ziglang.org, any stable) + cargo-zigbuild:
#   curl -L https://ziglang.org/download/.../zig-x86_64-linux-*.tar.xz | tar -xJ
#   cargo install cargo-zigbuild

./build-pi.sh             # → dist/iot-switch (aarch64)
```

For a 32-bit ARMv7 DietPi image:
```bash
TARGET=arm-unknown-linux-gnueabihf ./build-pi.sh
```

The UI and API are compiled in — `dist/iot-switch` is the whole app.

## Deploy to the Pi

1. Flash **Raspberry Pi OS / DietPi (64-bit)** on the Pi Zero 2, enable SSH.
2. Copy the binary + config to the Pi, run setup:

```bash
scp dist/iot-switch config.json pi@raspberrypi.local:~/
ssh pi@raspberrypi.local "sudo mkdir -p /opt/iot-switch && sudo cp iot-switch config.json /opt/iot-switch/"
# then run setup.sh from a clone of this repo on the Pi, or skip straight to the service:
```

### Commands

```bash
sudo systemctl status iot-switch
sudo systemctl restart iot-switch
sudo journalctl -u iot-switch -f
```

`config.json` is auto-created if missing:
```json
{
  "relay_gpio": 17,
  "schedules": [],
  "manual_override": null
}
```

## WiFi fallback hotspot

```bash
bash setup-wifi.sh   # SSID PumpController / pump1234, auto-starts when home WiFi is down
```

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/status` | Current state (on/off, remaining time, schedules) |
| `GET` | `/api/schedules` | List schedules |
| `POST` | `/api/schedules` | Add schedule `{"time":"06:00","duration_minutes":30,"days":[1,2,3,4,5,6,7]}` |
| `DELETE` | `/api/schedules/<id>` | Remove schedule |
| `POST` | `/api/manual` | Override `{"state":true, "duration":15}` |
| `DELETE` | `/api/manual` | Clear override, return to auto |

## Hardware Setup

See [HARDWARE.md](HARDWARE.md) for wiring diagrams, DIN rail layout, and shopping list.