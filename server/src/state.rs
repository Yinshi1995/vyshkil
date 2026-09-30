use axum::extract::FromRef;
use bus::SharedNatsClient;
use leptos::prelude::LeptosOptions;
use sea_orm::DatabaseConnection;

// FromRef дозволяє Axum і leptos_routes_with_context діставати LeptosOptions/DatabaseConnection
// з одного спільного стану роутера, не тримаючи два окремих .with_state() виклики.
#[derive(FromRef, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub db: DatabaseConnection,
    // Адмін-сторінка `/admin/whatsapp` (09 §4) читає готовий клієнт звідси -- підключення й
    // перепідключення веде `relay` (`bus::SharedNatsClient`, той самий клон, що передається
    // йому в `tokio::spawn`).
    pub nats: SharedNatsClient,
}
