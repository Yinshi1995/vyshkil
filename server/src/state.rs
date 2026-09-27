use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sea_orm::DatabaseConnection;

// FromRef дозволяє Axum і leptos_routes_with_context діставати LeptosOptions/DatabaseConnection
// з одного спільного стану роутера, не тримаючи два окремих .with_state() виклики.
#[derive(FromRef, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub db: DatabaseConnection,
}
