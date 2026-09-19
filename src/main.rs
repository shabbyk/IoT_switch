mod controller;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use controller::{Controller, Schedule, Status};
use rust_embed::RustEmbed;
use serde_json::{json, Value};
use tracing::{info, warn};
use axum::http::StatusCode;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct Templates;

#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let ctrl = Controller::load("config.json")?;
    ctrl.start_scheduler(controller::DEFAULT_TICK_SECS);

    let app = Router::new()
        .route("/", get(index))
        .route("/static/style.css", get(style_css))
        .route("/api/status", get(api_status))
        .route(
            "/api/schedules",
            get(api_get_schedules).post(api_add_schedule),
        )
        .route("/api/schedules/:id", delete(api_remove_schedule))
        .route("/api/manual", post(api_manual).delete(api_clear_manual))
        .route("/api/system/shutdown", post(api_shutdown))
        .with_state(ctrl);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await?;
    info!("listening on http://0.0.0.0:5000");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── static / embedded UI ─────────────────────────────────

fn asset_response(asset: Option<rust_embed::EmbeddedFile>, mime: &'static str) -> Response {
    match asset {
        Some(file) => ([(header::CONTENT_TYPE, mime)], file.data.into_owned()).into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn index() -> Response {
    asset_response(Templates::get("index.html"), "text/html; charset=utf-8")
}

async fn style_css() -> Response {
    asset_response(StaticAssets::get("style.css"), "text/css; charset=utf-8")
}

// ── API ──────────────────────────────────────────────────

async fn api_status(State(ctrl): State<Arc<Controller>>) -> Json<Status> {
    Json(ctrl.status())
}

async fn api_get_schedules(State(ctrl): State<Arc<Controller>>) -> Json<Vec<Schedule>> {
    Json(ctrl.get_schedules())
}

async fn api_add_schedule(
    State(ctrl): State<Arc<Controller>>,
    payload: Json<Value>,
) -> Response {
    if payload.get("time").and_then(Value::as_str).is_none() {
        warn!("rejected schedule without time");
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "time required"}))).into_response();
    }
    ctrl.add_schedule(payload.0);
    (StatusCode::CREATED, Json(ctrl.get_schedules())).into_response()
}

async fn api_remove_schedule(
    State(ctrl): State<Arc<Controller>>,
    Path(schedule_id): Path<u64>,
) -> Response {
    let ok = ctrl.remove_schedule(schedule_id);
    if ok {
        (StatusCode::OK, Json(json!({"deleted": true}))).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({"deleted": false}))).into_response()
    }
}

async fn api_manual(State(ctrl): State<Arc<Controller>>, payload: Json<Value>) -> Json<Status> {
    let state = payload.get("state").and_then(Value::as_bool).unwrap_or(false);
    let duration = payload.get("duration").and_then(Value::as_u64).map(|v| v as u32);
    ctrl.set_manual(state, duration);
    Json(ctrl.status())
}

async fn api_clear_manual(State(ctrl): State<Arc<Controller>>) -> Json<Status> {
    ctrl.clear_manual();
    Json(ctrl.status())
}

// ── system ───────────────────────────────────────────────

async fn api_shutdown(State(ctrl): State<Arc<Controller>>) -> Response {
    if !ctrl.allow_shutdown() {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "shutdown disabled (set \"allow_shutdown\": true in config.json)"})),
        )
            .into_response();
    }
    std::thread::spawn(shutdown_now);
    (StatusCode::ACCEPTED, Json(json!({"shutting_down": true}))).into_response()
}

fn shutdown_now() {
    if std::env::var("IOT_SWITCH_SIMULATE").is_ok() {
        info!("simulated shutdown requested — not powering off");
        return;
    }
    // let the HTTP response flush before the box goes dark
    std::thread::sleep(std::time::Duration::from_millis(600));
    let attempts: [(&str, &[&str]); 3] = [
        ("systemctl", &["poweroff"]),
        ("/sbin/shutdown", &["-h", "now"]),
        ("sudo", &["-n", "systemctl", "poweroff"]),
    ];
    for (bin, args) in attempts {
        match std::process::Command::new(bin).args(args).spawn() {
            Ok(_) => {
                info!("issued `{bin} {:?}`", args);
                return;
            }
            Err(e) => warn!("failed to launch `{bin}`: {e}"),
        }
    }
    warn!("no viable shutdown command found");
}