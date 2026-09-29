//! Форма редагування ОДНОГО рядка (feedback користувача — "як Notion": розгорнути рядок збоку,
//! а не тільки редагувати в тісних клітинках). Той самий `GroupFormRow`, ті самі поля/резолюція
//! (`OrgAutocomplete`/`Select`), що в `Grid`, — лише вертикальна розкладка з підписами.
//!
//! Заразом закриває реальну прогалину (02 §1 колонка 8): "Розподіл за підрозділами" мав
//! резолюцію лише з імпорту (`backend::import::*`), ручного способу ввести/змінити `composition`
//! не було зовсім — тут `CompositionEditor` додає, редагує, видаляє підрозділи.

use leptos::prelude::*;

use super::autocomplete::{OrgAutocomplete, VosPositionCourseAutocomplete};
use crate::components::{Select, SelectOption};
use crate::services::dictionaries::get_dictionaries_overview;
use crate::services::groups::get_training_sites;
use crate::types::submission::{CompositionRow, GroupFormRow, VosPositionCourseHint, VosPositionCourseKind};

#[component]
pub fn RowEditor(data: RwSignal<GroupFormRow>) -> impl IntoView {
    let noop_keydown = Callback::new(|_: web_sys::KeyboardEvent| {});

    let kinds = Resource::new(|| (), |_| get_dictionaries_overview());
    let kind_options = RwSignal::new(vec![SelectOption::new("", "—")]);
    Effect::new(move |_| {
        let mut opts = vec![SelectOption::new("", "—")];
        if let Some(Ok(o)) = kinds.get() {
            opts.extend(o.training_kinds.into_iter().map(|k| SelectOption::new(k.id.to_string(), k.label)));
        }
        kind_options.set(opts);
    });

    let sites = Resource::new(
        move || data.get().sender_org_id,
        |org_id| async move {
            match org_id {
                Some(id) => get_training_sites(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let site_options = RwSignal::new(vec![SelectOption::new("", "—")]);
    Effect::new(move |_| {
        let mut opts = vec![SelectOption::new("", "—")];
        if let Some(Ok(list)) = sites.get() {
            opts.extend(list.into_iter().map(|s| SelectOption::new(s.site_id.to_string(), s.label)));
        }
        site_options.set(opts);
    });

    view! {
        <h2 class="modal__title">"Група"</h2>
        <div class="row-editor">
            <div class="row-editor__field">
                <label>"Військова частина"</label>
                <OrgAutocomplete
                    id="row-editor-sender-org".to_string()
                    label=Signal::derive(move || data.get().sender_org_label)
                    on_select=move |id: i32, label: String| {
                        data.update(|d| {
                            d.sender_org_id = Some(id);
                            d.sender_org_label = label;
                            d.site_id = None;
                            d.site_label.clear();
                        });
                    }
                    on_label_input=Callback::new(move |v: String| {
                        data.update(|d| {
                            d.sender_org_label = v;
                            d.sender_org_id = None;
                        });
                    })
                    on_keydown=noop_keydown
                />
            </div>

            <div class="row-editor__field">
                <label>"Вид підготовки"</label>
                <Select
                    value=Signal::derive(move || data.get().training_kind_id.map(|id| id.to_string()).unwrap_or_default())
                    options=Signal::from(kind_options)
                    placeholder="—"
                    on_change=Callback::new(move |v: String| {
                        data.update(|d| d.training_kind_id = v.parse().ok());
                    })
                />
            </div>

            <div class="row-editor__field">
                <label>"ВОС / посада / курс"</label>
                <VosPositionCourseAutocomplete
                    id="row-editor-vos".to_string()
                    label=Signal::derive(move || data.get().vos_position_course_label)
                    on_select=Callback::new(move |hint: VosPositionCourseHint| {
                        data.update(|d| {
                            d.vos_id = None;
                            d.position_id = None;
                            d.course_id = None;
                            match hint.kind {
                                VosPositionCourseKind::Vos => d.vos_id = Some(hint.id),
                                VosPositionCourseKind::Position => d.position_id = Some(hint.id),
                                VosPositionCourseKind::Course => d.course_id = Some(hint.id),
                            }
                            d.vos_position_course_label = hint.label;
                        });
                    })
                    on_label_input=Callback::new(move |v: String| {
                        data.update(|d| d.vos_position_course_label = v);
                    })
                    on_keydown=noop_keydown
                />
            </div>

            <div class="row-editor__field">
                <label>"ОВТ"</label>
                <input
                    type="text"
                    class="cell__input"
                    prop:value=move || data.get().equipment_text
                    on:input=move |ev| data.update(|d| d.equipment_text = event_target_value(&ev))
                />
            </div>

            <div class="row-editor__field">
                <label>"Місце проведення"</label>
                <Select
                    value=Signal::derive(move || data.get().site_id.map(|id| id.to_string()).unwrap_or_default())
                    options=Signal::from(site_options)
                    placeholder="—"
                    on_change=Callback::new(move |v: String| {
                        let id: Option<i32> = v.parse().ok();
                        let sites_now = sites.get_untracked().and_then(|r| r.ok()).unwrap_or_default();
                        let label = sites_now.into_iter().find(|s| Some(s.site_id) == id).map(|s| s.label);
                        data.update(|d| {
                            d.site_id = id;
                            d.site_label = label.unwrap_or_default();
                        });
                    })
                />
            </div>

            <div class="row-editor__row">
                <div class="row-editor__field">
                    <label>"Термін з"</label>
                    <input
                        type="text"
                        class="cell__input"
                        placeholder="18.08"
                        prop:value=move || data.get().planned_start_raw
                        on:input=move |ev| data.update(|d| d.planned_start_raw = event_target_value(&ev))
                    />
                </div>
                <div class="row-editor__field">
                    <label>"Термін по"</label>
                    <input
                        type="text"
                        class="cell__input"
                        placeholder="09.10"
                        prop:value=move || data.get().planned_end_raw
                        on:input=move |ev| data.update(|d| d.planned_end_raw = event_target_value(&ev))
                    />
                </div>
            </div>

            <div class="row-editor__row">
                <div class="row-editor__field">
                    <label>"План"</label>
                    <input
                        type="number"
                        min="0"
                        class="cell__input"
                        prop:value=move || data.get().planned_count.to_string()
                        on:input=move |ev| {
                            let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                            data.update(|d| d.planned_count = v);
                        }
                    />
                </div>
                <div class="row-editor__field">
                    <label>"Прибуло"</label>
                    <input
                        type="number"
                        min="0"
                        class="cell__input"
                        prop:value=move || data.get().arrived_count.to_string()
                        on:input=move |ev| {
                            let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                            data.update(|d| d.arrived_count = v);
                        }
                    />
                </div>
                <div class="row-editor__field">
                    <label>"Навчаються"</label>
                    <input
                        type="number"
                        min="0"
                        class="cell__input"
                        prop:value=move || data.get().in_training_count.to_string()
                        on:input=move |ev| {
                            let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                            data.update(|d| d.in_training_count = v);
                        }
                    />
                </div>
            </div>

            <div class="row-editor__field">
                <label>"Розподіл за підрозділами"</label>
                <CompositionEditor data=data/>
            </div>

            <div class="row-editor__field">
                <label>"Організатор"</label>
                <OrgAutocomplete
                    id="row-editor-organizer-org".to_string()
                    label=Signal::derive(move || data.get().organizer_org_label)
                    on_select=move |id: i32, label: String| {
                        data.update(|d| {
                            d.organizer_org_id = Some(id);
                            d.organizer_org_label = label;
                        });
                    }
                    on_label_input=Callback::new(move |v: String| {
                        data.update(|d| {
                            d.organizer_org_label = v;
                            d.organizer_org_id = None;
                        });
                    })
                    on_keydown=noop_keydown
                />
            </div>

            <div class="row-editor__row">
                <div class="row-editor__field">
                    <label>"№ розпорядження"</label>
                    <input
                        type="text"
                        class="cell__input"
                        prop:value=move || data.get().basis_doc_number
                        on:input=move |ev| data.update(|d| d.basis_doc_number = event_target_value(&ev))
                    />
                </div>
                <div class="row-editor__field">
                    <label>"Дата розпорядження"</label>
                    <input
                        type="text"
                        class="cell__input"
                        placeholder="25.02.26"
                        prop:value=move || data.get().basis_doc_date_raw
                        on:input=move |ev| data.update(|d| d.basis_doc_date_raw = event_target_value(&ev))
                    />
                </div>
            </div>

            <div class="row-editor__field">
                <label>"Примітка"</label>
                <input
                    type="text"
                    class="cell__input"
                    prop:value=move || data.get().note
                    on:input=move |ev| data.update(|d| d.note = event_target_value(&ev))
                />
            </div>
        </div>
    }
}

/// Додає/змінює/видаляє підрозділи (02 §1 колонка 8) — раніше досяжно лише через імпорт.
/// Ключ `<For>` — індекс, не синтетичний id (`CompositionRow` його не має): прийнятно для
/// короткого (типово 0-3 елементи) невіртуалізованого списку, не критично для фокуса, як
/// основна сітка.
#[component]
fn CompositionEditor(data: RwSignal<GroupFormRow>) -> impl IntoView {
    view! {
        <div class="composition-editor">
            <For
                each=move || { (0..data.get().composition.len()).collect::<Vec<_>>() }
                key=|i| *i
                let:i
            >
                <div class="composition-editor__row">
                    <OrgAutocomplete
                        id=format!("row-editor-subunit-{i}")
                        label=Signal::derive(move || {
                            data.get().composition.get(i).map(|c| c.subunit_label.clone()).unwrap_or_default()
                        })
                        on_select=move |id: i32, label: String| {
                            data.update(|d| {
                                if let Some(c) = d.composition.get_mut(i) {
                                    c.subunit_org_id = Some(id);
                                    c.subunit_label = label;
                                }
                            });
                        }
                        on_label_input=Callback::new(move |v: String| {
                            data.update(|d| {
                                if let Some(c) = d.composition.get_mut(i) {
                                    c.subunit_label = v;
                                    c.subunit_org_id = None;
                                }
                            });
                        })
                        on_keydown=Callback::new(|_| {})
                    />
                    <input
                        type="number"
                        min="0"
                        class="cell__input"
                        prop:value=move || data.get().composition.get(i).map(|c| c.count.to_string()).unwrap_or_default()
                        on:input=move |ev| {
                            let v: i64 = event_target_value(&ev).parse().unwrap_or(0);
                            data.update(|d| {
                                if let Some(c) = d.composition.get_mut(i) {
                                    c.count = v;
                                }
                            });
                        }
                    />
                    <button
                        type="button"
                        class="btn btn--outline"
                        on:click=move |_| {
                            data.update(|d| {
                                if i < d.composition.len() {
                                    d.composition.remove(i);
                                }
                            });
                        }
                    >
                        "Видалити"
                    </button>
                </div>
            </For>
            <button
                type="button"
                class="btn btn--outline"
                on:click=move |_| data.update(|d| d.composition.push(CompositionRow::default()))
            >
                "+ Підрозділ"
            </button>
        </div>
    }
}
