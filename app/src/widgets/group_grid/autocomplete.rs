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

use leptos::prelude::*;

use crate::components::{Combobox, ComboboxItem, ComboboxMode, ComboboxVariant};
use crate::hooks::use_actor::use_actor;
use crate::services::groups::search_vos_position_course;
use crate::services::orgs::search_orgs;
use crate::types::submission::{VosPositionCourseHint, VosPositionCourseKind};

/// Пошук організації (02 §1 колонки 1 і 9) — звужений до видимого акторові піддерева (сервер
/// сам фільтрує через `policy::visible_org_ids`, як і `pages::home::components::OrgSearch`).
#[component]
pub fn OrgAutocomplete(
    id: String,
    #[prop(into)] label: Signal<String>,
    #[prop(into)] on_select: Callback<(i32, String)>,
    #[prop(into)] on_label_input: Callback<String>,
    #[prop(into)] on_keydown: Callback<web_sys::KeyboardEvent>,
) -> impl IntoView {
    let actor = use_actor();
    let query = RwSignal::new(String::new());
    let results = Resource::new(
        move || (actor.get(), query.get()),
        |(actor, q)| async move {
            if q.trim().is_empty() { Ok(Vec::new()) } else { search_orgs(actor, q).await }
        },
    );
    // Читання ресурсу через `Effect` у звичайний сигнал, не `Signal::derive` напряму — той самий
    // застереження, що `TrainingKindCell`/`SiteCell` (`grid.rs`): без цього Leptos попереджає
    // "reading a resource outside Suspense/effect" (можлива розбіжність гідратації).
    let items = RwSignal::new(Vec::<ComboboxItem>::new());
    Effect::new(move |_| {
        let list = results.get().and_then(|r| r.ok()).unwrap_or_default();
        items.set(
            list.into_iter()
                .map(|r| ComboboxItem::new(r.org_id.to_string(), r.label).with_description(format!("— \"{}\"", r.matched_raw)))
                .collect(),
        );
    });

    view! {
        <Combobox
            id=id
            mode=ComboboxMode::Input
            variant=ComboboxVariant::InCell
            value=Signal::derive(|| String::new())
            items=items
            label=label
            placeholder="Частина…".to_string()
            on_label_input=Callback::new(move |v: String| on_label_input.run(v))
            on_query_change=Callback::new(move |v: String| query.set(v))
            on_change=Callback::new(move |v: String| {
                let Ok(org_id) = v.parse::<i32>() else { return };
                let label = items.get_untracked().into_iter().find(|it| it.value == v).map(|it| it.label);
                on_select.run((org_id, label.unwrap_or_default()));
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
    let query = RwSignal::new(String::new());
    let results = Resource::new(
        move || query.get(),
        |q| async move {
            if q.trim().is_empty() { Ok(Vec::new()) } else { search_vos_position_course(q).await }
        },
    );
    let hints = RwSignal::new(Vec::<VosPositionCourseHint>::new());
    Effect::new(move |_| {
        hints.set(results.get().and_then(|r| r.ok()).unwrap_or_default());
    });
    let items = Signal::derive(move || {
        hints
            .get()
            .into_iter()
            .map(|h| {
                let kind_prefix = match h.kind {
                    VosPositionCourseKind::Vos => "vos",
                    VosPositionCourseKind::Position => "position",
                    VosPositionCourseKind::Course => "course",
                };
                let kind_label = match h.kind {
                    VosPositionCourseKind::Vos => "ВОС",
                    VosPositionCourseKind::Position => "посада",
                    VosPositionCourseKind::Course => "курс",
                };
                let mut item = ComboboxItem::new(
                    format!("{kind_prefix}:{}", h.id),
                    format!("{kind_label} · {}", h.label),
                );
                if let Some(why) = &h.why {
                    item = item.with_description(why.clone());
                }
                item
            })
            .collect::<Vec<_>>()
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
                let hint = hints.get_untracked().into_iter().find(|h| h.id == id && h.kind == kind);
                if let Some(hint) = hint {
                    on_select.run(hint);
                }
            })
            on_keydown=Callback::new(move |ev: web_sys::KeyboardEvent| on_keydown.run(ev))
            empty_message="нічого не знайдено".to_string()
        />
    }
}
