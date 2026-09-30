//! Доменні події (09-messaging.md §3.3, subject `vyshkil.discrepancy.*.v1`) — публікує `app`
//! (relay з outbox, Фаза 1), поки що ніхто не споживає (майбутні аналітика/аудит — сам §3.3 це
//! називає). `metric` — той самий `enum`, що рядкові значення в `discrepancy.metric`
//! (`app::backend::repo::reconciliation::metric_label`), не вільний текст.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub enum DiscrepancyMetric {
    PlannedCount,
    ArrivedCount,
    InTrainingCount,
    PlannedStart,
    PlannedEnd,
    SiteId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub struct DiscrepancyOpened {
    pub discrepancy_id: i32,
    pub org_id: i32,
    pub group_id: Option<i32>,
    pub metric: DiscrepancyMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema-gen", derive(schemars::JsonSchema))]
pub struct DiscrepancyResolved {
    pub discrepancy_id: i32,
    pub org_id: i32,
    pub group_id: Option<i32>,
    pub metric: DiscrepancyMetric,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrepancy_opened_round_trips() {
        let ev = DiscrepancyOpened {
            discrepancy_id: 7,
            org_id: 42,
            group_id: Some(3),
            metric: DiscrepancyMetric::ArrivedCount,
        };
        let json = serde_json::to_string(&ev).expect("серіалізація");
        let back: DiscrepancyOpened = serde_json::from_str(&json).expect("десеріалізація");
        assert_eq!(back.discrepancy_id, ev.discrepancy_id);
        assert_eq!(back.metric, ev.metric);
    }
}
