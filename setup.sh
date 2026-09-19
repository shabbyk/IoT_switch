#!/usr/bin/env bash
set -euo pipefail

echo "=== IoT Switch — Pi Setup (Rust binary) ==="

cd "$(dirname "$0")"

# 1. ensure the cross-compiled binary exists
BIN="dist/iot-switch"
if [ ! -x "$BIN" ]; then
  echo "error: $BIN not found. Build it first (./build-pi.sh on your dev machine)." >&2
  exit 1
fi

# 2. install binary + config to /opt/iot-switch
sudo mkdir -p /opt/iot-switch
sudo cp "$BIN" /opt/iot-switch/iot-switch
if [ -f config.json ]; then
  sudo cp config.json /opt/iot-switch/config.json
fi

# 3. /dev/gpiomem is owned by group `gpio`
sudo usermod -a -G gpio "$USER" || true

# 4. systemd service
SERVICE_NAME="iot-switch"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=IoT Switch
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=/opt/iot-switch
ExecStart=/opt/iot-switch/iot-switch
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"
sudo systemctl start "$SERVICE_NAME"

echo "=== Done ==="
echo "Service: sudo systemctl {status,start,stop,restart} $SERVICE_NAME"
echo "Web UI:  http://$(hostname -I | awk '{print $1}'):5000"