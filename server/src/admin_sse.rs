//! SSE-стрім стану прив'язки WhatsApp (09-messaging.md §4) для сторінки `/admin/whatsapp` —
//! сирий Axum-хендлер, НЕ Leptos `#[server]`-функція (ті не стрімлять): читає NATS KV bucket
//! `notifier_status`/ключ `whatsapp` (пише `services/notifier`) і пересилає кожне оновлення як
//! SSE-подію. Права — query-параметри actor, той самий довірчий контракт, що вже всюди на цьому
//! етапі проєкту (INTERFACES.md: "У Stage 1 немає реальної автентифікації").

use std::convert::Infallible;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::api::{require_admin_actor, require_auth};
use crate::state::AppState;

fn not_running_json() -> String {
    serde_json::json!({
        "state": "not_running",
        "qr_svg": null,
        "pairing_code": null,
        "phone_masked": null,
        "updated_at": ""
    })
    .to_string()
}

fn not_running_sse() -> Response {
    let events = futures::stream::iter(vec![
        Ok::<_, Infallible>(Event::default().data(not_running_json())),
    ])
    .chain(futures::stream::pending());
    Sse::new(events)
        .keep_alive(KeepAlive::default())
        .into_response()
}

pub async fn whatsapp_status_stream(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_auth(&state.db, &headers).await {
        Ok(u) => u,
        Err(status) => return (status, "").into_response(),
    };
    if let Err(status) = require_admin_actor(&user) {
        return (status, "лише адмін").into_response();
    }

    let client = { state.nats.lock().expect("shared NATS mutex отруєний").clone() };
    let Some(client) = client else {
        return not_running_sse();
    };

    let js = bus::jetstream(&client);
    let Ok(store) = js.get_key_value("notifier_status").await else {
        return not_running_sse();
    };

    let current = store.get("whatsapp").await.ok().flatten();
    let initial_json = match current {
        Some(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            render_status(&text)
        }
        None => not_running_json(),
    };

    // Watch for future changes (DeliverPolicy::New — only updates after this point)
    let watch_stream = match store.watch("whatsapp").await {
        Ok(w) => w.filter_map(|res| async move {
            let entry = res.ok()?;
            let text = String::from_utf8(entry.value.to_vec()).ok()?;
            Some(Ok::<_, Infallible>(Event::default().data(render_status(&text))))
        }).left_stream(),
        Err(_) => futures::stream::pending::<Result<Event, Infallible>>().right_stream(),
    };

    let events = futures::stream::iter(vec![
        Ok::<_, Infallible>(Event::default().data(initial_json)),
    ])
    .chain(watch_stream);

    Sse::new(events)
        .keep_alive(KeepAlive::default())
        .into_response()
}

#[derive(Deserialize)]
struct StatusRaw {
    state: String,
    qr: Option<String>,
    pairing_code: Option<String>,
    phone_masked: Option<String>,
    updated_at: String,
}

#[derive(Serialize)]
struct StatusForClient {
    state: String,
    qr_svg: Option<String>,
    pairing_code: Option<String>,
    phone_masked: Option<String>,
    updated_at: String,
}

/// QR — SVG, зрендерений СЕРВЕРОМ (не client-side JS з CDN — CLAUDE.md: "жодних мережевих
/// викликів... шрифти/CDN — локально", той самий принцип для будь-якого стороннього JS).
fn render_status(raw_json: &str) -> String {
    let Ok(raw) = serde_json::from_str::<StatusRaw>(raw_json) else {
        return raw_json.to_string();
    };
    let qr_svg = raw.qr.as_deref().and_then(render_qr_svg);
    let out = StatusForClient {
        state: raw.state,
        qr_svg,
        pairing_code: raw.pairing_code,
        phone_masked: raw.phone_masked,
        updated_at: raw.updated_at,
    };
    serde_json::to_string(&out).unwrap_or_else(|_| raw_json.to_string())
}

fn render_qr_svg(data: &str) -> Option<String> {
    use qrcode::render::svg;
    use qrcode::QrCode;
    let code = QrCode::new(data.as_bytes()).ok()?;
    Some(
        code.render::<svg::Color>()
            .min_dimensions(200, 200)
            .build(),
    )
}
