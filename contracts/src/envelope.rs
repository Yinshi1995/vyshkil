//! Конверт повідомлення (09-messaging.md §3.4) — однакова обгортка для команд і подій.
//! `id` (Uuid v7 — сортований у часі) відрізняється від дедуплікаційного ключа (`Nats-Msg-Id`
//! заголовка при публікації, §3.1) — той контролює продюсер (наприклад, `outbox.id`), тоді як
//! `id` тут — це ідентичність САМОГО конверта, не ключ ідемпотентності.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub struct Envelope<T> {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_: String,
    pub version: u16,
    pub occurred_at: DateTime<Utc>,
    pub producer: String,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub payload: T,
}

impl<T> Envelope<T> {
    /// Новий конверт для щойно створеної події/команди — `id`/`occurred_at` генеруються тут,
    /// не приймаються ззовні, щоб продюсер не міг випадково підробити час/ідентичність конверта.
    pub fn new(
        type_: impl Into<String>,
        version: u16,
        producer: impl Into<String>,
        correlation_id: Uuid,
        causation_id: Option<Uuid>,
        payload: T,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            type_: type_.into(),
            version,
            occurred_at: Utc::now(),
            producer: producer.into(),
            correlation_id,
            causation_id,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        n: i32,
    }

    #[test]
    fn round_trips_through_json() {
        let env = Envelope::new(
            "test.event.v1",
            1,
            "test-producer",
            Uuid::now_v7(),
            None,
            Payload { n: 42 },
        );
        let json = serde_json::to_string(&env).expect("серіалізація");
        let back: Envelope<Payload> = serde_json::from_str(&json).expect("десеріалізація");
        assert_eq!(back.payload, env.payload);
        assert_eq!(back.type_, "test.event.v1");
        assert_eq!(back.version, 1);
    }

    /// Сумісність версій (09 §3.4: "Споживач ігнорує невідомі поля"): конверт від НОВІШОГО
    /// продюсера, що додав поле (і в payload, і на верхньому рівні самого Envelope), МАЄ
    /// десеріалізуватись старим споживачем без помилки — інакше додавання поля стало б breaking
    /// change, а не `v2`-сумісним розширенням. Жоден тип тут не має
    /// `#[serde(deny_unknown_fields)]` — цей тест ловить, якщо хтось його колись додасть.
    #[test]
    fn unknown_fields_from_a_newer_producer_are_ignored_not_rejected() {
        let json = serde_json::json!({
            "id": Uuid::now_v7().to_string(),
            "type": "test.event.v1",
            "version": 1,
            "occurred_at": Utc::now().to_rfc3339(),
            "producer": "future-producer",
            "correlation_id": Uuid::now_v7().to_string(),
            "causation_id": null,
            "future_envelope_field": "щось, чого стара версія ще не знає",
            "payload": {
                "n": 7,
                "future_payload_field": { "nested": true },
            },
        })
        .to_string();

        let back: Envelope<Payload> =
            serde_json::from_str(&json).expect("невідомі поля НЕ мають ламати десеріалізацію");
        assert_eq!(back.payload, Payload { n: 7 });
    }
}
