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
        _ => unreachable!("невідомий метрик \"{metric}\" — див. METRICS"),
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

// ---------------------------------------------------------------------------
// Вертикальна звірка (04 §3): сума підлеглих проти зведеного подання органу
// ---------------------------------------------------------------------------

/// Агрегат по одному виду підготовки: `(total_count, kind_label)`.
/// `source_label` — хто це сказав: "підлеглі (сума)" або "зведене подання 17 АК".
#[derive(Debug, Clone, PartialEq)]
pub struct VerticalSide {
    pub label: String,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalDiscrepancy {
    pub metric: &'static str,
    pub parent: VerticalSide,
    pub children_sum: VerticalSide,
}

type MetricAccessor = (&'static str, fn(&AggregatedCounts) -> i64);

/// Порівнює зведене подання органу (parent) із сумою по підлеглих (children).
/// Кожен рядок — один вид лічильника (`total`/`finishing`/`started`). Розбіжність виникає, якщо
/// значення відрізняються (без толерантності — 04 §3, так само, як горизонтальна).
pub fn detect_vertical(
    parent_label: &str,
    parent: &AggregatedCounts,
    children_label: &str,
    children: &AggregatedCounts,
) -> Vec<VerticalDiscrepancy> {
    const METRICS: &[MetricAccessor] = &[
        ("total", |a| a.total),
        ("finishing", |a| a.finishing),
        ("started", |a| a.started),
    ];

    METRICS
        .iter()
        .filter_map(|&(metric, f)| {
            let pv = f(parent);
            let cv = f(children);
            if pv == cv {
                return None;
            }
            Some(VerticalDiscrepancy {
                metric,
                parent: VerticalSide { label: parent_label.to_string(), total: pv },
                children_sum: VerticalSide { label: children_label.to_string(), total: cv },
            })
        })
        .collect()
}

/// Агреговані лічильники для одного органу/виду підготовки/дати — підсумок, що звіряється
/// вертикально. Не прив'язаний до `submission_id` (на відміну від `ReportedValues` для
/// горизонтальної) — тут порівнюється СУМА, а не конкретне подання.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregatedCounts {
    pub total: i64,
    pub finishing: i64,
    pub started: i64,
}

impl AggregatedCounts {
    pub fn add(&mut self, other: &AggregatedCounts) {
        self.total += other.total;
        self.finishing += other.finishing;
        self.started += other.started;
    }
}

// ---------------------------------------------------------------------------
// Часова звірка (04 §3): нове подання суперечить попередньому без пояснення
// ---------------------------------------------------------------------------

/// Два знімки одного й того ж звіту (від одного джерела, по одній групі) у різний час.
#[derive(Debug, Clone, PartialEq)]
pub struct TemporalSnapshot {
    pub submission_id: i32,
    pub as_of: String,
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporalDiscrepancy {
    pub metric: &'static str,
    pub previous: TemporalSnapshot,
    pub current: TemporalSnapshot,
    pub explained_delta: i64,
}

/// Порівнює два послідовні подання одного джерела для однієї групи: якщо `total` зменшився, а
/// кількість пояснюючих подій (вибуття/завершення/корекція) не покриває різницю — розбіжність
/// "зміна без пояснення" (04 §3: "вчора 12, сьогодні 9, а подій вибуття нема").
///
/// `explained_delta` — сума `attrition + completed` подій між двома знімками (від'ємне, бо
/// зменшують кількість). Якщо фактичне зменшення більше за пояснене — розбіжність.
/// Збільшення total завжди пропускається (нових людей можуть додати без окремої "пояснюючої" події).
pub fn detect_temporal(
    previous: &TemporalSnapshot,
    current: &TemporalSnapshot,
    explained_decrease: i64,
) -> Option<TemporalDiscrepancy> {
    let delta = current.total - previous.total;
    if delta >= 0 {
        return None;
    }
    let unexplained = (-delta) - explained_decrease;
    if unexplained <= 0 {
        return None;
    }
    Some(TemporalDiscrepancy {
        metric: "total",
        previous: previous.clone(),
        current: current.clone(),
        explained_delta: explained_decrease,
    })
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

    // --- Vertical reconciliation ---

    #[test]
    fn vertical_equal_counts_no_discrepancy() {
        let parent = AggregatedCounts { total: 100, finishing: 5, started: 10 };
        let children = AggregatedCounts { total: 100, finishing: 5, started: 10 };
        assert_eq!(detect_vertical("17 АК", &parent, "підлеглі (сума)", &children), vec![]);
    }

    #[test]
    fn vertical_differing_total_flagged() {
        let parent = AggregatedCounts { total: 100, finishing: 5, started: 10 };
        let children = AggregatedCounts { total: 95, finishing: 5, started: 10 };
        let d = detect_vertical("17 АК", &parent, "підлеглі (сума)", &children);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].metric, "total");
        assert_eq!(d[0].parent.total, 100);
        assert_eq!(d[0].children_sum.total, 95);
    }

    #[test]
    fn vertical_multiple_metrics_each_get_own_row() {
        let parent = AggregatedCounts { total: 100, finishing: 5, started: 10 };
        let children = AggregatedCounts { total: 95, finishing: 8, started: 10 };
        let d = detect_vertical("17 АК", &parent, "підлеглі", &children);
        let metrics: Vec<&str> = d.iter().map(|x| x.metric).collect();
        assert_eq!(metrics, vec!["total", "finishing"]);
    }

    // --- Temporal reconciliation ---

    #[test]
    fn temporal_no_change_no_discrepancy() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 12 };
        assert_eq!(detect_temporal(&prev, &curr, 0), None);
    }

    #[test]
    fn temporal_increase_is_fine() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 15 };
        assert_eq!(detect_temporal(&prev, &curr, 0), None);
    }

    #[test]
    fn temporal_decrease_without_explanation_is_discrepancy() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 9 };
        let d = detect_temporal(&prev, &curr, 0).unwrap();
        assert_eq!(d.metric, "total");
        assert_eq!(d.previous.total, 12);
        assert_eq!(d.current.total, 9);
        assert_eq!(d.explained_delta, 0);
    }

    #[test]
    fn temporal_decrease_fully_explained_by_attrition_is_fine() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 9 };
        assert_eq!(detect_temporal(&prev, &curr, 3), None);
    }

    #[test]
    fn temporal_decrease_partially_explained_is_discrepancy() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 9 };
        let d = detect_temporal(&prev, &curr, 1).unwrap();
        assert_eq!(d.explained_delta, 1);
    }

    #[test]
    fn temporal_decrease_over_explained_is_fine() {
        let prev = TemporalSnapshot { submission_id: 1, as_of: "2026-09-25".into(), total: 12 };
        let curr = TemporalSnapshot { submission_id: 2, as_of: "2026-09-26".into(), total: 9 };
        assert_eq!(detect_temporal(&prev, &curr, 5), None);
    }
}
