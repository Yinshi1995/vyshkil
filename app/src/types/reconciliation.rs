//! DTO для екрана розбіжностей (04 §4, Етап 8 зріз 1).

use serde::{Deserialize, Serialize};

/// Одна розбіжність — рядок для `/discrepancies`, уже з контекстом (назва частини/групи), щоб
/// сторінці не довелось окремо резолвити id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyRow {
    pub id: i32,
    pub kind: String,
    pub org_id: i32,
    pub org_label: String,
    pub group_id: Option<i32>,
    /// Короткий підпис канонічної групи ("Фахова · ВОС 218 · з 18.08.2026") — `None`, якщо
    /// `group_id` немає (майбутні kind без прив'язки до однієї групи, 04 §4).
    pub group_label: Option<String>,
    pub as_of: String,
    pub metric: String,
    /// Українська підпис метрика для інтерфейсу ("Прибуло", не "arrived_count") —
    /// `domain::reconciliation::METRICS`-ключ → підпис, мапиться на сервері (`repo`), не тут:
    /// `types/` — лише дані, без логіки підпису.
    pub metric_label: String,
    /// `(submission_id, значення)` — "хто що сказав" (04 §4).
    pub values: Vec<(i32, String)>,
    pub status: String,
    pub created_at: String,
}
