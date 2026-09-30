use std::env;

// Мінімальна конфігурація з .env / env-змінних — без окремого crate типу `config`,
// бо параметрів мало і вони пласкі; ускладнення тут не окупиться.
pub struct Config {
    pub database_url: String,
    pub database_max_connections: u32,
    pub worker_threads: usize,
    pub log_level: String,
    pub nats_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        // .ok() навмисно: у продакшені змінні середовища зазвичай інжектяться оркестратором,
        // а не файлом .env, тому відсутність файлу — це нормальний, а не аварійний випадок.
        dotenvy::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set (see .env.example)"),
            database_max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                // 5 — свідомо мале число: слабкий сервер, кожне з'єднання Postgres важить кілька МБ RAM.
                .unwrap_or(5),
            worker_threads: env::var("SERVER_WORKER_THREADS")
                .ok()
                .and_then(|v| v.parse().ok())
                // 2 воркер-потоки достатньо для типового 1-2 vCPU VPS; більше — то марно виділена, але не використана пам'ять на стеки потоків.
                .unwrap_or(2),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".to_string()),
            // Дефолт — локальний NATS без docker-compose (порт мапиться 1:1, той самий, що й
            // ручний `docker run ... -p 4222:4222 nats:2-alpine -js` у MEMORY.md). Relay сам не
            // падає, якщо тут щось недосяжне (§3.1) — це не обов'язковий для старту сервіс.
            nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string()),
        }
    }
}
