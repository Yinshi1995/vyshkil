mod server;

use leptos::prelude::*;
use style_macros::cx;

use crate::components::{download_bytes, DatePicker};
use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::widgets::group_grid::OrgAutocomplete;
use crate::widgets::ActorNotice;
use server::{generate_d1, generate_d2, generate_d3, generate_d4};

#[component]
pub fn DocumentsPage() -> impl IntoView {
    let actor = use_actor();

    view! {
        <PageHeader title="Документи".to_string()/>
        <PageContent width=ContentWidth::Detail>
            {move || {
                if actor.get().is_none() {
                    view! { <ActorNotice/> }.into_any()
                } else {
                    view! { <DocumentsBody/> }.into_any()
                }
            }}
        </PageContent>
    }
}

#[component]
fn DocumentsBody() -> impl IntoView {
    view! {
        <div class=cx!("flex col gap3")>
            <D1Block/>
            <D2Block/>
            <D3Block/>
            <D4Block/>
        </div>
    }
}

#[component]
fn D1Block() -> impl IntoView {
    let actor = use_actor();

    let org_id = RwSignal::new(None::<i32>);
    let org_label = RwSignal::new(String::new());
    let any_day = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let generating = RwSignal::new(false);

    let do_generate = move |_| {
        let Some(actor) = actor.get_untracked() else { return };
        let Some(oid) = org_id.get_untracked() else {
            status.set("оберіть частину".to_string());
            return;
        };
        let date = any_day.get_untracked();
        if date.trim().is_empty() {
            status.set("оберіть будь-який день потрібного тижня".to_string());
            return;
        }
        generating.set(true);
        status.set("генерую…".to_string());
        let label = org_label.get_untracked();
        leptos::task::spawn_local(async move {
            match generate_d1(Some(actor), oid, date.clone()).await {
                Ok(bytes) => {
                    let filename = format!("D1_{label}_{date}.xlsx");
                    download_bytes(
                        &bytes,
                        &filename,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    );
                    status.set("готово".to_string());
                }
                Err(e) => status.set(format!("не вдалось згенерувати: {e}")),
            }
            generating.set(false);
        });
    };

    view! {
        <div class=cx!("flex col gap3")>
            <p class="card__desc">
                "D1 — тижнева зведена таблиця органу (05 §D1): 7 денних аркушів + тижневий "
                "підсумок \"Закінчили/Почали\" формулами SUM(початок:кінець!C5). "
                "Оберіть будь-який день потрібного тижня."
            </p>
            <div class=cx!("flex items-c gap2 wrap")>
                <div class=cx!("w-full bg-raised bd r1")>
                    <OrgAutocomplete
                        id="documents-d1-org".to_string()
                        label=Signal::derive(move || org_label.get())
                        on_select=Callback::new(move |(id, label): (i32, String)| {
                            org_id.set(Some(id));
                            org_label.set(label);
                        })
                        on_label_input=Callback::new(move |v: String| {
                            org_label.set(v);
                            org_id.set(None);
                        })
                        on_keydown=Callback::new(|_| {})
                    />
                </div>
                <DatePicker
                    value=Signal::derive(move || {
                        chrono::NaiveDate::parse_from_str(&any_day.get(), "%Y-%m-%d").ok()
                    })
                    on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                        any_day.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                    })
                    placeholder="дд.мм.рррр".to_string()
                />
                <button
                    class="btn btn--outline"
                    disabled=move || generating.get()
                    on:click=do_generate
                >
                    "Згенерувати D1 (тиждень)"
                </button>
                <span class=cx!("fg-muted")>{move || status.get()}</span>
            </div>
        </div>
    }
}

#[component]
fn D2Block() -> impl IntoView {
    let actor = use_actor();

    let any_day = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let generating = RwSignal::new(false);

    let do_generate = move |_| {
        let Some(actor) = actor.get_untracked() else { return };
        let date = any_day.get_untracked();
        if date.trim().is_empty() {
            status.set("оберіть будь-який день потрібного тижня".to_string());
            return;
        }
        generating.set(true);
        status.set("генерую…".to_string());
        leptos::task::spawn_local(async move {
            match generate_d2(Some(actor), date.clone()).await {
                Ok(bytes) => {
                    let filename = format!("D2_Контролька_{date}.xlsx");
                    download_bytes(
                        &bytes,
                        &filename,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    );
                    status.set("готово".to_string());
                }
                Err(e) => status.set(format!("не вдалось згенерувати: {e}")),
            }
            generating.set(false);
        });
    };

    view! {
        <div class=cx!("flex col gap3")>
            <p class="card__desc">
                "D2 — \"Контролька\" (05 §D2): той самий rollup, накопичувальним тижнем по всіх "
                "корпусах одразу (07 §1 — один тиждень, не весь журнал). Оберіть будь-який день "
                "потрібного тижня."
            </p>
            <div class=cx!("flex items-c gap2 wrap")>
                <DatePicker
                    value=Signal::derive(move || {
                        chrono::NaiveDate::parse_from_str(&any_day.get(), "%Y-%m-%d").ok()
                    })
                    on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                        any_day.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                    })
                    placeholder="дд.мм.рррр".to_string()
                />
                <button
                    class="btn btn--outline"
                    disabled=move || generating.get()
                    on:click=do_generate
                >
                    "Згенерувати D2 (тиждень)"
                </button>
                <span class=cx!("fg-muted")>{move || status.get()}</span>
            </div>
        </div>
    }
}

#[component]
fn D3Block() -> impl IntoView {
    let actor = use_actor();

    let as_of = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let generating = RwSignal::new(false);

    let do_generate = move |_| {
        let Some(actor) = actor.get_untracked() else { return };
        let date = as_of.get_untracked();
        if date.trim().is_empty() {
            status.set("оберіть дату".to_string());
            return;
        }
        generating.set(true);
        status.set("генерую…".to_string());
        leptos::task::spawn_local(async move {
            match generate_d3(Some(actor), date.clone()).await {
                Ok(bytes) => {
                    let filename = format!("D3_Говорілка_{date}.docx");
                    download_bytes(
                        &bytes,
                        &filename,
                        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                    );
                    status.set("готово".to_string());
                }
                Err(e) => status.set(format!("не вдалось згенерувати: {e}")),
            }
            generating.set(false);
        });
    };

    view! {
        <div class=cx!("flex col gap3")>
            <p class="card__desc">
                "D3 — \"Говорілка\" (05 §D3): текст доповіді за один день, абзаци по слайдах "
                "презентації. Числа з пробілом-розділювачем тисяч, відсотки цілі, зміни за добу "
                "зі справжнім мінусом. Оберіть дату."
            </p>
            <div class=cx!("flex items-c gap2 wrap")>
                <DatePicker
                    value=Signal::derive(move || {
                        chrono::NaiveDate::parse_from_str(&as_of.get(), "%Y-%m-%d").ok()
                    })
                    on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                        as_of.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                    })
                    placeholder="дд.мм.рррр".to_string()
                />
                <button
                    class="btn btn--outline"
                    disabled=move || generating.get()
                    on:click=do_generate
                >
                    "Згенерувати D3"
                </button>
                <span class=cx!("fg-muted")>{move || status.get()}</span>
            </div>
        </div>
    }
}

#[component]
fn D4Block() -> impl IntoView {
    let actor = use_actor();

    let as_of = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let generating = RwSignal::new(false);

    let do_generate = move |_| {
        let Some(actor) = actor.get_untracked() else { return };
        let date = as_of.get_untracked();
        if date.trim().is_empty() {
            status.set("оберіть дату".to_string());
            return;
        }
        generating.set(true);
        status.set("генерую…".to_string());
        leptos::task::spawn_local(async move {
            match generate_d4(Some(actor), date.clone()).await {
                Ok(bytes) => {
                    let filename = format!("D4_Підготовка_{date}.pptx");
                    download_bytes(
                        &bytes,
                        &filename,
                        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                    );
                    status.set("готово".to_string());
                }
                Err(e) => status.set(format!("не вдалось згенерувати: {e}")),
            }
            generating.set(false);
        });
    };

    view! {
        <div class=cx!("flex col gap3")>
            <p class="card__desc">
                "D4 — \"Підготовка\" (05 §D4): презентація pptx з KPI-плитками та "
                "таблицями по корпусах. Стиль за spec (фон #0E0C08, акцент #F39200). "
                "Оберіть дату."
            </p>
            <div class=cx!("flex items-c gap2 wrap")>
                <DatePicker
                    value=Signal::derive(move || {
                        chrono::NaiveDate::parse_from_str(&as_of.get(), "%Y-%m-%d").ok()
                    })
                    on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                        as_of.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                    })
                    placeholder="дд.мм.рррр".to_string()
                />
                <button
                    class="btn btn--outline"
                    disabled=move || generating.get()
                    on:click=do_generate
                >
                    "Згенерувати D4"
                </button>
                <span class=cx!("fg-muted")>{move || status.get()}</span>
            </div>
        </div>
    }
}
