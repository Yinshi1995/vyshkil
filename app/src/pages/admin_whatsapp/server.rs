//! Команди прив'язки WhatsApp (09-messaging.md §4) — NATS core request-reply до `notifier`
//! (`services/notifier/src/commands.ts`), лише `admin`. Стрім статусу (QR/стан) — ОКРЕМИЙ raw
//! Axum SSE-ендпоінт (`server::admin_sse`, не тут — `#[server]`-функції не стрімлять).

use leptos::prelude::*;
use serde::Deserialize;

use crate::types::actor::Actor;

const SUBJECT_PAIR_CODE: &str = "vyshkil.notifier.whatsapp.pair.code";
const SUBJECT_LOGOUT: &str = "vyshkil.notifier.whatsapp.logout";
const SUBJECT_TEST: &str = "vyshkil.notifier.whatsapp.test";

// `#[cfg(feature = "ssr")]` явно -- це ПЛОСКІ допоміжні функції, не самі `#[server(...)]`-функції
// (той макрос сам вирізає своє тіло під non-ssr збірку, тому звичайні `#[server]` fn можуть
// вільно імпортувати `backend::policy` без явного cfg) — без цього атрибута `backend`
// (`#[cfg(feature = "ssr")] pub mod backend;`, `app/src/lib.rs`) і опційний `bus` (§ Cargo.toml)
// не існують під `hydrate`-only збіркою WASM-фронтенду, і ця тека не скомпілюється для клієнта.
#[cfg(feature = "ssr")]
fn require_admin(actor: Option<Actor>) -> Result<Actor, ServerFnError> {
    use crate::backend::policy;
    let actor = actor.ok_or_else(|| ServerFnError::new("оберіть актора вгорі"))?;
    if !policy::is_admin(actor) {
        return Err(ServerFnError::new("лише адміністратор керує прив'язкою WhatsApp"));
    }
    Ok(actor)
}

#[cfg(feature = "ssr")]
async fn nats_request(subject: &str, payload_json: String) -> Result<String, ServerFnError> {
    let shared = expect_context::<bus::SharedNatsClient>();
    let client = { shared.lock().map_err(|_| ServerFnError::new("NATS mutex отруєний"))?.clone() };
    let client = client.ok_or_else(|| ServerFnError::new("NATS недоступний"))?;
    let reply = bus::request(&client, subject, payload_json.into_bytes())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    String::from_utf8(reply).map_err(|e| ServerFnError::new(format!("невалідна відповідь: {e}")))
}

#[derive(Deserialize)]
struct ErrorReply {
    error: Option<String>,
}

fn check_error(raw: &str) -> Result<(), ServerFnError> {
    if let Ok(ErrorReply { error: Some(e) }) = serde_json::from_str::<ErrorReply>(raw) {
        return Err(ServerFnError::new(e));
    }
    Ok(())
}

#[derive(Deserialize)]
struct PairCodeReply {
    pairing_code: Option<String>,
}

/// Повертає 8-символьний pairing-код (09 §4: "Прив'язати за номером") — `phone` у форматі
/// `notifier` очікує (цифри, міжнародний формат, без `+`/пробілів — те саме, що `whatsapp-web.js`'s
/// `requestPairingCode`).
#[server(RequestPairingCode, "/api")]
pub async fn request_pairing_code(actor: Option<Actor>, phone: String) -> Result<String, ServerFnError> {
    require_admin(actor)?;
    let payload = serde_json::json!({ "phone": phone }).to_string();
    let raw = nats_request(SUBJECT_PAIR_CODE, payload).await?;
    check_error(&raw)?;
    let reply: PairCodeReply =
        serde_json::from_str(&raw).map_err(|e| ServerFnError::new(format!("невалідна відповідь: {e}")))?;
    reply.pairing_code.ok_or_else(|| ServerFnError::new("notifier не повернув pairing-код"))
}

#[server(LogoutWhatsapp, "/api")]
pub async fn logout_whatsapp(actor: Option<Actor>) -> Result<(), ServerFnError> {
    require_admin(actor)?;
    let raw = nats_request(SUBJECT_LOGOUT, "{}".to_string()).await?;
    check_error(&raw)
}

#[server(SendTestNotification, "/api")]
pub async fn send_test_notification(actor: Option<Actor>, org_id: i32) -> Result<String, ServerFnError> {
    require_admin(actor)?;
    let payload = serde_json::json!({ "org_id": org_id }).to_string();
    let raw = nats_request(SUBJECT_TEST, payload).await?;
    check_error(&raw)?;
    Ok(raw)
}
