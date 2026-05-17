#!/usr/bin/env bash
set -euo pipefail

echo "=== Water Pump — WiFi Setup ==="
echo "This will set up:"
echo "  1. Home WiFi connection (auto-connect when available)"
echo "  2. Fallback hotspot (auto-starts when home WiFi is unreachable)"
echo ""

# ── get home WiFi credentials ──
read -rp "Home WiFi SSID: " HOME_SSID
read -rsp "Home WiFi password: " HOME_PSK
echo ""

# ── set hotspot credentials ──
HOTSPOT_SSID="PumpController"
HOTSPOT_PSK="pump1234"

# ── configure via NetworkManager ──
echo "Configuring home WiFi..."
nmcli connection delete "$HOME_SSID" 2>/dev/null || true
nmcli dev wifi connect "$HOME_SSID" password "$HOME_PSK" \
  connection.autoconnect-priority 10 \
  connection.autoconnect yes

echo "Configuring fallback hotspot..."
nmcli connection delete "$HOTSPOT_SSID" 2>/dev/null || true
nmcli connection add type wifi ifname wlan0 con-name "$HOTSPOT_SSID" \
  ssid "$HOTSPOT_SSID" \
  connection.autoconnect-priority 5 \
  connection.autoconnect yes \
  802-11-wireless.mode ap \
  802-11-wireless.band bg \
  ipv4.method shared \
  wifi-sec.key-mgmt wpa-psk \
  wifi-sec.psk "$HOTSPOT_PSK"

# ── install fallback monitor service ──
SERVICE_NAME="pump-wifi-fallback"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

sudo tee "$SERVICE_FILE" > /dev/null <<'EOF'
[Unit]
Description=Pump WiFi Fallback Monitor
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/pump-wifi-check.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

TIMER_FILE="/etc/systemd/system/${SERVICE_NAME}.timer"
sudo tee "$TIMER_FILE" > /dev/null <<'EOF'
[Unit]
Description=Check WiFi every 2 minutes

[Timer]
OnBootSec=30
OnUnitActiveSec=120

[Install]
WantedBy=timers.target
EOF

sudo tee /usr/local/bin/pump-wifi-check.sh > /dev/null <<'CHECKEOF'
#!/usr/bin/env bash
HOTSPOT_SSID="PumpController"
HOME_CONNECTED=$(nmcli -t -f DEVICE,STATE con show --active 2>/dev/null | grep "^wlan0:" | cut -d: -f2 || echo "")

if [ "$HOME_CONNECTED" != "activated" ]; then
    # try to connect to home network
    nmcli connection up "$(nmcli -t -f NAME,DEVICE con show | grep ":wlan0$" | head -1 | cut -d: -f1)" 2>/dev/null || true
    sleep 5
    HOME_CONNECTED=$(nmcli -t -f DEVICE,STATE con show --active 2>/dev/null | grep "^wlan0:" | cut -d: -f2 || echo "")
fi

if [ "$HOME_CONNECTED" != "activated" ]; then
    # fallback: start hotspot
    nmcli connection up "$HOTSPOT_SSID" 2>/dev/null || true
fi
CHECKEOF
sudo chmod +x /usr/local/bin/pump-wifi-check.sh

sudo systemctl daemon-reload
sudo systemctl enable "${SERVICE_NAME}.timer"
sudo systemctl start "${SERVICE_NAME}.timer"

echo ""
echo "=== Done ==="
echo "Home WiFi:  $HOME_SSID"
echo "Hotspot:    $HOTSPOT_SSID / $HOTSPOT_PSK"
echo "Web UI:     http://$(hostname -I | awk '{print $1}'):5000"
echo ""
echo "The Pi will try home WiFi first. If unreachable,"
echo "it starts its own hotspot after ~30 seconds."
echo "Connect to '$HOTSPOT_SSID' and visit http://192.168.4.1:5000"
