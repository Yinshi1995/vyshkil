//! Допоміжне для REST-автентифікації (`server/src/api.rs`): прапорець аварійного парольного входу
//! і обмеження частоти спроб входу. Вся решта (server fn Leptos) видалена — рішення `remove-leptos-fallback`.

/// Аварійний вхід за паролем (12-auth.md §2.1): основний і єдиний штатний шлях — «Увійти через
/// ІКС» (§2.5). Парольний вхід, зміна/скидання пароля, створення акаунта з тимчасовим паролем і
/// «Запросити обліковий запис» працюють лише при `PASSWORD_LOGIN=1` на сервері (за замовчуванням
/// вимкнено). Читається з env на кожен виклик — дешево, і вмикається без перезбірки.
#[cfg(feature = "ssr")]
pub fn password_login_enabled() -> bool {
    std::env::var("PASSWORD_LOGIN").map(|v| v.trim() == "1").unwrap_or(false)
}

#[cfg(feature = "ssr")]
pub const PASSWORD_LOGIN_DISABLED: &str = "Вхід за паролем вимкнено — увійдіть через ІКС";

#[cfg(feature = "ssr")]
const MAX_LOGIN_ATTEMPTS: usize = 5;
#[cfg(feature = "ssr")]
const RATE_LIMIT_WINDOW_SECS: u64 = 900; // 15 хвилин

#[cfg(feature = "ssr")]
fn rate_limiter() -> &'static std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>> {
    use std::sync::OnceLock;
    static LIMITER: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>>> = OnceLock::new();
    LIMITER.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

#[cfg(feature = "ssr")]
pub fn check_rate_limit(ip: &str) -> bool {
    let mut map = rate_limiter().lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    let window = std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
    if let Some(attempts) = map.get_mut(ip) {
        attempts.retain(|t| now.duration_since(*t) < window);
        attempts.len() < MAX_LOGIN_ATTEMPTS
    } else {
        true
    }
}

#[cfg(feature = "ssr")]
pub fn record_failed_attempt(ip: &str) {
    let mut map = rate_limiter().lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    let window = std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
    let attempts = map.entry(ip.to_string()).or_default();
    attempts.retain(|t| now.duration_since(*t) < window);
    attempts.push(now);
}
