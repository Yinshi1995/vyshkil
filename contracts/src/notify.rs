//! Команда "доставити сповіщення" + результат доставки (09-messaging.md §3.3, §3.5).
//!
//! **Приватність вбудована в тип, не в перевірку на виклику**: `NotifySend`/`NotifyResult` не
//! мають жодного поля вільного тексту — лише ідентифікатори й `enum`. `dedupe_key` — виняток:
//! це ключ ідемпотентності/групування (наприклад, "org:42:discrepancy"), який будує сам продюсер
//! із власних ідентифікаторів, не текст для читання людиною — allowlisted у тесті нижче явно,
//! а не мовчки.

use serde::{Deserialize, Serialize};

/// Шаблон сповіщення — сам текст (знеособлений, 04 §5) рендерить `notifier` за цим варіантом,
/// не отримує його рядком через брокер.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub enum NotifyTemplate {
    DiscrepancyDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub struct NotifySend {
    pub recipient_org_id: i64,
    pub template: NotifyTemplate,
    pub dedupe_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub enum NotifyStatus {
    Delivered,
    Failed,
    Suppressed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub struct NotifyResult {
    pub recipient_org_id: i64,
    pub status: NotifyStatus,
    pub dedupe_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Рекурсивно перевіряє, що кожне рядкове ЛИСТОВЕ значення в JSON належить полю з
    /// `allowed_keys` — регресійний тест на "хтось мовчки додав НОВЕ поле" (09-messaging.md §3.5).
    /// `serde(rename_all = "snake_case")]`-enum'и (`template`/`status`) теж серіалізуються як
    /// JSON-рядки — вони тут в allowlist НАВМИСНО (їхня "текстовість" обмежена закритим набором
    /// варіантів у цьому ж файлі, не довільна) razóm із `dedupe_key` (справжній вільний рядок,
    /// але це ключ ідемпотентності, не контент для читання людиною). Це не автоматичний
    /// компілятор-рівня type-check (Rust не має рефлексії полів), а фіксація ОЧІКУВАНОЇ форми:
    /// нове поле, доданого без оновлення цього списку, провалить тест і змусить свідомо
    /// вирішити, чи воно теж навмисний виняток.
    fn assert_only_allowed_string_leaves(value: &Value, allowed_keys: &[&str], path: &str) {
        match value {
            Value::Object(map) => {
                for (key, v) in map {
                    let child_path = format!("{path}.{key}");
                    if let Value::String(_) = v {
                        assert!(
                            allowed_keys.contains(&key.as_str()),
                            "неочікуване рядкове поле {child_path} — notify.* не має мати вільний текст (09 §3.5), \
                             якщо це навмисно — додай ключ у allowed_keys цього тесту"
                        );
                    } else {
                        assert_only_allowed_string_leaves(v, allowed_keys, &child_path);
                    }
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    assert_only_allowed_string_leaves(v, allowed_keys, &format!("{path}[{i}]"));
                }
            }
            _ => {}
        }
    }

    #[test]
    fn notify_send_has_no_free_text_fields() {
        let cmd = NotifySend {
            recipient_org_id: 42,
            template: NotifyTemplate::DiscrepancyDetected,
            dedupe_key: "org:42:discrepancy".to_string(),
        };
        let json = serde_json::to_value(&cmd).expect("серіалізація");
        assert_only_allowed_string_leaves(&json, &["dedupe_key", "template"], "NotifySend");
    }

    #[test]
    fn notify_result_has_no_free_text_fields() {
        let result = NotifyResult {
            recipient_org_id: 42,
            status: NotifyStatus::Delivered,
            dedupe_key: "org:42:discrepancy".to_string(),
        };
        let json = serde_json::to_value(&result).expect("серіалізація");
        assert_only_allowed_string_leaves(&json, &["dedupe_key", "status"], "NotifyResult");
    }

    #[test]
    fn round_trips_through_json() {
        let cmd = NotifySend {
            recipient_org_id: 42,
            template: NotifyTemplate::DiscrepancyDetected,
            dedupe_key: "org:42:discrepancy".to_string(),
        };
        let json = serde_json::to_string(&cmd).expect("серіалізація");
        let back: NotifySend = serde_json::from_str(&json).expect("десеріалізація");
        assert_eq!(back.recipient_org_id, cmd.recipient_org_id);
        assert_eq!(back.template, cmd.template);
    }
}
