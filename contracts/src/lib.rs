//! Типи повідомлень для брокера (`docs/spec/09-messaging.md`) — спільні для `app` (продюсер
//! доменних подій), `server` (relay), `bus` (транспорт) і `services/notifier` (споживач). Кожна
//! зміна контракту — нова версія типу/subject-а (`v2`), стара лишається, поки є споживачі (§3.4).

mod envelope;
mod events;
mod notify;
pub mod subjects;

pub use envelope::Envelope;
pub use events::{DiscrepancyMetric, DiscrepancyOpened, DiscrepancyResolved};
pub use notify::{NotifySend, NotifyStatus, NotifyResult, NotifyTemplate};
