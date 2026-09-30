// Дефолт 128 не вистачає компілятору, коли цей бінарник компілюється в test-cfg (`cargo test -p
// server` — глибоко вкладені типи `tachys`-view-дерева з `app::app::{shell, App}` штовхають
// query-глибину за межу саме в test-режимі, не в звичайному `cargo build`, який завжди проходив
// без цього атрибута; знайдено при додаванні першого тесту в цьому крейті, `tests/relay.rs`).
#![recursion_limit = "256"]

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use mimalloc::MiMalloc;
use migration::MigratorTrait;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use app::app::{shell, App};
use server::{admin_sse, config, db, relay, state::AppState};

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

    // get_configuration(None) — рекомендований для cargo-leptos спосіб: налаштування (адреса, шлях сайту)
    // запікаються в бінарник на етапі збірки й переопределяються env-змінними LEPTOS_*, а не читанням Cargo.toml у рантаймі.
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let shared_nats: bus::SharedNatsClient = std::sync::Arc::new(std::sync::Mutex::new(None));
    let app_state = AppState { leptos_options, db, nats: shared_nats.clone() };

    // Перша фонова задача в цьому проєкті (09-messaging.md §3.1, Фаза 1) -- `tokio::spawn`, не
    // блокує запуск: NATS недоступний при старті — relay сам ретраїть, сервер піднімається як
    // завжди. `shared_nats` — той самий клон, що адмін-сторінка `/admin/whatsapp` читає через
    // контекст (09 §4) — relay єдиний володіє з'єднанням.
    tokio::spawn(relay::run(app_state.db.clone(), config.nats_url.clone(), shared_nats));

    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let db = app_state.db.clone();
                let nats = app_state.nats.clone();
                move || {
                    provide_context(db.clone());
                    provide_context(nats.clone());
                }
            },
            {
                let leptos_options = app_state.leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .route("/api/admin/whatsapp/events", axum::routing::get(admin_sse::whatsapp_status_stream))
        .fallback(file_and_error_handler)
        .with_state(app_state);

    tracing::info!("Taktoblik listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to the configured address");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}

// Fallback, що спершу намагається віддати статичний файл із site-root (JS/WASM/CSS з cargo-leptos),
// і лише якщо такого файлу немає — рендерить Leptos-застосунок (SPA-подібна навігація на невідомих шляхах).
// Контекст (db) провайдиться так само, як у leptos_routes_with_context — інакше будь-який
// невідомий шлях (типо/відсутній асет, бот-скан) падав з panic "no DatabaseConnection in context"
// замість того, щоб SSR коректно показав "не знайдено" через <Routes fallback>.
async fn file_and_error_handler(uri: Uri, State(state): State<AppState>, req: Request<Body>) -> Response {
    let root = state.leptos_options.site_root.clone();

    match get_static_file(uri, root.as_ref()).await {
        Ok(res) if res.status() == StatusCode::OK => res,
        _ => {
            let db = state.db.clone();
            let nats = state.nats.clone();
            leptos_axum::render_app_to_stream_with_context(
                move || {
                    provide_context(db.clone());
                    provide_context(nats.clone());
                },
                move || shell(state.leptos_options.clone()),
            )(req)
            .await
            .into_response()
        }
    }
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response, (StatusCode, String)> {
    let req = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("failed to build a static-file request");

    ServeDir::new(root)
        .oneshot(req)
        .await
        .map(IntoResponse::into_response)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("static file error: {e}")))
}
