// Unix Domain Socket IPC server for Linux and macOS.
//
// `/tmp/coucou-<uid>.sock` or `$XDG_RUNTIME_DIR/coucou.sock`
// Forwards hook events to the island and holds PermissionRequest open for user approval.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use crate::island::WINDOW_LABEL;
use crate::log;

const DECISION_TIMEOUT: Duration = Duration::from_secs(108);
const ACK_TIMEOUT: Duration = Duration::from_millis(800);
const MAX_PAYLOAD: usize = 1 << 20;

pub enum Reply {
    Ack,
    Decision(String),
    Decline,
}

#[derive(Default)]
pub struct Pending(pub Mutex<HashMap<String, mpsc::Sender<Reply>>>);

static COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn pipe_name() -> String {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return format!("{runtime_dir}/coucou.sock");
    }
    let user = crate::win_user::current_user_sid().unwrap_or_else(|| "user".into());
    format!("/tmp/coucou-{user}.sock")
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let path = pipe_name();
        let _ = std::fs::remove_file(&path); // Clean up stale socket file if any
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(err) => {
                log::line(format!("cannot bind Unix socket at {path}: {err}"));
                return;
            }
        };

        // Restrict socket file permissions to 0700 (owner only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700));
        }

        log::line(format!("Unix socket relay server listening on {path}"));

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move { handle(app_clone, stream).await });
                }
                Err(err) => {
                    log::line(format!("socket accept error: {err}"));
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    });
}

async fn handle(app: AppHandle, mut stream: UnixStream) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_PAYLOAD {
                    log::line("payload exceeds limit");
                    return;
                }
            }
            Err(_) => break,
        }
    }

    let Ok(mut json) = serde_json::from_slice::<Value>(&buf) else {
        return;
    };
    let Some(event_type) = json.get("hook_event_name").and_then(|v| v.as_str()).map(String::from) else {
        return;
    };

    if event_type == "PermissionRequest" {
        let id = format!("req-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
        json["coucou_request_id"] = json!(id);

        let (tx, mut rx) = mpsc::channel(4);
        if let Some(pending) = app.try_state::<Pending>() {
            pending.0.lock().unwrap().insert(id.clone(), tx);
        }

        let _ = app.emit_to(WINDOW_LABEL, "hook", &json);

        let ack = tokio::time::timeout(ACK_TIMEOUT, rx.recv()).await;
        match ack {
            Ok(Some(Reply::Ack)) => {}
            _ => {
                cleanup(&app, &id);
                return;
            }
        }

        let decision = tokio::time::timeout(DECISION_TIMEOUT, rx.recv()).await;
        cleanup(&app, &id);

        let reply_str = match decision {
            Ok(Some(Reply::Decision(d))) => d,
            _ => "deny".into(),
        };

        let _ = stream.write_all(reply_str.as_bytes()).await;
        let _ = stream.flush().await;
    } else {
        let _ = app.emit_to(WINDOW_LABEL, "hook", &json);
    }
}

fn cleanup(app: &AppHandle, id: &str) {
    if let Some(pending) = app.try_state::<Pending>() {
        pending.0.lock().unwrap().remove(id);
    }
}

pub fn acknowledge(app: &AppHandle, id: &str) {
    send_reply(app, id, Reply::Ack);
}

pub fn answer(app: &AppHandle, id: &str, decision: &str) {
    send_reply(app, id, Reply::Decision(decision.to_string()));
}

pub fn decline(app: &AppHandle, id: &str) {
    send_reply(app, id, Reply::Decline);
}

fn send_reply(app: &AppHandle, id: &str, reply: Reply) {
    let sender = app
        .try_state::<Pending>()
        .and_then(|p| p.0.lock().unwrap().remove(id));
    if let Some(tx) = sender {
        let _ = tx.try_send(reply);
    }
}
