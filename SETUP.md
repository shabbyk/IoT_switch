# Water Pump Switch — Quick Setup

## 1. Flash Raspberry Pi OS

Download **Raspberry Pi OS Lite** (no desktop needed) from raspberrypi.com and flash to SD card using Raspberry Pi Imager.

## 2. Pre-configure WiFi + SSH (before first boot)

After flashing, the SD card's **boot** partition will be accessible. Run:

```bash
# Paths vary — on Ubuntu/Debian:
#   /media/$USER/bootfs/  or  /media/$USER/boot/

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

## 3. Copy project to SD card

Mount the **rootfs** partition and copy the project:

```bash
cp -r water-pump-switch /media/$USER/rootfs/home/pi/
```

Safely eject the SD card, insert into Pi, and power on.

## 4. SSH in & install

```bash
ssh pi@raspberrypi.local
# default password: raspberry

cd water-pump-switch
sudo bash setup.sh
```

This creates a virtualenv, installs Flask + RPi.GPIO, and sets up a systemd service that auto-starts on boot.

## 5. Set up WiFi fallback hotspot

```bash
bash setup-wifi.sh
```

You'll be prompted for your home WiFi credentials. The hotspot credentials default to:

| Setting | Value |
|---------|-------|
| SSID | `PumpController` |
| Password | `pump1234` |

## 6. Access the web UI

| Network | URL |
|---------|-----|
| Home WiFi | `http://<pi-ip>:5000` |
| Fallback hotspot | Connect to `PumpController`, then visit `http://192.168.4.1:5000` |

## Commands

```bash
sudo systemctl status water-pump    # check status
sudo systemctl restart water-pump   # restart after code changes
sudo journalctl -u water-pump -f    # live logs
```

## Updating code

```bash
# Edit files locally, then scp to Pi:
scp app.py pi@<pi-ip>:water-pump-switch/
# Or pull from git if you set that up
# Then restart:
sudo systemctl restart water-pump
```
