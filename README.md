# IoT Switch

A general-purpose programmable smart switch with a web UI — written in **Rust**. Controls any electrical load (pump, light, motor, sprinkler) via a Raspberry Pi + relay + contactor. Ships as a single ~1.2 MB ARM binary; the web UI is embedded inside it.

- **axum** (HTTP server + API) — same endpoints as before, same `config.json` format
- **rppal** — toggles the relay on GPIO (active-low)
- **tokio** — background scheduler ticks every 15s (schedules + manual override expiry)
- **rust-embed** — HTML/CSS compiled into the binary
- Gates on hardware: without a Pi (or with `IOT_SWITCH_SIMULATE=1`) it runs in simulation mode.

## Run locally (on your dev machine, no Pi needed)

The app auto-detects hardware: if there's no Raspberry Pi GPIO, it starts in **simulation mode**.

```bash
cargo run                      # build + start
# → http://localhost:5000
curl localhost:5000/api/status # or open the web UI in a browser
```

To force simulation mode explicitly:

```bash
IOT_SWITCH_SIMULATE=1 cargo run
```

Stop it with `Ctrl+C`. State persists to `config.json` in the repo, and a fresh `config.json`
is created automatically if it doesn't exist.

## Build the Pi binary (cross-compile)

Builds a single aarch64 ARM binary on any machine — you never need to compile on the Pi.

One-time toolchain setup:

```bash
# Raspberry Pi 64-bit target (DietPi 64-bit, Pi Zero 2 / Pi 3+)
rustup target add aarch64-unknown-linux-gnu

# zig — any stable version, put it on PATH
curl -L https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz | tar -xJ
# then add zig-x86_64-linux-0.14.1/zig to your PATH (e.g. symlink into ~/.local/bin)

# cross-compile wrapper
cargo install cargo-zigbuild
```

Now build:

```bash
./build-pi.sh                  # → dist/iot-switch (aarch64, ~1.2 MB)
```

The web UI and API are compiled **into** the binary — `dist/iot-switch` is the entire app.

For a 32-bit ARMv7 DietPi image (older Pis):

```bash
TARGET=arm-unknown-linux-gnueabihf ./build-pi.sh
```

## Move it to the Pi

```bash
# copy from your dev machine to the Pi (enable SSH first)
scp dist/iot-switch config.json pi@raspberrypi.local:~/

# SSH in and install
ssh pi@raspberrypi.local
sudo mkdir -p /opt/iot-switch
sudo cp iot-switch config.json /opt/iot-switch/
```

### Run it as a service (recommended)

Simplest: install `setup.sh` from a clone of this repo on the Pi, then:

```bash
git clone <this-repo> ~/iot-switch && cd ~/iot-switch
sudo bash setup.sh             # installs to /opt/iot-switch + registers iot-switch.service
```

Or create the service by hand:

```bash
sudo tee /etc/systemd/system/iot-switch.service > /dev/null <<'EOF'
[Unit]
Description=IoT Switch
After=network.target

[Service]
Type=simple
User=pi
WorkingDirectory=/opt/iot-switch
ExecStart=/opt/iot-switch/iot-switch
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now iot-switch
```

### Verify

```bash
sudo systemctl status iot-switch   # running?
curl http://<pi-ip>:5000/api/status
# open http://<pi-ip>:5000 in a browser
```

`config.json` is auto-created if missing:
```json
{
  "relay_gpio": 17,
  "schedules": [],
  "manual_override": null,
  "allow_shutdown": false
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
| `POST` | `/api/system/shutdown` | Power off the Pi (403 unless config has `"allow_shutdown": true`) |

## Hardware Setup

See [HARDWARE.md](HARDWARE.md) for wiring diagrams, DIN rail layout, and shopping list.