use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

use crate::config::Config;

pub async fn connect(config: &Config) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(&config.database_url);
    opt
        // Максимум = database_max_connections (5-10 за замовчуванням): на слабкому сервері дешевше
        // трохи почекати в черзі за з'єднанням, ніж витратити RAM на десятки простих conn-ів.
        .max_connections(config.database_max_connections)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(300))
        // SQLx-логування кожного запиту в проді — зайвий I/O і шум; вмикається окремо через RUST_LOG для дебагу.
        .sqlx_logging(false);

    Database::connect(opt).await
}
