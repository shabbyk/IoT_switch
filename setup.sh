#!/usr/bin/env bash
set -euo pipefail

echo "=== Water Pump Controller — Pi Setup ==="

# 1. update system
sudo apt-get update && sudo apt-get upgrade -y

# 2. install python & pip
sudo apt-get install -y python3 python3-pip python3-venv

# 3. create venv & install deps
cd "$(dirname "$0")"
python3 -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt

# 4. enable GPIO access for non-root
sudo usermod -a -G gpio "$USER" || true

# 5. create systemd service
SERVICE_NAME="water-pump"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=Water Pump Controller
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/venv/bin/python app.py
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
