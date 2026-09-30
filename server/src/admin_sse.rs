//! SSE-стрім стану прив'язки WhatsApp (09-messaging.md §4) для сторінки `/admin/whatsapp` —
//! сирий Axum-хендлер, НЕ Leptos `#[server]`-функція (ті не стрімлять): читає NATS KV bucket
//! `notifier_status`/ключ `whatsapp` (пише `services/notifier`) і пересилає кожне оновлення як
//! SSE-подію. Права — query-параметри actor, той самий довірчий контракт, що вже всюди на цьому
//! етапі проєкту (INTERFACES.md: "У Stage 1 немає реальної автентифікації").

use std::convert::Infallible;

use app::backend::policy;
use app::types::actor::{Actor, Role};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ActorQuery {
    org_id: i32,
    role: String,
}

pub async fn whatsapp_status_stream(State(state): State<AppState>, Query(q): Query<ActorQuery>) -> Response {
    let Some(role) = Role::parse(&q.role) else {
        return (StatusCode::BAD_REQUEST, "невідома роль").into_response();
    };
    let actor = Actor { org_id: q.org_id, role };
    if !policy::is_admin(actor) {
        return (StatusCode::FORBIDDEN, "лише адмін").into_response();
    }

    let client = { state.nats.lock().expect("shared NATS mutex отруєний").clone() };
    let Some(client) = client else {
        return (StatusCode::SERVICE_UNAVAILABLE, "NATS недоступний").into_response();
    };

    let js = bus::jetstream(&client);
    let Ok(store) = js.get_key_value("notifier_status").await else {
        return (StatusCode::SERVICE_UNAVAILABLE, "notifier_status bucket ще не створено notifier'ом")
            .into_response();
    };
    // `watch_with_history` (не голий `watch`, який за замовчуванням `DeliverPolicy::New` -- лише
    // МАЙБУТНІ зміни): адмін, що відкрив сторінку, коли QR УЖЕ чекає, має побачити його одразу,
    // не чекати наступного оновлення (whatsapp-web.js оновлює QR раз на ~20с).
    let Ok(watch) = store.watch_with_history("whatsapp").await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "не вдалось підписатись на KV").into_response();
    };

    let events = watch.filter_map(|res| async move {
        let entry = res.ok()?;
        let text = String::from_utf8(entry.value.to_vec()).ok()?;
        Some(Ok::<_, Infallible>(Event::default().data(render_status(&text))))
    });

    Sse::new(events).keep_alive(KeepAlive::default()).into_response()
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
    Some(code.render::<svg::Color>().min_dimensions(200, 200).build())
}
