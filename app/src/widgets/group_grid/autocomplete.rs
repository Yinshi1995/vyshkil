//! Одне поле "з розумним пошуком" (02 §3) — тонкі домен-обізнані обгортки над
//! `components::Combobox mode=Input` (`docs/spec/components/grid.md` §2, розділ Б): компонент сам
//! не знає про `search_orgs`/`search_vos_position_course` (лишається domain-agnostic), обгортки
//! тут тримають `Resource` і передають уже готовий `Vec<ComboboxItem>` через `items`.
//!
//! Дві конкретні реалізації (організація / ВОС-посада-курс), а не один generic-компонент:
//! результати мають різну форму (`OrgSearchResult` vs `VosPositionCourseHint`), а Leptos
//! `#[component]` погано дружить з generic-параметрами по типу відповіді ресурсу.
//!
//! Кодування `ComboboxItem.value` (`Combobox` лишається domain-agnostic, не знає `i32`/
//! `VosPositionCourseKind`): організація — `org_id.to_string()`; ВОС/посада/курс — префікс
//! `"vos:{id}"`/`"position:{id}"`/`"course:{id}"`, розбір на виклику `on_select`.
//!
//! **Недавні значення** (`grid-interaction.md` §2: "фокус... одразу відкритий список: недавні...
//! далі весь довідник") — `localStorage`, per-актор, ручне рядкове кодування (той самий підхід,
//! що `columns.rs`: `serde_json` доступний лише під фічею `ssr`, тут WASM). Порожній запит більше
//! НЕ дає порожній результат: `backend::repo::{orgs,groups}::search_*` повертають бюджетний
//! "перші N довідника" список замість `Vec::new()` — недавні клієнта йдуть ПЕРЕД ним.

use leptos::prelude::*;

use crate::components::{Combobox, ComboboxItem, ComboboxMode, ComboboxVariant};
use crate::hooks::use_actor::use_actor;
use crate::services::groups::search_vos_position_course;
use crate::services::orgs::search_orgs;
use crate::types::submission::{VosPositionCourseHint, VosPositionCourseKind};

const RECENT_LIMIT: usize = 6;

fn recent_key(kind: &str, actor_org_id: i32) -> String {
    format!("taktoblik.recent.{kind}.{actor_org_id}")
}

/// `"value\tlabel"` на рядок -- значення/мітки в цьому довіднику не містять табуляцій/переносів.
fn load_recent(kind: &str, actor_org_id: i32) -> Vec<ComboboxItem> {
    if !cfg!(target_arch = "wasm32") {
        return Vec::new();
    }
    let Some(win) = web_sys::window() else { return Vec::new() };
    let Ok(Some(storage)) = win.local_storage() else { return Vec::new() };
    let Ok(Some(raw)) = storage.get_item(&recent_key(kind, actor_org_id)) else { return Vec::new() };
    raw.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(value, label)| ComboboxItem::new(value, label).with_group("Недавні"))
        .collect()
}

fn push_recent(kind: &str, actor_org_id: i32, value: &str, label: &str) {
    if !cfg!(target_arch = "wasm32") {
        return;
    }
    let Some(win) = web_sys::window() else { return };
    let Ok(Some(storage)) = win.local_storage() else { return };
    let key = recent_key(kind, actor_org_id);
    let mut items: Vec<(String, String)> = storage
        .get_item(&key)
        .ok()
        .flatten()
        .map(|raw| {
            raw.lines()
                .filter_map(|l| l.split_once('\t'))
                .map(|(v, l)| (v.to_string(), l.to_string()))
                .collect()
        })
        .unwrap_or_default();
    items.retain(|(v, _)| v != value);
    items.insert(0, (value.to_string(), label.to_string()));
    items.truncate(RECENT_LIMIT);
    let encoded = items.iter().map(|(v, l)| format!("{v}\t{l}")).collect::<Vec<_>>().join("\n");
    let _ = storage.set_item(&key, &encoded);
}

/// Пошук організації (02 §1 колонки 1 і 9, тулбар "Частина" — дефект 1) — звужений до видимого
/// акторові піддерева (сервер сам фільтрує через `policy::visible_org_ids`, як і
/// `pages::home::components::OrgSearch`).
#[component]
pub fn OrgAutocomplete(
    id: String,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] on_select: Callback<(i32, String)>,
    #[prop(into)] on_label_input: Callback<String>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
    // `InCell` (без рамки, дефолт тут — НЕ те саме, що дефолт `ComboboxVariant` -- `Field`) для
    // клітинок сітки/drawer, типовий випадок; тулбар-використання (дефект 1: частина-відправник
    // у тулбарі `/training-form`) передає `Field`, щоб мати ту саму рамку/підкладку, що
    // DatePicker/Select поруч, а не виглядати голим текстом.
    #[prop(optional, default = ComboboxVariant::InCell)] variant: ComboboxVariant,
) -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
    // Порожній запит теж шле -- сервер тепер повертає бюджетний довідник, не порожньо (дефект 3).
    let results = Resource::new(move || (actor.get(), query.get()), |(actor, q)| search_orgs(actor, q));
    let items = RwSignal::new(Vec::<ComboboxItem>::new());
    Effect::new(move |_| {
        let list = results.get().and_then(|r| r.ok()).unwrap_or_default();
        let recent = actor.get().map(|a| load_recent("org", a.org_id)).unwrap_or_default();
        let recent_values: std::collections::HashSet<String> =
            recent.iter().map(|i| i.value.clone()).collect();
        let mut merged = recent;
        merged.extend(list.into_iter().filter(|r| !recent_values.contains(&r.org_id.to_string())).map(
            |r| {
                let mut item = ComboboxItem::new(r.org_id.to_string(), r.label).with_group("Довідник");
                if !r.matched_raw.is_empty() {
                    item = item.with_description(format!("— \"{}\"", r.matched_raw));
                }
                item
            },
        ));
        items.set(merged);
    });

    view! {
        <Combobox
            id=id
            mode=ComboboxMode::Input
            variant=variant
            value=Signal::derive(|| String::new())
            items=items
            label=label
            placeholder="Частина…".to_string()
            on_label_input=Callback::new(move |v: String| on_label_input.run(v))
            on_query_change=Callback::new(move |v: String| query.set(v))
            on_change=Callback::new(move |v: String| {
                let Ok(org_id) = v.parse::<i32>() else { return };
                let label = items.get_untracked().into_iter().find(|it| it.value == v).map(|it| it.label);
                let label = label.unwrap_or_default();
                if let Some(a) = actor.get_untracked() {
                    push_recent("org", a.org_id, &v, &label);
                }
                on_select.run((org_id, label));
            })
            on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run(ev))
            empty_message="нічого не знайдено".to_string()
        />
    }
}

/// Поле "ВОС / посада / курс" (02 §3) — одне поле з поясненням "бо …" для непрямих (через ОВТ)
/// підказок.
#[component]
pub fn VosPositionCourseAutocomplete(
    id: String,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] on_select: Callback<VosPositionCourseHint>,
    #[prop(into)] on_label_input: Callback<String>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
) -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
    let results = Resource::new(move || query.get(), |q| search_vos_position_course(q));
    let hints = RwSignal::new(Vec::<VosPositionCourseHint>::new());
    Effect::new(move |_| {
        hints.set(results.get().and_then(|r| r.ok()).unwrap_or_default());
    });
    let items = Signal::derive(move || {
        let recent = actor.get().map(|a| load_recent("vos_position_course", a.org_id)).unwrap_or_default();
        let recent_values: std::collections::HashSet<String> =
            recent.iter().map(|i| i.value.clone()).collect();
        let mut merged = recent;
        merged.extend(hints.get().into_iter().filter_map(|h| {
            let kind_prefix = match h.kind {
                VosPositionCourseKind::Vos => "vos",
                VosPositionCourseKind::Position => "position",
                VosPositionCourseKind::Course => "course",
            };
            let value = format!("{kind_prefix}:{}", h.id);
            if recent_values.contains(&value) {
                return None;
            }
            let kind_label = match h.kind {
                VosPositionCourseKind::Vos => "ВОС",
                VosPositionCourseKind::Position => "посада",
                VosPositionCourseKind::Course => "курс",
            };
            let mut item =
                ComboboxItem::new(value, format!("{kind_label} · {}", h.label)).with_group("Довідник");
            if let Some(why) = &h.why {
                item = item.with_description(why.clone());
            }
            Some(item)
        }));
        merged
    });

    view! {
        <Combobox
            id=id
            mode=ComboboxMode::Input
            variant=ComboboxVariant::InCell
            value=Signal::derive(|| String::new())
            items=items
            label=label
            placeholder="ВОС / посада / курс…".to_string()
            on_label_input=Callback::new(move |v: String| on_label_input.run(v))
            on_query_change=Callback::new(move |v: String| query.set(v))
            on_change=Callback::new(move |v: String| {
                let Some((prefix, id_str)) = v.split_once(':') else { return };
                let Ok(id) = id_str.parse::<i32>() else { return };
                let kind = match prefix {
                    "vos" => VosPositionCourseKind::Vos,
                    "position" => VosPositionCourseKind::Position,
                    "course" => VosPositionCourseKind::Course,
                    _ => return,
                };
                let item_label =
                    items.get_untracked().into_iter().find(|it| it.value == v).map(|it| it.label);
                // Обране з "недавніх" -- не в поточному `hints` (той з ІНШОГО пошуку). Мітка
                // збережена в самому ComboboxItem ("ВОС · код — назва"), реконструюємо мінімальний
                // Hint з неї, коли пошук уже не тримає оригінал.
                let hint = hints.get_untracked().into_iter().find(|h| h.id == id && h.kind == kind).or_else(|| {
                    let label = item_label.clone()?;
                    let label = label.split_once(" · ").map(|(_, l)| l.to_string()).unwrap_or(label);
                    Some(VosPositionCourseHint { kind, id, label, matched_raw: String::new(), is_exact: false, why: None })
                });
                if let Some(hint) = hint {
                    if let Some(a) = actor.get_untracked() {
                        push_recent("vos_position_course", a.org_id, &v, &item_label.unwrap_or_default());
                    }
                    on_select.run(hint);
                }
            })
            on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run(ev))
            empty_message="нічого не знайдено".to_string()
        />
    }
}
