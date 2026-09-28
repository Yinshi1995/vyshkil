mod server;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::hooks::use_actor::use_actor;
use crate::widgets::ActorNotice;
use server::get_org_detail;

/// Сторінка `/org/:id`: поточні дані + історія назв/статусів (порожня історія — легітимний стан,
/// поки в частини не було жодного перейменування чи зміни статусу). Доступ перевіряється
/// `backend::policy::can_view_org` (01 §6) — без обраного актора показуємо `<ActorNotice/>`.
#[component]
pub fn OrgDetailPage() -> impl IntoView {
    let actor = use_actor();
    let params = use_params_map();
    let org_id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or_default()
    };
    let detail = Resource::new(
        move || (actor.get(), org_id()),
        |(actor, id)| async move { get_org_detail(actor, id).await },
    );

    view! {
        <Suspense fallback=|| view! { <p>"…"</p> }>
            {move || {
                if actor.get().is_none() {
                    return Some(view! { <ActorNotice/> }.into_any());
                }
                detail
                    .get()
                    .map(|res| match res {
                        Ok(d) => {
                            let label = match &d.number {
                                Some(n) => format!("{} ({n})", d.short_name),
                                None => d.short_name.clone(),
                            };
                            view! {
                                <h1>{label}</h1>
                                <p>
                                    {d.full_name.clone().unwrap_or_default()} " · " {d.kind.clone()}
                                    " · " {if d.is_active { "діюча" } else { "неактивна" }}
                                </p>
                                <div class="eyebrow">"Історія назв"</div>
                                <div class="card">
                                    {if d.name_history.is_empty() {
                                        view! { <p class="card__desc">"Перейменувань не було."</p> }.into_any()
                                    } else {
                                        view! {
                                            <table>
                                                <thead>
                                                    <tr>
                                                        <th>"Назва"</th>
                                                        <th>"Діє з"</th>
                                                        <th>"Діє до"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {d.name_history
                                                        .into_iter()
                                                        .map(|(name, from, to)| {
                                                            view! {
                                                                <tr>
                                                                    <td>{name}</td>
                                                                    <td>{from}</td>
                                                                    <td>{to.unwrap_or_else(|| "—".to_string())}</td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        }
                                            .into_any()
                                    }}
                                </div>
                                <div class="eyebrow">"Історія статусів"</div>
                                <div class="card">
                                    {if d.status_history.is_empty() {
                                        view! { <p class="card__desc">"Змін статусу не було."</p> }.into_any()
                                    } else {
                                        view! {
                                            <table>
                                                <thead>
                                                    <tr>
                                                        <th>"Статус"</th>
                                                        <th>"Діє з"</th>
                                                        <th>"Діє до"</th>
                                                        <th>"Примітка"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {d.status_history
                                                        .into_iter()
                                                        .map(|(status, from, to, note)| {
                                                            view! {
                                                                <tr>
                                                                    <td>{status}</td>
                                                                    <td>{from}</td>
                                                                    <td>{to.unwrap_or_else(|| "—".to_string())}</td>
                                                                    <td>{note.unwrap_or_default()}</td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        }
                                            .into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => {
                            view! { <p class="status-error">{e.to_string()}</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
