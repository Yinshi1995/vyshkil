//! Тонка обгортка над `async-nats` (09-messaging.md §4): publish з дедуплікацією, durable
//! pull-consumer, контракт ідемпотентності споживача. Не знає про Postgres/`outbox`/`inbox`-
//! таблиці конкретно — ті лишаються в `app`/`services/notifier`.

mod consumer;
mod error;
mod inbox;
mod publish;
mod stream;

pub use async_nats;
pub use consumer::{ensure_pull_consumer, ConsumerSpec};
pub use error::{BusError, BusResult};
pub use inbox::InboxStore;
pub use publish::{connect, jetstream, publish_raw_with_msg_id, publish_with_msg_id, request};
pub use stream::ensure_stream;

/// Спільний слот з'єднання (09 §4: `server::relay` єдиний володіє життєвим циклом підключення —
/// `app`'s `#[server]`-функції для `/admin/whatsapp` лише ЧИТАЮТЬ клон готового клієнта звідси
/// через контекст Leptos, самі не підключаються). Визначено тут (не дублюється в `server`/`app`
/// окремо) — обидва вже залежать від `bus`.
pub type SharedNatsClient = std::sync::Arc<std::sync::Mutex<Option<async_nats::Client>>>;
