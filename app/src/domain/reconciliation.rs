//! Горизонтальна звірка (04 §3, Етап 8 зріз 1 — `.claude/decisions/
//! etap8-horizontal-reconciliation-first-slice.md`): та сама канонічна `training_group`, різні
//! подання — розходяться кількості/терміни/місце. Чиста логіка (WASM-безпечна): порівнює вже
//! зібрані значення, нічого не читає з БД сама (`backend::repo::reconciliation` збирає
//! `ReportedValues` з `reported_group` і викликає це).
//!
//! Одна розбіжність = один незгодний МЕТРИК (не весь рядок одразу) — той самий принцип, що
//! `discrepancy.metric` у спеці (04 §4: одне поле `metric`, не список) — "прибуло" й "місце"
//! можуть розходитись незалежно одне від одного, кожне — окремий рядок `discrepancy`.

/// Одне подання, зведене до полів, що звіряються (з `reported_group`, 04 §2).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportedValues {
    pub submission_id: i32,
    pub planned_count: i64,
    pub arrived_count: i64,
    pub in_training_count: i64,
    /// `YYYY-MM-DD` — лексикографічне порівняння, той самий підхід, що `domain::counting`.
    pub planned_start: String,
    pub planned_end: String,
    pub site_id: i32,
}

/// Незгодний метрик — значення підписані `submission_id` (04 §4: "хто що сказав").
#[derive(Debug, Clone, PartialEq)]
pub struct HorizontalDiscrepancy {
    pub metric: &'static str,
    pub values: Vec<(i32, String)>,
}

const METRICS: &[&str] =
    &["planned_count", "arrived_count", "in_training_count", "planned_start", "planned_end", "site_id"];

fn metric_value(r: &ReportedValues, metric: &str) -> String {
    match metric {
        "planned_count" => r.planned_count.to_string(),
        "arrived_count" => r.arrived_count.to_string(),
        "in_training_count" => r.in_training_count.to_string(),
        "planned_start" => r.planned_start.clone(),
        "planned_end" => r.planned_end.clone(),
        "site_id" => r.site_id.to_string(),
        _ => unreachable!("невідомий метрик «{metric}» — див. METRICS"),
    }
}

/// Порівнює ВСІ подання, зіставлені з тією самою канонічною групою. Менш ніж 2 подання —
/// нема з чим звіряти (порожньо). Метрик потрапляє в результат, лише якщо серед подань є
/// ХОЧА Б ДВА РІЗНІ значення — жодного допуску (толерантності) тут нема: це вже той самий
/// group_id, а не пошук кандидата (там допуск ±3 дні, тут — точна незгода).
pub fn detect_horizontal(reports: &[ReportedValues]) -> Vec<HorizontalDiscrepancy> {
    if reports.len() < 2 {
        return Vec::new();
    }

    METRICS
        .iter()
        .filter_map(|&metric| {
            let mut values: Vec<(i32, String)> =
                reports.iter().map(|r| (r.submission_id, metric_value(r, metric))).collect();
            let distinct = {
                let mut v: Vec<&String> = values.iter().map(|(_, v)| v).collect();
                v.sort();
                v.dedup();
                v.len()
            };
            if distinct < 2 {
                return None;
            }
            values.sort_by_key(|(sid, _)| *sid);
            Some(HorizontalDiscrepancy { metric, values })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(submission_id: i32, planned: i64, arrived: i64, in_training: i64, site_id: i32) -> ReportedValues {
        ReportedValues {
            submission_id,
            planned_count: planned,
            arrived_count: arrived,
            in_training_count: in_training,
            planned_start: "2026-08-18".to_string(),
            planned_end: "2026-10-09".to_string(),
            site_id,
        }
    }

    #[test]
    fn single_report_has_nothing_to_compare() {
        assert_eq!(detect_horizontal(&[r(1, 20, 18, 15, 5)]), Vec::new());
    }

    #[test]
    fn identical_reports_have_no_discrepancy() {
        assert_eq!(detect_horizontal(&[r(1, 20, 18, 15, 5), r(2, 20, 18, 15, 5)]), Vec::new());
    }

    #[test]
    fn differing_arrived_count_is_flagged_with_both_values() {
        let d = detect_horizontal(&[r(1, 20, 18, 15, 5), r(2, 20, 20, 15, 5)]);
        assert_eq!(
            d,
            vec![HorizontalDiscrepancy {
                metric: "arrived_count",
                values: vec![(1, "18".to_string()), (2, "20".to_string())],
            }]
        );
    }

    #[test]
    fn multiple_disagreeing_metrics_each_get_own_row() {
        let mut b = r(2, 20, 20, 15, 5);
        b.site_id = 9;
        let d = detect_horizontal(&[r(1, 20, 18, 15, 5), b]);
        let metrics: Vec<&str> = d.iter().map(|x| x.metric).collect();
        assert_eq!(metrics, vec!["arrived_count", "site_id"]);
    }

    #[test]
    fn three_reports_two_agreeing_one_off_still_flags() {
        // Дві сесії подали "18", одна — "20" — усе одно розбіжність (не голосування більшістю,
        // 04 §3 не згадує кворум — будь-яка незгода видима людині).
        let d = detect_horizontal(&[r(1, 20, 18, 15, 5), r(2, 20, 18, 15, 5), r(3, 20, 20, 15, 5)]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].metric, "arrived_count");
    }

    #[test]
    fn third_report_agreeing_with_majority_does_not_remove_the_minority_disagreement() {
        // detect_horizontal сама не "автозакриває" — це робить repo-шар, коли ЗАМІНЮЄ множину
        // звірюваних подань (напр. остання версія на джерело). Тут лише перевіряємо стабільність:
        // порядок подань не впливає на результат.
        let a = detect_horizontal(&[r(1, 20, 18, 15, 5), r(2, 20, 20, 15, 5)]);
        let b = detect_horizontal(&[r(2, 20, 20, 15, 5), r(1, 20, 18, 15, 5)]);
        assert_eq!(a, b);
    }
}
