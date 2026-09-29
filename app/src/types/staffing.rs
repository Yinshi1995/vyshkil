//! DTO укомплектованості (01 §4, Етап 5 — КВід/ІВС). Простіше за `submission::GroupFormRow`:
//! немає ВОС/дат/воронки, лише організація + сім чисел; тому не переюзаємо `widgets::group_grid`
//! (інша форма даних) — окремий, простіший превʼю-компонент у `pages/import`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaffingRow {
    pub org_id: Option<i32>,
    pub org_label: String,
    pub by_tos: i64,
    pub by_list: i64,
    pub present: i64,
    pub trained_sergeant: i64,
    pub in_training: i64,
    pub planned_next_month: i64,
    pub need_training: i64,
}
