import json
import threading
import time
import logging
from datetime import datetime, timedelta
from pathlib import Path

logging.basicConfig(level=logging.INFO)
log = logging.getLogger(__name__)

CONFIG_PATH = Path(__file__).parent / "config.json"

try:
    import RPi.GPIO as GPIO
    GPIO_AVAILABLE = True
except (ImportError, RuntimeError):
    GPIO_AVAILABLE = False
    log.warning("RPi.GPIO not available — running in simulation mode")


class PumpController:
    def __init__(self, config_path=CONFIG_PATH):
        self.config_path = Path(config_path)
        self.gpio = None
        self.pump_on = False
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._thread = None
        self._active_schedule = None
        self._pump_started_at = None
        self._listeners = []

        self._load_config()
        self._setup_gpio()
        self._start_scheduler()

    # ── config ──────────────────────────────────────────────

    def _load_config(self):
        with open(self.config_path) as f:
            self.config = json.load(f)

    def _save_config(self):
        with open(self.config_path, "w") as f:
            json.dump(self.config, f, indent=2)

    def get_config(self):
        return dict(self.config)

    def update_config(self, data):
        with self._lock:
            self.config.update(data)
            self._save_config()

    # ── schedules ───────────────────────────────────────────

    def get_schedules(self):
        return list(self.config.get("schedules", []))

    def add_schedule(self, schedule):
        with self._lock:
            schedules = self.config.setdefault("schedules", [])
            schedule["id"] = int(time.time() * 1000)
            schedules.append(schedule)
            self._save_config()
        return schedule

    def remove_schedule(self, schedule_id):
        with self._lock:
            before = len(self.config["schedules"])
            self.config["schedules"] = [
                s for s in self.config["schedules"] if s["id"] != schedule_id
            ]
            self._save_config()
        return before != len(self.config["schedules"])

    # ── manual override ─────────────────────────────────────

    def set_manual(self, state, duration_minutes=None):
        with self._lock:
            if state:
                until = (
                    datetime.now().isoformat()
                    if duration_minutes is not None
                    else None
                )
                self.config["manual_override"] = {
                    "state": True,
                    "until": until,
                    "duration": duration_minutes,
                }
            else:
                self.config["manual_override"] = {"state": False}
            self._save_config()
            self._apply_state()

    def clear_manual(self):
        with self._lock:
            self.config["manual_override"] = None
            self._save_config()
            self._apply_state()

    # ── GPIO ────────────────────────────────────────────────

    def _setup_gpio(self):
        if not GPIO_AVAILABLE:
            return
        GPIO.setwarnings(False)
        GPIO.setmode(GPIO.BCM)
        pin = self.config.get("relay_gpio", 17)
        GPIO.setup(pin, GPIO.OUT)
        GPIO.output(pin, GPIO.HIGH)
        self.gpio = pin

    def _set_relay(self, on):
        self.pump_on = on
        self._pump_started_at = datetime.now() if on else None
        log.info("PUMP %s", "ON" if on else "OFF")
        if GPIO_AVAILABLE and self.gpio is not None:
            GPIO.output(self.gpio, GPIO.LOW if on else GPIO.HIGH)
        for cb in self._listeners:
            try:
                cb(on)
            except Exception:
                log.exception("listener error")

    # ── scheduling loop ─────────────────────────────────────

    def _start_scheduler(self):
        self._stop_event.clear()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    def _run_loop(self):
        while not self._stop_event.is_set():
            try:
                self._tick()
            except Exception:
                log.exception("scheduler tick error")
            time.sleep(15)

    def _tick(self):
        override = self.config.get("manual_override")
        if override and override.get("state"):
            duration = override.get("duration")
            if duration and override.get("until"):
                start = datetime.fromisoformat(override["until"])
                elapsed = (datetime.now() - start).total_seconds() / 60
                if elapsed >= duration:
                    log.info("manual override expired")
                    self.config["manual_override"] = None
                    self._save_config()
                    self._apply_state()
                    return
            self._set_relay(True)
            return

        if override and not override.get("state"):
            self._set_relay(False)
            return

        now = datetime.now()
        current_min = now.hour * 60 + now.minute
        today = now.isoweekday()

        for s in self.config.get("schedules", []):
            h, m = s["time"].split(":")
            start_min = int(h) * 60 + int(m)
            end_min = start_min + s.get("duration_minutes", 30)
            days = s.get("days", [1, 2, 3, 4, 5, 6, 7])

            if today in days and start_min <= current_min < end_min:
                self._set_relay(True)
                self._active_schedule = s
                return

        self._set_relay(False)
        self._active_schedule = None

    def _apply_state(self):
        self._tick()

    def stop(self):
        self._stop_event.set()
        if self._thread:
            self._thread.join(timeout=5)
        if GPIO_AVAILABLE:
            GPIO.cleanup()

    def subscribe(self, callback):
        self._listeners.append(callback)
        return lambda: self._listeners.remove(callback)

    def _pump_remaining(self):
        if not self.pump_on:
            return None
        override = self.config.get("manual_override")
        if override and override.get("state") and override.get("duration"):
            start = datetime.fromisoformat(override["until"])
            secs = int(override["duration"] * 60 - (datetime.now() - start).total_seconds())
            return max(0, secs)
        if self._active_schedule:
            now = datetime.now()
            h, m = self._active_schedule["time"].split(":")
            sched_start = now.replace(hour=int(h), minute=int(m), second=0, microsecond=0)
            if now < sched_start:
                sched_start -= timedelta(days=1)
            end = sched_start + timedelta(minutes=self._active_schedule["duration_minutes"])
            secs = int((end - now).total_seconds())
            return max(0, secs)
        return None

    @property
    def status(self):
        override = self.config.get("manual_override")
        return {
            "pump_on": self.pump_on,
            "pump_started_at": self._pump_started_at.isoformat() if self._pump_started_at else None,
            "pump_remaining_seconds": self._pump_remaining(),
            "manual_override": override,
            "active_schedule": self._active_schedule,
            "schedules": self.get_schedules(),
            "relay_gpio": self.config.get("relay_gpio"),
        }
