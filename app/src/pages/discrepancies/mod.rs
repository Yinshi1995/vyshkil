mod server;

use leptos::prelude::*;

use crate::components::{Select, SelectOption};
use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader, Toolbar};
use crate::types::reconciliation::DiscrepancyRow;
use crate::widgets::ActorNotice;
use server::get_discrepancies;

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "horizontal" => "Горизонтальна",
        "vertical" => "Вертикальна",
        "temporal" => "Часова",
        "duplicate" => "Дублікат",
        "data_quality" => "Якість даних",
        _ => "?",
    }
}

/// Екран розбіжностей (04 §4, Етап 8 зріз 1 — `.claude/decisions/
/// etap8-horizontal-reconciliation-first-slice.md`): лише перегляд, без workflow "взяти в
/// роботу"/"закрити вручну" — відкриття/автозакриття відбувається саме при фіксації сітки
/// (`repo::reconciliation::refresh_horizontal`), не тут.
#[component]
pub fn DiscrepanciesPage() -> impl IntoView {
    let actor = use_actor();

    view! {
        <PageHeader title="Розбіжності".to_string()/>
        {move || {
            if actor.get().is_none() {
                view! { <ActorNotice/> }.into_any()
            } else {
                view! { <DiscrepanciesBody/> }.into_any()
            }
        }}
    }
}

#[component]
fn DiscrepanciesBody() -> impl IntoView {
    let actor = use_actor();
    let status = RwSignal::new("open".to_string());
    let discrepancies = Resource::new(
        move || (actor.get(), status.get()),
        |(actor, status)| async move { get_discrepancies(actor, Some(status)).await },
    );

    view! {
        <PageContent width=ContentWidth::Data>
            <Toolbar>
                <div class="toolbar__left">
                    <Select
                        value=status
                        options=Signal::derive(|| {
                            vec![
                                SelectOption::new("open", "Відкриті"),
                                SelectOption::new("resolved", "Закриті"),
                                SelectOption::new("", "Усі"),
                            ]
                        })
                        on_change=Callback::new(move |v: String| status.set(v))
                    />
                </div>
            </Toolbar>
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    discrepancies
                        .get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Розбіжностей нема."</p> }.into_any()
                            }
                            Ok(list) => view! { <DiscrepancyTable rows=list/> }.into_any(),
                            Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                        })
                }}
            </Suspense>
        </PageContent>
    }
}

#[component]
fn DiscrepancyTable(rows: Vec<DiscrepancyRow>) -> impl IntoView {
    view! {
        <table>
            <thead>
                <tr>
                    <th>"Тип"</th>
                    <th>"Частина"</th>
                    <th>"Група"</th>
                    <th>"Метрик"</th>
                    <th>"Значення"</th>
                    <th>"Статус"</th>
                    <th>"Виявлено"</th>
                </tr>
            </thead>
            <tbody>
                {rows
                    .into_iter()
                    .map(|r| {
                        let values_text = r
                            .values
                            .iter()
                            .map(|(sid, v)| format!("подання №{sid}: {v}"))
                            .collect::<Vec<_>>()
                            .join(" · ");
                        let status_class = if r.status == "open" { "status-error" } else { "status-ok" };
                        let kind_label = kind_label(&r.kind);
                        view! {
                            <tr>
                                <td>{kind_label}</td>
                                <td>{r.org_label}</td>
                                <td>{r.group_label.unwrap_or_else(|| "—".to_string())}</td>
                                <td>{r.metric_label}</td>
                                <td>{values_text}</td>
                                <td class=status_class>{r.status}</td>
                                <td>{r.created_at}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}
