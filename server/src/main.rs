use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Request},
    middleware::Next,
    response::Response,
    Router,
};
use mimalloc::MiMalloc;
use migration::MigratorTrait;

use server::{admin_sse, api, chat_api, config, db, personnel_api, relay, spa, state::AppState};

// mimalloc фрагментує купу помітно менше за glibc malloc на довгоживучих процесах
// і активно повертає вільну пам'ять ОС — на сервері з обмеженим RAM це відчутно знижує RSS.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let config = config::Config::from_env();
    init_tracing(&config.log_level);

    // Раннтайм будуємо вручну (а не через #[tokio::main]), бо кількість воркер-потоків
    // має братися зі SERVER_WORKER_THREADS у рантаймі, а не бути захардкоджена в атрибуті макроса.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    runtime.block_on(run(config));
}

fn init_tracing(log_level: &str) {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    tracing_subscriber::registry()
        .with(EnvFilter::try_new(log_level).unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn run(config: config::Config) {
    let db = db::connect(&config)
        .await
        .expect("failed to connect to the database\n {e}");

    // Міграції прогонюються при старті сервера, щоб схема БД завжди відповідала коду, який щойно задеплоїли,
    // без окремого ручного кроку `sea-orm-cli migrate up` на кожному релізі.
    migration::Migrator::up(&db, None)
        .await
        .expect("failed to apply pending migrations");

    // Адреса прослуховування: `LEPTOS_SITE_ADDR` (ім'я лишилось від Leptos, щоб не міняти compose/прод-env).
    let addr: std::net::SocketAddr = std::env::var("LEPTOS_SITE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()
        .expect("LEPTOS_SITE_ADDR має бути адресою виду host:port");

    let shared_nats: bus::SharedNatsClient = std::sync::Arc::new(std::sync::Mutex::new(None));
    let (chat_tx, _) = tokio::sync::broadcast::channel(256);
    let chat_tx = std::sync::Arc::new(chat_tx);
    let oidc = server::oidc::from_config(config.oidc.clone());
    if oidc.is_some() {
        tracing::info!("вхід через ІКС увімкнено");
    }
    if app::services::auth::password_login_enabled() {
        tracing::warn!("PASSWORD_LOGIN=1 — аварійний вхід за паролем увімкнено");
    }
    if let Some(login) = std::env::var("DEV_AUTOLOGIN").ok().filter(|v| !v.is_empty()) {
        // Лише стенд тестування: без ІКС і лише при аварійному парольному режимі (див. api::DEV_AUTOLOGIN).
        assert!(
            oidc.is_none() && std::env::var("OIDC_ISSUER").map(|v| v.is_empty()).unwrap_or(true) && app::services::auth::password_login_enabled(),
            "DEV_AUTOLOGIN заборонено разом із входом через ІКС і без PASSWORD_LOGIN=1"
        );
        tracing::warn!("DEV_AUTOLOGIN={login} — автовхід без пароля (ТІЛЬКИ стенд тестування)");
        let _ = server::api::DEV_AUTOLOGIN.set(login);
    }
    let app_state = AppState { db, nats: shared_nats.clone(), chat_tx, oidc };

    // Перша фонова задача в цьому проєкті (09-messaging.md §3.1, Фаза 1) -- `tokio::spawn`, не
    // блокує запуск: NATS недоступний при старті — relay сам ретраїть, сервер піднімається як
    // завжди. `shared_nats` — той самий клон, що адмін-сторінка `/admin/whatsapp` читає через
    // контекст (09 §4) — relay єдиний володіє з'єднанням.
    tokio::spawn(relay::run(app_state.db.clone(), config.nats_url.clone(), shared_nats));

    if !spa::is_available() {
        tracing::warn!("web/dist/index.html не знайдено — віддаю лише API (зберіть фронтенд: cd web && npx vite build)");
    }

    let app = Router::new()
        .merge(api::api_router())
        .merge(chat_api::chat_router())
        .merge(personnel_api::router())
        .route("/api/admin/whatsapp/events", axum::routing::get(admin_sse::whatsapp_status_stream))
        .merge(spa::spa_router())
        .layer(axum::middleware::from_fn(security_headers))
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
        .with_state(app_state);

    tracing::info!("Вишкіл listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to the configured address");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}

async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut resp = next.run(req).await;
    let hdrs = resp.headers_mut();
    hdrs.insert("x-content-type-options", "nosniff".parse().unwrap());
    hdrs.insert("x-frame-options", "DENY".parse().unwrap());
    hdrs.insert("referrer-policy", "strict-origin-when-cross-origin".parse().unwrap());
    resp
}
