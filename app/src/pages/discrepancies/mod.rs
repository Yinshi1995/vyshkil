mod server;

use leptos::prelude::*;

use crate::components::{Select, SelectOption};
use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader, Toolbar};
use crate::types::actor::Role;
use crate::types::reconciliation::DiscrepancyRow;
use crate::widgets::ActorNotice;
use server::{get_discrepancies, update_discrepancy_status};

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

fn status_label(status: &str) -> &'static str {
    match status {
        "open" => "Відкрита",
        "notified" => "Повідомлено",
        "in_progress" => "В роботі",
        "resolved" => "Закрита",
        "dismissed" => "Відхилена",
        _ => "?",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        "open" => "status-error",
        "in_progress" => "status-warning",
        "resolved" | "dismissed" => "status-ok",
        _ => "",
    }
}

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
    let refresh_counter = RwSignal::new(0u32);
    let discrepancies = Resource::new(
        move || (actor.get(), status.get(), refresh_counter.get()),
        |(actor, status, _)| async move { get_discrepancies(actor, Some(status)).await },
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
                                SelectOption::new("in_progress", "В роботі"),
                                SelectOption::new("resolved", "Закриті"),
                                SelectOption::new("dismissed", "Відхилені"),
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
                            Ok(list) => {
                                view! { <DiscrepancyTable rows=list on_refresh=refresh_counter/> }
                                    .into_any()
                            }
                            Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                        })
                }}
            </Suspense>
        </PageContent>
    }
}

#[component]
fn DiscrepancyTable(rows: Vec<DiscrepancyRow>, on_refresh: RwSignal<u32>) -> impl IntoView {
    let actor = use_actor();
    let can_edit = move |org_id: i32| {
        actor.get().is_some_and(|a| {
            a.role == Role::Admin || (a.role == Role::OrgEditor && a.org_id == org_id)
        })
    };

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
                    <th>"Дії"</th>
                </tr>
            </thead>
            <tbody>
                {rows
                    .into_iter()
                    .map(|r| {
                        let values_text = r
                            .values
                            .iter()
                            .map(|dv| format!("{}: {}", dv.source_label, dv.value))
                            .collect::<Vec<_>>()
                            .join(" · ");
                        let st_class = status_class(&r.status);
                        let st_label = status_label(&r.status);
                        let kind_label = kind_label(&r.kind);
                        let is_actionable = r.status == "open" || r.status == "in_progress";
                        let row_org_id = r.org_id;
                        let row_id = r.id;
                        view! {
                            <tr>
                                <td>{kind_label}</td>
                                <td>{r.org_label}</td>
                                <td>{r.group_label.unwrap_or_else(|| "—".to_string())}</td>
                                <td>{r.metric_label}</td>
                                <td>{values_text}</td>
                                <td class=st_class>{st_label}</td>
                                <td>{r.created_at}</td>
                                <td>
                                    {move || {
                                        if is_actionable && can_edit(row_org_id) {
                                            view! {
                                                <DiscrepancyActions
                                                    id=row_id
                                                    status=r.status.clone()
                                                    on_refresh=on_refresh
                                                />
                                            }
                                                .into_any()
                                        } else {
                                            view! { <span>"—"</span> }.into_any()
                                        }
                                    }}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

#[component]
fn DiscrepancyActions(id: i32, status: String, on_refresh: RwSignal<u32>) -> impl IntoView {
    let actor = use_actor();
    let action_pending = RwSignal::new(false);
    let action_error = RwSignal::new(Option::<String>::None);

    let do_action = move |new_status: &'static str, note: Option<&'static str>| {
        let actor_val = actor.get();
        action_pending.set(true);
        action_error.set(None);
        leptos::task::spawn_local(async move {
            let result =
                update_discrepancy_status(actor_val, id, new_status.to_string(), note.map(String::from))
                    .await;
            action_pending.set(false);
            match result {
                Ok(()) => on_refresh.update(|c| *c += 1),
                Err(e) => action_error.set(Some(e.to_string())),
            }
        });
    };

    let is_open = status == "open";

    view! {
        <div class="discrepancy-actions">
            {move || {
                if action_pending.get() {
                    return view! { <span>"…"</span> }.into_any();
                }
                if let Some(err) = action_error.get() {
                    return view! { <span class="status-error" title=err>"✗"</span> }.into_any();
                }
                view! {
                    <span>
                        {if is_open {
                            Some(
                                view! {
                                    <button
                                        class="btn btn--ghost btn--sm"
                                        on:click=move |_| do_action("in_progress", None)
                                    >
                                        "В роботу"
                                    </button>
                                },
                            )
                        } else {
                            None
                        }}
                        <button
                            class="btn btn--ghost btn--sm"
                            on:click=move |_| do_action("resolved", Some("закрито вручну"))
                        >
                            "Закрити"
                        </button>
                        <button
                            class="btn btn--ghost btn--sm"
                            on:click=move |_| do_action("dismissed", Some("відхилено"))
                        >
                            "Відхилити"
                        </button>
                    </span>
                }
                    .into_any()
            }}
        </div>
    }
}
