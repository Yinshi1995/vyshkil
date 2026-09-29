//! DTO укомплектованості (01 §4, Етап 5 — КВід/ІВС). Простіше за `submission::GroupFormRow`:
//! немає ВОС/дат/воронки, лише організація + кілька чисел; тому не переюзаємо `widgets::group_grid`
//! (інша форма даних) — окремий, простіший превʼю-компонент у `pages/import`.
//!
//! КВід (`StaffingRow`, `category='squad_leaders'`) і ІВС (`InstructorStaffingRow`,
//! `category='instructors'`) — РІЗНІ набори метрик (01 §4: ІВС має `trained_kibr`, немає
//! `present`/`in_training`/`planned_next_month`/`need_training`) — окремі типи, не один із
//! опційними полями: вставляти "0" за метрику, якої файл не звітує, було б вигаданим значенням
//! (той самий принцип, що й "чесний плейсхолдер" для `vos.title` у Етапі 2).

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstructorStaffingRow {
    pub org_id: Option<i32>,
    pub org_label: String,
    pub by_tos: i64,
    pub by_list: i64,
    pub trained_sergeant: i64,
    pub trained_kibr: i64,
}
