//! Іменування subject-ів (09-messaging.md §3.3): `vyshkil.<контекст>.<подія|команда>.v<N>`.
//! Константи тут, не рядкові літерали на місці виклику — одне місце для перейменування/`v2`.

pub const NOTIFY_SEND_V1: &str = "vyshkil.notify.send.v1";
pub const NOTIFY_RESULT_V1: &str = "vyshkil.notify.result.v1";

pub const DISCREPANCY_OPENED_V1: &str = "vyshkil.discrepancy.opened.v1";
pub const DISCREPANCY_RESOLVED_V1: &str = "vyshkil.discrepancy.resolved.v1";

/// Префікс мертвих листів (`vyshkil.dlq.>`) — конкретний subject додає контекст споживача,
/// що вичерпав `max_deliver` (наприклад, `vyshkil.dlq.notify.send.v1`).
pub const DLQ_PREFIX: &str = "vyshkil.dlq.";

/// Назви стрімів JetStream (§3.3 таблиця) — окремо від subject-ів: один стрім часто приймає
/// кілька subject-патернів (наприклад, `EVENTS` — усі майбутні доменні події).
pub mod stream {
    pub const NOTIFY_CMD: &str = "NOTIFY_CMD";
    pub const NOTIFY_RESULT: &str = "NOTIFY_RESULT";
    pub const EVENTS: &str = "EVENTS";
    pub const DLQ: &str = "DLQ";
}
