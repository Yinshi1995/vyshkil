//! Вхід через ІКС — OpenID Connect (Authorization Code + PKCE S256) до Pocket ID
//! (12-auth.md §2.5, `.claude/decisions/iks-sso-oidc.md`).
//!
//! Потік: `GET /api/auth/oidc/login` → редирект на `authorization_endpoint` ІКС (state, nonce,
//! code_challenge) → людина входить passkey-ключем → `GET /api/auth/oidc/callback?code&state` →
//! обмін коду на токени (client_secret_basic + code_verifier) → перевірка id_token (iss, aud,
//! exp, nonce) → доступ лише членам групи `OIDC_GROUP` → обліковий запис за логіном
//! (`preferred_username`, без урахування регістру) → синхронізація профілю → звичайна сесія
//! (та сама cookie `session`, що й при вході за паролем) → редирект на `/`.
//!
//! Підпис id_token не перевіряємо (JWT-бібліотеки в репозиторії немає): токен отримано напряму
//! з token endpoint по TLS з автентифікацією клієнта — OIDC Core 1.0 §3.1.3.7 п.6 дозволяє в
//! такому разі покладатися на TLS замість підпису.
//!
//! Стан незавершених входів (state → verifier+nonce) — у пам'яті процесу, TTL 10 хв (як
//! passkey-challenge, 12-auth.md §6): сервер один, рестарт = людина просто натискає кнопку ще раз.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use app::backend::repo;
use app::backend::repo::auth::{SsoAccount, SsoProfile};

use crate::state::AppState;

/// Скоупи: `groups` — щоб перевірити членство в `OIDC_GROUP`; кастомні claims (rank, callsign,
/// phone, delta_nick) Pocket ID віддає в межах `profile`.
pub const SCOPES: &str = "openid profile email groups";
const COOKIE_STATE: &str = "oidc_state";
const COOKIE_PATH: &str = "/api/auth/oidc";
const FLOW_TTL: Duration = Duration::from_secs(600);
/// Верхня межа незавершених входів у пам'яті — щоб боти, що смикають `/login`, не роздули RSS.
const MAX_PENDING: usize = 2000;
const DISCOVERY_TTL: Duration = Duration::from_secs(3600);
/// Допуск на розбіжність годинників між сервером і ІКС.
const CLOCK_SKEW_SECS: i64 = 60;
const AVATAR_MAX_BYTES: usize = 2 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Конфігурація
// ---------------------------------------------------------------------------

/// Без `Debug`: тут секрет клієнта, він не має потрапити в логи.
#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub group: String,
}

impl OidcConfig {
    /// Усі п'ять змінних обов'язкові; якщо хоч одна порожня — вхід через ІКС вимкнено
    /// (кнопка на `/login` не показується).
    pub fn from_env() -> Option<Self> {
        let get = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let vars = ["OIDC_ISSUER", "OIDC_CLIENT_ID", "OIDC_CLIENT_SECRET", "OIDC_REDIRECT_URI", "OIDC_GROUP"];
        let vals: Vec<Option<String>> = vars.iter().map(|k| get(k)).collect();
        if vals.iter().all(Option::is_none) {
            return None;
        }
        if let Some(missing) = vars.iter().zip(&vals).find(|(_, v)| v.is_none()).map(|(k, _)| k) {
            tracing::warn!("вхід через ІКС вимкнено: не задано {missing}");
            return None;
        }
        let mut it = vals.into_iter().map(Option::unwrap);
        Some(Self {
            issuer: it.next().unwrap().trim_end_matches('/').to_string(),
            client_id: it.next().unwrap(),
            client_secret: it.next().unwrap(),
            redirect_uri: it.next().unwrap(),
            group: it.next().unwrap(),
        })
    }

    fn secure_cookies(&self) -> bool {
        self.redirect_uri.starts_with("https://")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

struct PendingFlow {
    verifier: String,
    nonce: String,
    created: Instant,
}

/// Живе в `AppState` (`Option<Arc<Oidc>>`: `None` = ІКС не налаштовано).
pub struct Oidc {
    pub cfg: OidcConfig,
    http: reqwest::Client,
    pending: Mutex<HashMap<String, PendingFlow>>,
    discovery: tokio::sync::Mutex<Option<(Instant, Discovery)>>,
}

impl Oidc {
    pub fn new(cfg: OidcConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            // Без переходів за редиректами: аватар і токени беремо лише з хоста ІКС.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest client");
        Self { cfg, http, pending: Mutex::new(HashMap::new()), discovery: tokio::sync::Mutex::new(None) }
    }

    async fn discovery(&self) -> Result<Discovery, SsoError> {
        let mut guard = self.discovery.lock().await;
        if let Some((at, d)) = guard.as_ref() {
            if at.elapsed() < DISCOVERY_TTL {
                return Ok(d.clone());
            }
        }
        let url = format!("{}/.well-known/openid-configuration", self.cfg.issuer);
        let d: Discovery = self
            .http
            .get(&url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| {
                tracing::warn!("ІКС discovery недоступний: {e}");
                SsoError::Unavailable
            })?
            .json()
            .await
            .map_err(|_| SsoError::Unavailable)?;
        if !same_issuer(&d.issuer, &self.cfg.issuer) {
            tracing::warn!("ІКС discovery: issuer {} ≠ OIDC_ISSUER", d.issuer);
            return Err(SsoError::Unavailable);
        }
        *guard = Some((Instant::now(), d.clone()));
        Ok(d)
    }

    fn store_flow(&self, state: String, flow: PendingFlow) {
        let mut map = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        map.retain(|_, f| f.created.elapsed() < FLOW_TTL);
        if map.len() >= MAX_PENDING {
            // Переповнення = хтось бомбить /login; найстаріші входи відкидаємо.
            if let Some(oldest) = map.iter().min_by_key(|(_, f)| f.created).map(|(k, _)| k.clone()) {
                map.remove(&oldest);
            }
        }
        map.insert(state, flow);
    }

    /// Одноразово: забираємо запис, повторне використання того самого state неможливе.
    fn take_flow(&self, state: &str) -> Option<PendingFlow> {
        let mut map = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(state).filter(|f| f.created.elapsed() < FLOW_TTL)
    }
}

// ---------------------------------------------------------------------------
// Помилки → /login?sso_error=<код> (тексти — у web/src/pages/login.tsx)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsoError {
    /// ІКС не налаштовано на цьому сервері.
    Disabled,
    /// ІКС недоступний (discovery/token endpoint).
    Unavailable,
    /// Людина скасувала вхід або ІКС відмовив.
    Denied,
    /// state не збігся / прострочено / немає cookie.
    State,
    /// id_token або відповідь token endpoint не пройшли перевірку.
    Token,
    /// Не в групі `OIDC_GROUP`.
    Forbidden,
    /// Обліковий запис щойно створено — чекає адміністратора.
    Pending,
    /// Обліковий запис деактивовано (або ще не активовано адміністратором).
    Inactive,
    /// Логін уже прив'язано до іншого профілю ІКС.
    Conflict,
    /// Помилка бази/сервера.
    Internal,
}

impl SsoError {
    pub fn code(self) -> &'static str {
        match self {
            SsoError::Disabled => "disabled",
            SsoError::Unavailable => "unavailable",
            SsoError::Denied => "denied",
            SsoError::State => "state",
            SsoError::Token => "token",
            SsoError::Forbidden => "forbidden",
            SsoError::Pending => "pending",
            SsoError::Inactive => "inactive",
            SsoError::Conflict => "conflict",
            SsoError::Internal => "internal",
        }
    }
}

// ---------------------------------------------------------------------------
// Чисті функції (покриті тестами нижче)
// ---------------------------------------------------------------------------

/// Випадковий токен (base64url без padding) з криптографічного ГВЧ.
pub fn random_token(bytes: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

/// PKCE S256: `BASE64URL(SHA256(verifier))` (RFC 7636 §4.2).
pub fn pkce_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn authorize_url(endpoint: &str, cfg: &OidcConfig, state: &str, nonce: &str, challenge: &str) -> String {
    let enc = |s: &str| urlencoding::encode(s).into_owned();
    let sep = if endpoint.contains('?') { '&' } else { '?' };
    format!(
        "{endpoint}{sep}response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}\
         &code_challenge={}&code_challenge_method=S256",
        enc(&cfg.client_id),
        enc(&cfg.redirect_uri),
        enc(SCOPES),
        enc(state),
        enc(nonce),
        enc(challenge),
    )
}

fn same_issuer(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, id: &str) -> bool {
        match self {
            Audience::One(a) => a == id,
            Audience::Many(v) => v.iter().any(|a| a == id),
        }
    }
    fn len(&self) -> usize {
        match self {
            Audience::One(_) => 1,
            Audience::Many(v) => v.len(),
        }
    }
}

/// Claims id_token, які нас цікавлять. Кастомні (rank, callsign, phone, delta_nick) — з
/// Pocket ID, можуть бути відсутні.
#[derive(Debug, Clone, Deserialize)]
pub struct IdClaims {
    pub iss: String,
    pub sub: String,
    aud: Audience,
    pub exp: i64,
    #[serde(default)]
    pub azp: Option<String>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub rank: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub callsign: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub phone: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub delta_nick: Option<String>,
}

/// Кастомний claim у Pocket ID — рядок, але не даємо всьому входу впасти, якщо адміністратор
/// ІКС колись задасть число/булеве: беремо текстове представлення, решту ігноруємо.
fn lenient_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let v = Option::<serde_json::Value>::deserialize(d)?;
    Ok(match v {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    })
}

/// Розбирає payload JWT (без перевірки підпису — див. шапку модуля).
pub fn decode_jwt_claims(token: &str) -> Result<IdClaims, SsoError> {
    let mut parts = token.split('.');
    let (Some(_header), Some(payload), Some(_sig), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(SsoError::Token);
    };
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| SsoError::Token)?;
    serde_json::from_slice(&bytes).map_err(|_| SsoError::Token)
}

/// Перевірка id_token за OIDC Core §3.1.3.7: iss, aud (+azp при кількох aud), exp, nonce;
/// плюс наші вимоги — непорожні `sub` і коректний `preferred_username`.
pub fn validate_id_token(
    token: &str,
    issuer: &str,
    client_id: &str,
    expected_nonce: &str,
    now_unix: i64,
) -> Result<IdClaims, SsoError> {
    let c = decode_jwt_claims(token)?;
    if !same_issuer(&c.iss, issuer) {
        return Err(SsoError::Token);
    }
    if !c.aud.contains(client_id) {
        return Err(SsoError::Token);
    }
    if c.aud.len() > 1 && c.azp.as_deref() != Some(client_id) {
        return Err(SsoError::Token);
    }
    if c.exp + CLOCK_SKEW_SECS < now_unix {
        return Err(SsoError::Token);
    }
    if c.nonce.as_deref() != Some(expected_nonce) {
        return Err(SsoError::Token);
    }
    if c.sub.trim().is_empty() {
        return Err(SsoError::Token);
    }
    if login_from_claims(&c).is_none() {
        return Err(SsoError::Token);
    }
    Ok(c)
}

/// Логін — `preferred_username` (латиниця). Ті самі обмеження, що в адмінському створенні
/// (мін. 3 символи), плюс безпечний набір символів.
pub fn login_from_claims(c: &IdClaims) -> Option<String> {
    let login = c.preferred_username.as_deref()?.trim();
    let ok = login.len() >= 3
        && login.len() <= 64
        && login.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'));
    ok.then(|| login.to_string())
}

pub fn has_group(c: &IdClaims, group: &str) -> bool {
    c.groups.iter().any(|g| g == group)
}

fn clean(v: &Option<String>) -> Option<String> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// Поля профілю з claims. Відсутній/порожній claim → `None` (поле в ІКС порожнє — і тут).
pub fn profile_from_claims(c: &IdClaims) -> SsoProfile {
    let first_name = clean(&c.given_name);
    let last_name = clean(&c.family_name);
    let display_name = clean(&c.name).or_else(|| {
        let joined = [first_name.as_deref(), last_name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        (!joined.is_empty()).then_some(joined)
    });
    SsoProfile {
        display_name,
        first_name,
        last_name,
        rank: clean(&c.rank),
        phone: clean(&c.phone),
        callsign: clean(&c.callsign),
        delta_nick: clean(&c.delta_nick),
    }
}

/// Аватар качаємо лише з того ж хоста (схема+хост+порт), що й ІКС — не довільний URL з claim.
pub fn avatar_url_allowed(picture: &str, issuer: &str) -> bool {
    let (Ok(p), Ok(i)) = (reqwest::Url::parse(picture), reqwest::Url::parse(issuer)) else {
        return false;
    };
    p.scheme() == i.scheme() && p.host_str() == i.host_str() && p.port_or_known_default() == i.port_or_known_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapping {
    /// Наявний обліковий запис; `bind` = вперше входить через ІКС, треба зберегти `sub`.
    Existing { user_id: i32, bind: bool },
    /// Облікового запису з таким логіном немає — створити (неактивним, див. repo::create_sso_user).
    Create,
    /// Логін або `sub` уже прив'язано до іншого профілю/акаунта — вирішує адміністратор.
    Conflict,
}

/// `by_login` — запис з тим самим логіном (без регістру); `sub_owner` — id запису, вже
/// прив'язаного до цього `sub` (якщо є).
pub fn decide_mapping(by_login: Option<&SsoAccount>, sub_owner: Option<i32>, sub: &str) -> Mapping {
    match by_login {
        Some(acc) => {
            if acc.oidc_sub.as_deref().is_some_and(|s| s != sub) {
                return Mapping::Conflict;
            }
            if sub_owner.is_some_and(|id| id != acc.id) {
                return Mapping::Conflict;
            }
            Mapping::Existing { user_id: acc.id, bind: acc.oidc_sub.is_none() }
        }
        // Профіль ІКС вже прив'язаний до акаунта з іншим логіном (перейменування в ІКС) —
        // не створюємо дубль мовчки.
        None if sub_owner.is_some() => Mapping::Conflict,
        None => Mapping::Create,
    }
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

fn redirect(location: &str, cookies: &[String]) -> Response {
    let mut b = Response::builder().status(StatusCode::SEE_OTHER).header(header::LOCATION, location);
    for c in cookies {
        // builder.header() додає, а не замінює — кілька Set-Cookie в одній відповіді.
        b = b.header(header::SET_COOKIE, c);
    }
    b.header(header::CACHE_CONTROL, "no-store").body(Body::empty()).unwrap()
}

fn state_cookie(cfg: &OidcConfig, value: &str, max_age: u32) -> String {
    // SameSite=Lax (не Strict): повернення з id.lab — крос-сайтова навігація, Strict-cookie
    // браузер на неї не надіслав би.
    let secure = if cfg.secure_cookies() { "; Secure" } else { "" };
    format!("{COOKIE_STATE}={value}; HttpOnly; SameSite=Lax; Path={COOKIE_PATH}; Max-Age={max_age}{secure}")
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|p| p.trim().strip_prefix(prefix.as_str()).map(str::to_string))
        .filter(|v| !v.is_empty())
}

fn fail(cfg: Option<&OidcConfig>, e: SsoError) -> Response {
    let cookies: Vec<String> = cfg.map(|c| vec![state_cookie(c, "", 0)]).unwrap_or_default();
    redirect(&format!("/login?sso_error={}", e.code()), &cookies)
}

#[derive(Serialize)]
struct MethodsDto {
    sso: bool,
    password: bool,
    /// Публічна адреса ІКС (для посилання «Профіль ведеться в ІКС» у налаштуваннях).
    sso_url: Option<String>,
}

/// Які способи входу показувати на `/login` (публічний ендпоінт).
async fn methods_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(MethodsDto {
        sso: state.oidc.is_some(),
        password: app::services::auth::password_login_enabled(),
        sso_url: state.oidc.as_ref().map(|o| o.cfg.issuer.clone()),
    })
}

async fn login_handler(State(state): State<AppState>) -> Response {
    let Some(oidc) = state.oidc.as_ref() else {
        return fail(None, SsoError::Disabled);
    };
    let disc = match oidc.discovery().await {
        Ok(d) => d,
        Err(e) => return fail(Some(&oidc.cfg), e),
    };
    let state_val = random_token(32);
    let nonce = random_token(32);
    let verifier = random_token(48); // 64 символи — в межах 43..128 (RFC 7636 §4.1)
    let url = authorize_url(&disc.authorization_endpoint, &oidc.cfg, &state_val, &nonce, &pkce_challenge(&verifier));
    oidc.store_flow(state_val.clone(), PendingFlow { verifier, nonce, created: Instant::now() });
    redirect(&url, &[state_cookie(&oidc.cfg, &state_val, FLOW_TTL.as_secs() as u32)])
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
}

async fn callback_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let Some(oidc) = state.oidc.clone() else {
        return fail(None, SsoError::Disabled);
    };
    match complete_login(&state, &oidc, &headers, q).await {
        Ok(session_cookie) => redirect("/", &[session_cookie, state_cookie(&oidc.cfg, "", 0)]),
        Err(e) => fail(Some(&oidc.cfg), e),
    }
}

async fn complete_login(
    state: &AppState,
    oidc: &Oidc,
    headers: &HeaderMap,
    q: CallbackQuery,
) -> Result<String, SsoError> {
    if q.error.is_some() {
        return Err(SsoError::Denied);
    }
    let (Some(code), Some(state_q)) = (q.code, q.state) else {
        return Err(SsoError::State);
    };
    // state з URL має збігтися з cookie цього браузера (захист від login-CSRF) і бути живим.
    if cookie_value(headers, COOKIE_STATE).as_deref() != Some(state_q.as_str()) {
        return Err(SsoError::State);
    }
    let flow = oidc.take_flow(&state_q).ok_or(SsoError::State)?;

    let disc = oidc.discovery().await?;
    let cfg = &oidc.cfg;
    // client_secret_basic: id і секрет form-urlencoded перед Basic (RFC 6749 §2.3.1).
    let resp = oidc
        .http
        .post(&disc.token_endpoint)
        .basic_auth(
            urlencoding::encode(&cfg.client_id).into_owned(),
            Some(urlencoding::encode(&cfg.client_secret).into_owned()),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", cfg.redirect_uri.as_str()),
            ("code_verifier", flow.verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("ІКС token endpoint недоступний: {e}");
            SsoError::Unavailable
        })?;
    if !resp.status().is_success() {
        tracing::warn!("ІКС token endpoint відповів {}", resp.status());
        return Err(SsoError::Token);
    }
    let tokens: TokenResponse = resp.json().await.map_err(|_| SsoError::Token)?;
    let id_token = tokens.id_token.ok_or(SsoError::Token)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let claims = validate_id_token(&id_token, &cfg.issuer, &cfg.client_id, &flow.nonce, now)?;

    if !has_group(&claims, &cfg.group) {
        return Err(SsoError::Forbidden);
    }
    let login = login_from_claims(&claims).ok_or(SsoError::Token)?;

    let db = &state.db;
    let ise = |e: sea_orm::DbErr| {
        tracing::error!("вхід через ІКС: помилка БД: {e}");
        SsoError::Internal
    };
    let by_login = repo::auth::find_user_for_sso(db, &login).await.map_err(ise)?;
    let sub_owner = repo::auth::find_user_id_by_oidc_sub(db, &claims.sub).await.map_err(ise)?;
    let (user_id, created) = match decide_mapping(by_login.as_ref(), sub_owner, &claims.sub) {
        Mapping::Conflict => {
            tracing::warn!("вхід через ІКС: конфлікт прив'язки для логіна {login}");
            return Err(SsoError::Conflict);
        }
        Mapping::Existing { user_id, bind } => {
            if bind {
                repo::auth::bind_oidc_sub(db, user_id, &claims.sub).await.map_err(ise)?;
            }
            (user_id, false)
        }
        Mapping::Create => {
            let id = repo::auth::create_sso_user(db, &login, &claims.sub).await.map_err(ise)?;
            tracing::info!("вхід через ІКС: створено обліковий запис id={id}, чекає адміністратора");
            (id, true)
        }
    };

    // Профіль синхронізуємо завжди (і для неактивних — адміністратор бачить, кого активує).
    repo::auth::sync_sso_profile(db, user_id, &profile_from_claims(&claims)).await.map_err(ise)?;
    if let Some(picture) = clean(&claims.picture) {
        if avatar_url_allowed(&picture, &cfg.issuer) {
            if let Err(e) = sync_avatar(oidc, db, user_id, &picture).await {
                // Аватар — не привід зривати вхід.
                tracing::warn!("вхід через ІКС: аватар не оновлено: {e}");
            }
        }
    }

    // Активність перевіряємо після синхронізації, але до створення сесії.
    let active = repo::auth::find_user_for_sso(db, &login)
        .await
        .map_err(ise)?
        .is_some_and(|a| a.id == user_id && a.is_active);
    if !active {
        return Err(if created { SsoError::Pending } else { SsoError::Inactive });
    }

    let session_id = crate::api::start_session(db, user_id).await.map_err(ise)?;
    tracing::info!("вхід через ІКС: user_id={user_id}");
    Ok(crate::api::session_cookie(&session_id))
}

/// Аватар з ІКС → та сама локальна тека й колонка, що й завантаження з налаштувань
/// (`data/avatars/user_<id>.<ext>`, `user_account.avatar_path`): браузер тягне його з нашого
/// origin, решта застосунку (довідник, чат) працює без змін.
async fn sync_avatar(oidc: &Oidc, db: &sea_orm::DatabaseConnection, user_id: i32, url: &str) -> Result<(), String> {
    let resp = oidc.http.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    if resp.content_length().is_some_and(|l| l as usize > AVATAR_MAX_BYTES) {
        return Err("завеликий файл".into());
    }
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let ext = match ct.as_str() {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        other => return Err(format!("непідтримуваний тип {other:?}")),
    };
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() > AVATAR_MAX_BYTES || bytes.is_empty() {
        return Err("неприйнятний розмір".into());
    }
    let dir = std::path::Path::new("data/avatars");
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let filename = format!("user_{user_id}.{ext}");
    std::fs::write(dir.join(&filename), &bytes).map_err(|e| e.to_string())?;
    // Старий файл з іншим розширенням — прибрати, щоб не лишався сиротою.
    for other in ["png", "jpg", "webp"].into_iter().filter(|e| *e != ext) {
        let _ = std::fs::remove_file(dir.join(format!("user_{user_id}.{other}")));
    }
    repo::auth::set_avatar_path(db, user_id, Some(&filename)).await.map_err(|e| e.to_string())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/methods", get(methods_handler))
        .route("/api/auth/oidc/login", get(login_handler))
        .route("/api/auth/oidc/callback", get(callback_handler))
}

pub fn from_config(cfg: Option<OidcConfig>) -> Option<Arc<Oidc>> {
    cfg.map(|c| Arc::new(Oidc::new(c)))
}

// ---------------------------------------------------------------------------
// Тести
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ISS: &str = "https://id.lab.okpivden.uk";
    const CLIENT: &str = "vyshkil";
    const NOW: i64 = 1_800_000_000;

    fn jwt(payload: serde_json::Value) -> String {
        let e = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        format!("{}.{}.{}", e(br#"{"alg":"RS256","typ":"JWT"}"#), e(payload.to_string().as_bytes()), e(b"sig"))
    }

    fn base() -> serde_json::Value {
        serde_json::json!({
            "iss": ISS,
            "sub": "0b9f7a2e-1111-2222-3333-444455556666",
            "aud": CLIENT,
            "exp": NOW + 300,
            "iat": NOW,
            "nonce": "n-123",
            "preferred_username": "Petrenko",
            "given_name": " Іван ",
            "family_name": "Петренко",
            "name": "Іван Петренко",
            "picture": "https://id.lab.okpivden.uk/api/users/x/profile-picture.png",
            "groups": ["vyshkil", "pult"],
            "rank": "капітан",
            "callsign": "Сокіл",
            "phone": "",
        })
    }

    fn with(mut v: serde_json::Value, k: &str, val: serde_json::Value) -> serde_json::Value {
        v[k] = val;
        v
    }

    fn validate(v: serde_json::Value) -> Result<IdClaims, SsoError> {
        validate_id_token(&jwt(v), ISS, CLIENT, "n-123", NOW)
    }

    #[test]
    fn pkce_rfc7636_appendix_b() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn random_tokens_are_urlsafe_and_unique() {
        let a = random_token(48);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(a, random_token(48));
    }

    #[test]
    fn authorize_url_has_pkce_state_nonce() {
        let cfg = OidcConfig {
            issuer: ISS.into(),
            client_id: CLIENT.into(),
            client_secret: "s".into(),
            redirect_uri: "https://vyshkil.lab.okpivden.uk/api/auth/oidc/callback".into(),
            group: "vyshkil".into(),
        };
        let u = authorize_url("https://id.lab.okpivden.uk/authorize", &cfg, "st", "nn", "ch");
        assert!(u.starts_with("https://id.lab.okpivden.uk/authorize?response_type=code&client_id=vyshkil"));
        assert!(u.contains("redirect_uri=https%3A%2F%2Fvyshkil.lab.okpivden.uk%2Fapi%2Fauth%2Foidc%2Fcallback"));
        assert!(u.contains("scope=openid%20profile%20email%20groups"));
        assert!(u.contains("&state=st&nonce=nn&code_challenge=ch&code_challenge_method=S256"));
    }

    #[test]
    fn valid_token_passes_and_maps_profile() {
        let c = validate(base()).expect("valid");
        assert_eq!(login_from_claims(&c).as_deref(), Some("Petrenko"));
        assert!(has_group(&c, "vyshkil"));
        assert!(!has_group(&c, "Vyshkil"));
        let p = profile_from_claims(&c);
        assert_eq!(p.first_name.as_deref(), Some("Іван"));
        assert_eq!(p.last_name.as_deref(), Some("Петренко"));
        assert_eq!(p.display_name.as_deref(), Some("Іван Петренко"));
        assert_eq!(p.rank.as_deref(), Some("капітан"));
        assert_eq!(p.callsign.as_deref(), Some("Сокіл"));
        assert_eq!(p.phone, None, "порожній claim = порожнє поле");
        assert_eq!(p.delta_nick, None, "відсутній claim = порожнє поле");
    }

    #[test]
    fn wrong_issuer_audience_nonce_or_expired_rejected() {
        assert_eq!(validate(with(base(), "iss", "https://evil.example".into())).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "aud", "pult".into())).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "nonce", "other".into())).unwrap_err(), SsoError::Token);
        let mut no_nonce = base();
        no_nonce.as_object_mut().unwrap().remove("nonce");
        assert_eq!(validate(no_nonce).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "exp", (NOW - 120).into())).unwrap_err(), SsoError::Token);
        // У межах допуску годинників — ще дійсний.
        assert!(validate(with(base(), "exp", (NOW - 30).into())).is_ok());
    }

    #[test]
    fn issuer_trailing_slash_tolerated() {
        assert!(validate(with(base(), "iss", format!("{ISS}/").into())).is_ok());
    }

    #[test]
    fn multiple_audiences_require_azp() {
        let multi = with(base(), "aud", serde_json::json!(["vyshkil", "other"]));
        assert_eq!(validate(multi.clone()).unwrap_err(), SsoError::Token);
        assert!(validate(with(multi, "azp", CLIENT.into())).is_ok());
        assert!(validate(with(base(), "aud", serde_json::json!(["vyshkil"]))).is_ok());
    }

    #[test]
    fn bad_login_or_sub_rejected() {
        assert_eq!(validate(with(base(), "preferred_username", "ab".into())).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "preferred_username", "іван".into())).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "preferred_username", "a b c".into())).unwrap_err(), SsoError::Token);
        assert_eq!(validate(with(base(), "sub", "".into())).unwrap_err(), SsoError::Token);
    }

    #[test]
    fn malformed_tokens_rejected() {
        for t in ["", "a.b", "a.b.c.d", "a.!!!.c"] {
            assert_eq!(validate_id_token(t, ISS, CLIENT, "n-123", NOW).unwrap_err(), SsoError::Token);
        }
    }

    #[test]
    fn missing_groups_means_no_access() {
        let mut v = base();
        v.as_object_mut().unwrap().remove("groups");
        let c = validate(v).unwrap();
        assert!(!has_group(&c, "vyshkil"));
    }

    #[test]
    fn non_string_custom_claims_tolerated() {
        let c = validate(with(base(), "phone", serde_json::json!(380501234567u64))).unwrap();
        assert_eq!(profile_from_claims(&c).phone.as_deref(), Some("380501234567"));
    }

    #[test]
    fn display_name_falls_back_to_given_family() {
        let mut v = base();
        v.as_object_mut().unwrap().remove("name");
        let c = validate(v).unwrap();
        assert_eq!(profile_from_claims(&c).display_name.as_deref(), Some("Іван Петренко"));
    }

    #[test]
    fn avatar_only_from_issuer_host() {
        assert!(avatar_url_allowed("https://id.lab.okpivden.uk/api/users/1/profile-picture.png", ISS));
        assert!(!avatar_url_allowed("http://id.lab.okpivden.uk/x.png", ISS));
        assert!(!avatar_url_allowed("https://id.lab.okpivden.uk:8443/x.png", ISS));
        assert!(!avatar_url_allowed("https://evil.example/x.png", ISS));
        assert!(!avatar_url_allowed("https://id.lab.okpivden.uk.evil.example/x.png", ISS));
        assert!(!avatar_url_allowed("/relative.png", ISS));
    }

    fn acc(id: i32, sub: Option<&str>) -> SsoAccount {
        SsoAccount { id, login: "petrenko".into(), is_active: true, oidc_sub: sub.map(str::to_string) }
    }

    #[test]
    fn mapping_existing_unbound_binds() {
        assert_eq!(decide_mapping(Some(&acc(7, None)), None, "S"), Mapping::Existing { user_id: 7, bind: true });
    }

    #[test]
    fn mapping_existing_same_sub() {
        assert_eq!(decide_mapping(Some(&acc(7, Some("S"))), Some(7), "S"), Mapping::Existing { user_id: 7, bind: false });
    }

    #[test]
    fn mapping_login_bound_to_other_profile_is_conflict() {
        assert_eq!(decide_mapping(Some(&acc(7, Some("OTHER"))), None, "S"), Mapping::Conflict);
    }

    #[test]
    fn mapping_sub_bound_to_other_account_is_conflict() {
        assert_eq!(decide_mapping(Some(&acc(7, None)), Some(9), "S"), Mapping::Conflict);
        assert_eq!(decide_mapping(None, Some(9), "S"), Mapping::Conflict);
    }

    #[test]
    fn mapping_unknown_creates() {
        assert_eq!(decide_mapping(None, None, "S"), Mapping::Create);
    }

    #[test]
    fn state_cookie_is_lax_and_scoped() {
        let cfg = OidcConfig {
            issuer: ISS.into(),
            client_id: CLIENT.into(),
            client_secret: "s".into(),
            redirect_uri: "https://vyshkil.lab.okpivden.uk/api/auth/oidc/callback".into(),
            group: "vyshkil".into(),
        };
        let c = state_cookie(&cfg, "abc", 600);
        assert!(c.contains("SameSite=Lax") && c.contains("Path=/api/auth/oidc") && c.contains("Secure") && c.contains("HttpOnly"));
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, "session=x; oidc_state=abc".parse().unwrap());
        assert_eq!(cookie_value(&h, COOKIE_STATE).as_deref(), Some("abc"));
        assert_eq!(cookie_value(&h, "missing"), None);
    }
}
