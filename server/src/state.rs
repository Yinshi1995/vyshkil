use std::sync::Arc;

use axum::extract::FromRef;
use bus::SharedNatsClient;
use leptos::prelude::LeptosOptions;
use sea_orm::DatabaseConnection;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatBroadcastMsg {
    pub room_id: i32,
    pub message_id: i32,
}

pub type ChatTx = Arc<broadcast::Sender<ChatBroadcastMsg>>;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub db: DatabaseConnection,
    pub nats: SharedNatsClient,
    pub chat_tx: ChatTx,
}
