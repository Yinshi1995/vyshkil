//! Воронка групи, порахована з подій (01 §3): `planned → arrived → started (+added) →
//! (−attrition) → completed → vos_awarded/vos_not_awarded`. Кількості не зберігаються полями —
//! лише подіями (`group_event`), тому вся арифметика — тут, чиста, без SQL/БД (WASM-безпечна),
//! на списку подій, які репозиторій просто вибирає з таблиці.
//!
//! Дати — `String` у форматі `YYYY-MM-DD` (сортуються лексикографічно так само, як хронологічно) —
//! той самий підхід, що й в `backend::repo::orgs::org_detail` (`to_char(..., 'YYYY-MM-DD')`),
//! щоб не тягнути `chrono` в WASM-безпечний `domain/` заради самого лише порівняння дат.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Planned,
    Arrived,
    Started,
    Added,
    Attrition,
    Completed,
    VosAwarded,
    VosNotAwarded,
    Correction,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Planned => "planned",
            EventType::Arrived => "arrived",
            EventType::Started => "started",
            EventType::Added => "added",
            EventType::Attrition => "attrition",
            EventType::Completed => "completed",
            EventType::VosAwarded => "vos_awarded",
            EventType::VosNotAwarded => "vos_not_awarded",
            EventType::Correction => "correction",
        }
    }

    pub fn parse(s: &str) -> Option<EventType> {
        match s {
            "planned" => Some(EventType::Planned),
            "arrived" => Some(EventType::Arrived),
            "started" => Some(EventType::Started),
            "added" => Some(EventType::Added),
            "attrition" => Some(EventType::Attrition),
            "completed" => Some(EventType::Completed),
            "vos_awarded" => Some(EventType::VosAwarded),
            "vos_not_awarded" => Some(EventType::VosNotAwarded),
            "correction" => Some(EventType::Correction),
            _ => None,
        }
    }
}

/// Одна подія групи — те, що репозиторій читає з `group_event`. `count` — зі знаком (значущий
/// лише для `Correction`, для решти типів БД гарантує `count > 0` через CHECK).
#[derive(Debug, Clone)]
pub struct GroupEventRecord {
    pub event_type: EventType,
    pub count: i32,
    /// Коли сталося (`YYYY-MM-DD`).
    pub occurred_on: String,
    /// Коли дізналися (`YYYY-MM-DD HH:MI:SS`, бітемпоральність — 01 §3).
    pub recorded_at: String,
}

/// `in_training(D)` — скільки зараз навчається в групі, за подіями з `occurred_on ≤ as_of`
/// (і, якщо задано `known_at`, лише подіями, про які вже знали на той момент — "як ми знали на
/// момент T", 01 §3). `= started + added − attrition − completed ± correction`.
pub fn in_training(events: &[GroupEventRecord], as_of: &str, known_at: Option<&str>) -> i64 {
    events
        .iter()
        .filter(|e| e.occurred_on.as_str() <= as_of)
        .filter(|e| known_at.is_none_or(|t| e.recorded_at.as_str() <= t))
        .map(|e| match e.event_type {
            EventType::Started | EventType::Added => i64::from(e.count),
            EventType::Attrition | EventType::Completed => -i64::from(e.count),
            EventType::Correction => i64::from(e.count),
            EventType::Planned
            | EventType::Arrived
            | EventType::VosAwarded
            | EventType::VosNotAwarded => 0,
        })
        .sum()
}

/// Сума подій конкретного типу рівно на дату `date` (`started_on`/`completed_on`/`attrition_on`
/// з 01 §3 — усі одна й та сама форма запиту, тільки різний `event_type`).
pub fn events_on(
    events: &[GroupEventRecord],
    event_type: EventType,
    date: &str,
    known_at: Option<&str>,
) -> i64 {
    events
        .iter()
        .filter(|e| e.event_type == event_type && e.occurred_on == date)
        .filter(|e| known_at.is_none_or(|t| e.recorded_at.as_str() <= t))
        .map(|e| i64::from(e.count))
        .sum()
}

/// "Закінчують сьогодні" (01 §3): фактична подія `completed` на `date`, а якщо її ще нема —
/// прогноз за `planned_end групи = date` (позначається окремо, не змішується з фактом).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finishing {
    /// Фактично завершили (кількість — сума `completed`-подій на цю дату).
    Actual(i64),
    /// Подій ще нема, але `planned_end` групи збігається з датою — прогноз, не факт.
    Forecast,
    /// Ні факту, ні прогнозу на цю дату.
    None,
}

pub fn finishing_on(
    events: &[GroupEventRecord],
    planned_end: &str,
    date: &str,
    known_at: Option<&str>,
) -> Finishing {
    let actual = events_on(events, EventType::Completed, date, known_at);
    if actual > 0 {
        Finishing::Actual(actual)
    } else if planned_end == date {
        Finishing::Forecast
    } else {
        Finishing::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(event_type: EventType, count: i32, occurred_on: &str, recorded_at: &str) -> GroupEventRecord {
        GroupEventRecord {
            event_type,
            count,
            occurred_on: occurred_on.to_string(),
            recorded_at: recorded_at.to_string(),
        }
    }

    /// Приклад воронки з 01 §3: started 20 (05.03), додались 5 (10.03), вибуло 3 (15.03),
    /// завершили 12 (01.04) — лишається 20+5-3-12 = 10 у навчанні на кінець періоду.
    fn sample_funnel() -> Vec<GroupEventRecord> {
        vec![
            ev(EventType::Started, 20, "2026-03-05", "2026-03-05T10:00:00"),
            ev(EventType::Added, 5, "2026-03-10", "2026-03-10T10:00:00"),
            ev(EventType::Attrition, 3, "2026-03-15", "2026-03-15T10:00:00"),
            ev(EventType::Completed, 12, "2026-04-01", "2026-04-01T10:00:00"),
        ]
    }

    #[test]
    fn in_training_accumulates_through_funnel() {
        let events = sample_funnel();
        assert_eq!(in_training(&events, "2026-03-04", None), 0, "до старту — нуль");
        assert_eq!(in_training(&events, "2026-03-05", None), 20, "одразу після started");
        assert_eq!(in_training(&events, "2026-03-10", None), 25, "+5 added");
        assert_eq!(in_training(&events, "2026-03-15", None), 22, "-3 attrition");
        assert_eq!(in_training(&events, "2026-04-01", None), 10, "-12 completed, лишилось 10");
    }

    #[test]
    fn events_on_matches_exact_date_only() {
        let events = sample_funnel();
        assert_eq!(events_on(&events, EventType::Started, "2026-03-05", None), 20);
        assert_eq!(events_on(&events, EventType::Started, "2026-03-06", None), 0, "не в цей день");
        assert_eq!(events_on(&events, EventType::Attrition, "2026-03-15", None), 3);
    }

    #[test]
    fn correction_flips_sign_and_others_stay_non_negative_by_db_check() {
        let mut events = sample_funnel();
        // виправлення помилки внесення/задвоєння (01 §3) — від'ємна корекція.
        events.push(ev(EventType::Correction, -2, "2026-03-20", "2026-03-20T10:00:00"));
        assert_eq!(in_training(&events, "2026-03-20", None), 20, "22 - 2 (виправлення задвоєння)");
    }

    #[test]
    fn finishing_on_prefers_actual_over_forecast() {
        let events = sample_funnel();
        assert_eq!(
            finishing_on(&events, "2026-04-01", "2026-04-01", None),
            Finishing::Actual(12),
            "є факт completed — не прогноз"
        );
        // Інша дата planned_end, події ще немає -> прогноз.
        let no_actual_yet: Vec<GroupEventRecord> = sample_funnel()
            .into_iter()
            .filter(|e| e.event_type != EventType::Completed)
            .collect();
        assert_eq!(
            finishing_on(&no_actual_yet, "2026-04-15", "2026-04-15", None),
            Finishing::Forecast,
            "planned_end настав, факту ще нема — прогноз"
        );
        assert_eq!(
            finishing_on(&no_actual_yet, "2026-04-15", "2026-04-14", None),
            Finishing::None,
            "не той день — нічого"
        );
    }

    /// Бітемпоральний тест: "як ми знали на момент T" (01 §3) — той самий факт, різні `known_at`
    /// дають різну картину, бо `attrition`-подію про 15.03 внесли в систему лише 20.03.
    #[test]
    fn known_at_reproduces_past_belief() {
        let events = vec![
            ev(EventType::Started, 20, "2026-03-05", "2026-03-05T09:00:00"),
            // сталось 15.03, але внесли (recorded_at) лише 20.03 — типова затримка подання.
            ev(EventType::Attrition, 3, "2026-03-15", "2026-03-20T09:00:00"),
        ];

        // На 18.03 (до того, як про вибуття дізнались) документ показував би 20 у навчанні.
        assert_eq!(
            in_training(&events, "2026-03-18", Some("2026-03-18T00:00:00")),
            20,
            "на момент T=18.03 ще не знали про attrition від 15.03"
        );
        // Сьогоднішній погляд (без обмеження known_at) — уже видно вибуття.
        assert_eq!(in_training(&events, "2026-03-18", None), 17, "зараз бачимо і attrition теж");
        // На 21.03 (після того, як внесли) — той самий D=18.03 показав би вже коректні 17.
        assert_eq!(
            in_training(&events, "2026-03-18", Some("2026-03-21T00:00:00")),
            17,
            "на момент T=21.03 вже знали про attrition від 15.03"
        );
    }
}
