mod server;

use leptos::prelude::*;
use style_macros::cx;

use crate::components::{download_bytes, DatePicker};
use crate::hooks::use_actor::use_actor;
use crate::widgets::group_grid::OrgAutocomplete;
use crate::widgets::ActorNotice;
use server::generate_d1;

/// Генерація документів (Етап 7, 05) — перший вертикальний зріз: лише D1 (щоденна зведена
/// таблиця органу, один денний аркуш). D2/D3/D4 — окремі кроки після.
#[component]
pub fn DocumentsPage() -> impl IntoView {
    let actor = use_actor();

    view! {
        <h1>"Документи"</h1>
        {move || {
            if actor.get().is_none() {
                view! { <ActorNotice/> }.into_any()
            } else {
                view! { <DocumentsBody/> }.into_any()
            }
        }}
    }
}

#[component]
fn DocumentsBody() -> impl IntoView {
    let actor = use_actor();

    let org_id = RwSignal::new(None::<i32>);
    let org_label = RwSignal::new(String::new());
    let as_of_date = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let generating = RwSignal::new(false);

    let do_generate = move |_| {
        let Some(actor) = actor.get_untracked() else { return };
        let Some(oid) = org_id.get_untracked() else {
            status.set("оберіть частину".to_string());
            return;
        };
        let date = as_of_date.get_untracked();
        if date.trim().is_empty() {
            status.set("«станом на»: оберіть дату".to_string());
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
                "D1 — щоденна зведена таблиця органу (05 §D1): БЗВП/Фахова/Адаптація по прямих "
                "підрозділах, за правилом групування (01 §1)."
            </p>
            <div class=cx!("flex items-c gap2 wrap")>
                <div class=cx!("w-full bg-raised bd r1")>
                    <OrgAutocomplete
                        id="documents-org".to_string()
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
                        chrono::NaiveDate::parse_from_str(&as_of_date.get(), "%Y-%m-%d").ok()
                    })
                    on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                        as_of_date.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                    })
                    placeholder="дд.мм.рррр".to_string()
                />
                <button
                    class="btn btn--primary"
                    disabled=move || generating.get()
                    on:click=do_generate
                >
                    "Згенерувати D1"
                </button>
                <span class=cx!("fg-muted")>{move || status.get()}</span>
            </div>
        </div>
    }
}
