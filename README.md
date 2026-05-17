# IoT Switch

A general-purpose programmable smart switch with a web UI. Controls any electrical load (pump, light, motor, sprinkler) via a Raspberry Pi + relay + contactor.

## Quick Start

### 1. Flash Raspberry Pi OS

Download **Raspberry Pi OS Lite** and flash to SD card.

### 2. Pre-configure WiFi + SSH

```bash
touch /media/$USER/boot/ssh

cat > /media/$USER/boot/wpa_supplicant.conf << 'EOF'
country=US
ctrl_interface=DIR=/var/run/wpa_supplicant GROUP=netdev
network={
    ssid="YourWiFiName"
    psk="YourPassword"
}
EOF
```

### 3. Copy project & boot

```bash
cp -r water-pump-switch /media/$USER/rootfs/home/pi/
```

Eject SD, insert into Pi, power on.

### 4. SSH & install

```bash
ssh pi@raspberrypi.local
cd water-pump-switch
sudo bash setup.sh
```

### 5. Set up WiFi fallback hotspot

```bash
bash setup-wifi.sh
```

| Setting | Default |
|---------|---------|
| SSID | `PumpController` |
| Password | `pump1234` |

### 6. Access

| Network | URL |
|---------|-----|
| Home WiFi | `http://<pi-ip>:5000` |
| Hotspot | Connect to `PumpController`, visit `http://192.168.4.1:5000` |

### Commands

```bash
sudo systemctl status water-pump
sudo systemctl restart water-pump
sudo journalctl -u water-pump -f
```

## Hardware Setup

See [HARDWARE.md](HARDWARE.md) for wiring diagrams, DIN rail layout, and shopping list.

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/status` | Current state (on/off, remaining time, schedules) |
| `GET` | `/api/schedules` | List schedules |
| `POST` | `/api/schedules` | Add schedule `{"time":"06:00","duration_minutes":30,"days":[1,2,3,4,5,6,7]}` |
| `DELETE` | `/api/schedules/<id>` | Remove schedule |
| `POST` | `/api/manual` | Override `{"state":true, "duration":15}` |
| `DELETE` | `/api/manual` | Clear override, return to auto |
