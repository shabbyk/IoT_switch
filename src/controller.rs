use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use rppal::gpio::{Gpio, OutputPin};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

const TIME_FMT: &str = "%Y-%m-%dT%H:%M:%S%.6f";
pub const DEFAULT_TICK_SECS: u64 = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: u64,
    pub time: String,
    #[serde(default = "default_duration")]
    pub duration_minutes: u32,
    #[serde(default = "default_days")]
    pub days: Vec<u8>,
}

fn default_duration() -> u32 {
    30
}

fn default_days() -> Vec<u8> {
    (1..=7).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualOverride {
    pub state: bool,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub duration: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_pin")]
    pub relay_gpio: u8,
    #[serde(default)]
    pub schedules: Vec<Schedule>,
    #[serde(default)]
    pub manual_override: Option<ManualOverride>,
    #[serde(default)]
    pub allow_shutdown: bool,
}

fn default_pin() -> u8 {
    17
}

impl Default for Config {
    fn default() -> Self {
        Config {
            relay_gpio: 17,
            schedules: Vec::new(),
            manual_override: None,
            allow_shutdown: false,
        }
    }
}

enum Relay {
    Gpio(OutputPin),
    Simulated,
}

struct Inner {
    config: Config,
    relay: Relay,
    relay_on: bool,
    active_schedule: Option<Schedule>,
    relay_started_at: Option<SystemTime>,
}

pub struct Controller {
    config_path: PathBuf,
    inner: Mutex<Inner>,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub relay_on: bool,
    pub relay_started_at: Option<String>,
    pub relay_remaining_seconds: Option<i64>,
    pub manual_override: Option<ManualOverride>,
    pub active_schedule: Option<Schedule>,
    pub schedules: Vec<Schedule>,
    pub relay_gpio: u8,
    pub allow_shutdown: bool,
    pub gpio_level: Option<u8>,
}

impl Controller {
    pub fn load(config_path: impl Into<PathBuf>) -> anyhow::Result<Arc<Self>> {
        let config_path = config_path.into();
        let config = if config_path.exists() {
            let raw = fs::read_to_string(&config_path)?;
            serde_json::from_str(&raw)?
        } else {
            info!("{} not found — writing default config", config_path.display());
            let cfg = Config::default();
            fs::write(&config_path, serde_json::to_string_pretty(&cfg)?)?;
            cfg
        };

        let relay = setup_relay(config.relay_gpio);
        let ctrl = Arc::new(Controller {
            config_path,
            inner: Mutex::new(Inner {
                config,
                relay,
                relay_on: false,
                active_schedule: None,
                relay_started_at: None,
            }),
        });
        ctrl.tick();
        Ok(ctrl)
    }

    pub fn start_scheduler(self: &Arc<Self>, interval_secs: u64) {
        let ctrl = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                ctrl.tick();
            }
        });
        info!("scheduler started (tick every {}s)", interval_secs);
    }

    // ── public API ────────────────────────────────────────

    pub fn get_schedules(&self) -> Vec<Schedule> {
        self.inner.lock().unwrap().config.schedules.clone()
    }

    pub fn add_schedule(&self, payload: Value) -> Schedule {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_else(|_| rand_fallback_id());
        let schedule = Schedule {
            id,
            time: payload.get("time").and_then(Value::as_str).unwrap_or("").to_string(),
            duration_minutes: payload
                .get("duration_minutes")
                .and_then(Value::as_u64)
                .map(|v| v as u32)
                .unwrap_or_else(default_duration),
            days: payload
                .get("days")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).map(|v| v as u8).collect())
                .unwrap_or_else(default_days),
        };
        let mut inner = self.inner.lock().unwrap();
        inner.config.schedules.push(schedule.clone());
        self.save_config(&inner);
        schedule
    }

    pub fn remove_schedule(&self, schedule_id: u64) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let before = inner.config.schedules.len();
        inner.config.schedules.retain(|s| s.id != schedule_id);
        let removed = inner.config.schedules.len() != before;
        if removed {
            self.save_config(&inner);
        }
        removed
    }

    pub fn set_manual(&self, state: bool, duration_minutes: Option<u32>) {
        let override_ = if state {
            let until = duration_minutes
                .map(|_| Local::now().format(TIME_FMT).to_string());
            Some(ManualOverride {
                state: true,
                until,
                duration: duration_minutes,
            })
        } else {
            Some(ManualOverride {
                state: false,
                until: None,
                duration: None,
            })
        };
        {
            let mut inner = self.inner.lock().unwrap();
            inner.config.manual_override = override_;
            self.save_config(&inner);
        }
        self.tick();
    }

    pub fn clear_manual(&self) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.config.manual_override = None;
            self.save_config(&inner);
        }
        self.tick();
    }

    pub fn status(&self) -> Status {
        let inner = self.inner.lock().unwrap();
        Status {
            relay_on: inner.relay_on,
            relay_started_at: inner.relay_started_at.map(fmt_time),
            relay_remaining_seconds: Self::relay_remaining(&inner),
            manual_override: inner.config.manual_override.clone(),
            active_schedule: inner.active_schedule.clone(),
            schedules: inner.config.schedules.clone(),
            relay_gpio: inner.config.relay_gpio,
            allow_shutdown: inner.config.allow_shutdown,
            gpio_level: Self::gpio_level(&*inner),
        }
    }

    fn gpio_level(inner: &Inner) -> Option<u8> {
        match &inner.relay {
            Relay::Gpio(pin) => Some(if pin.is_set_low() { 0 } else { 1 }),
            Relay::Simulated => None,
        }
    }

    pub fn allow_shutdown(&self) -> bool {
        self.inner.lock().unwrap().config.allow_shutdown
    }

    // ── internals ─────────────────────────────────────────

    fn save_config(&self, inner: &Inner) {
        match serde_json::to_string_pretty(&inner.config) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.config_path, json) {
                    warn!("failed to save config: {e}");
                }
            }
            Err(e) => warn!("failed to serialize config: {e}"),
        }
    }

    fn tick(&self) {
        let mut inner = self.inner.lock().unwrap();
        if Self::tick_inner(&mut inner) {
            self.save_config(&inner);
        }
    }

    /// Applies the current desired relay state. Returns `true` if config was
    /// mutated (i.e., a manual override expired and was cleared).
    fn tick_inner(inner: &mut Inner) -> bool {
        let now = Local::now();

        if let Some(mo) = inner.config.manual_override.clone() {
            if mo.state {
                let mut expired = false;
                if let (Some(dur), Some(until)) = (mo.duration, &mo.until) {
                    if let Some(start) = parse_local_dt(until) {
                        let elapsed_min = (now.naive_local() - start).num_seconds() as f64 / 60.0;
                        if elapsed_min >= dur as f64 {
                            expired = true;
                        }
                    }
                }
                if expired {
                    info!("manual override expired");
                    inner.config.manual_override = None;
                    inner.active_schedule = None;
                } else {
                    apply_relay(inner, true, "manual override");
                    return false;
                }
            } else {
                apply_relay(inner, false, "manually locked OFF");
                inner.active_schedule = None;
                return false;
            }
        }

        let current_min = now.hour() as u16 * 60 + now.minute() as u16;
        let today = now.weekday().num_days_from_monday() as u8 + 1;

        let mut matched: Option<Schedule> = None;
        for s in &inner.config.schedules {
            let Some((h, m)) = parse_time(&s.time) else {
                continue;
            };
            let start_min = h * 60 + m;
            let end_min = start_min + s.duration_minutes as u16;
            if s.days.contains(&today) && start_min <= current_min && current_min < end_min {
                matched = Some(s.clone());
                break;
            }
        }

        if let Some(s) = matched {
            apply_relay(inner, true, &format!("schedule {}", s.time));
            inner.active_schedule = Some(s);
            return false;
        }

        apply_relay(inner, false, "no active schedule");
        inner.active_schedule = None;
        false
    }

    fn relay_remaining(inner: &Inner) -> Option<i64> {
        if !inner.relay_on {
            return None;
        }
        let now = Local::now().naive_local();

        if let Some(mo) = &inner.config.manual_override {
            if mo.state {
                if let (Some(dur), Some(until)) = (mo.duration, &mo.until) {
                    if let Some(start) = parse_local_dt(until) {
                        let secs = dur as i64 * 60 - (now - start).num_seconds();
                        return Some(secs.max(0));
                    }
                }
            }
        }

        if let Some(s) = &inner.active_schedule {
            let (h, m) = parse_time(&s.time)?;
            let mut start = now
                .date()
                .and_hms_opt(h as u32, m as u32, 0)?;
            if now < start {
                start -= chrono::Duration::days(1);
            }
            let end = start + chrono::Duration::minutes(s.duration_minutes as i64);
            let secs = (end - now).num_seconds();
            return Some(secs.max(0));
        }

        None
    }
}

fn parse_time(s: &str) -> Option<(u16, u16)> {
    let (h, m) = s.split_once(':')?;
    Some((h.parse().ok()?, m.parse().ok()?))
}

fn parse_local_dt(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, TIME_FMT).ok()
}

fn fmt_time(t: SystemTime) -> String {
    let dt: chrono::DateTime<Local> = t.into();
    dt.format(TIME_FMT).to_string()
}

fn rand_fallback_id() -> u64 {
    Local::now().timestamp_millis() as u64
}

fn apply_relay(inner: &mut Inner, on: bool, reason: &str) {
    if inner.relay_on != on {
        info!("relay {} ({reason})", if on { "ON" } else { "OFF" });
    }
    inner.relay_on = on;
    inner.relay_started_at = if on { Some(SystemTime::now()) } else { None };

    match &mut inner.relay {
        Relay::Gpio(pin) => {
            if on {
                pin.set_low();
            } else {
                pin.set_high();
            }
        }
        Relay::Simulated => {}
    }
}

fn setup_relay(pin: u8) -> Relay {
    if std::env::var("IOT_SWITCH_SIMULATE").is_ok() {
        warn!("IOT_SWITCH_SIMULATE set — running in simulation mode");
        return Relay::Simulated;
    }
    match Gpio::new() {
        Ok(gpio) => match gpio.get(pin) {
            Ok(candidate) => {
                let mut pin_out = candidate.into_output();
                pin_out.set_high();
                info!("GPIO {pin} ready as relay (active-low)");
                Relay::Gpio(pin_out)
            }
            Err(e) => {
                warn!("failed to access GPIO {pin}: {e} — simulation mode");
                Relay::Simulated
            }
        },
        Err(e) => {
            warn!("no GPIO access ({}: {e}) — simulation mode", std::env::consts::ARCH);
            Relay::Simulated
        }
    }
}