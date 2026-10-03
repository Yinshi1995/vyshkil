mod server;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::hooks::use_actor::use_actor;
use crate::layout::{Breadcrumb, ContentWidth, PageContent, PageHeader};
use crate::types::actor::Role;
use crate::types::auth::{AdminGroupRow, AdminSubmissionRow, AdminUserRow};
use crate::widgets::ActorNotice;
use server::{get_org_children, get_org_detail, get_org_groups, get_org_submissions, get_org_users};

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
    let children = Resource::new(
        move || (actor.get(), org_id()),
        |(actor, id)| async move { get_org_children(actor, id).await },
    );
    let groups = Resource::new(
        move || (actor.get(), org_id()),
        |(actor, id)| async move { get_org_groups(actor, id).await },
    );
    let submissions = Resource::new(
        move || (actor.get(), org_id()),
        |(actor, id)| async move { get_org_submissions(actor, id).await },
    );
    let is_admin = move || actor.get().map(|a| a.role == Role::Admin).unwrap_or(false);
    let users = Resource::new(
        move || (actor.get(), org_id(), is_admin()),
        |(actor, id, admin)| async move {
            if !admin { return Ok(vec![]); }
            get_org_users(actor, id).await
        },
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
                            let label = d.short_name.clone();
                            let subtitle = [
                                d.full_name.clone().filter(|s| !s.is_empty()),
                                Some(d.kind.clone()),
                                Some(if d.is_active { "діюча" } else { "неактивна" }.to_string()),
                            ]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join(" · ");
                            view! {
                                <PageHeader
                                    title=label
                                    subtitle=subtitle
                                    breadcrumbs=vec![Breadcrumb::link("Головна", "/")]
                                />
                                <PageContent width=ContentWidth::Data>
                                    // --- Підлеглі підрозділи ---
                                    <ChildrenSection children/>

                                    // --- Групи на навчанні ---
                                    <GroupsSection groups/>

                                    // --- Подання ---
                                    <SubmissionsSection submissions/>

                                    // --- Користувачі (admin) ---
                                    <Show when=is_admin>
                                        <UsersSection users/>
                                    </Show>

                                    // --- Історія ---
                                    <div class="page-content__grid">
                                        <div>
                                            <div class="eyebrow">"Історія назв"</div>
                                            <div class="card">
                                                {if d.name_history.is_empty() {
                                                    view! {
                                                        <p class="card__desc">"Перейменувань не було."</p>
                                                    }
                                                        .into_any()
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
                                        </div>
                                        <div>
                                            <div class="eyebrow">"Історія статусів"</div>
                                            <div class="card">
                                                {if d.status_history.is_empty() {
                                                    view! {
                                                        <p class="card__desc">"Змін статусу не було."</p>
                                                    }
                                                        .into_any()
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
                                        </div>
                                    </div>
                                </PageContent>
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

#[component]
fn ChildrenSection(
    children: Resource<Result<Vec<(i32, String)>, ServerFnError>>,
) -> impl IntoView {
    view! {
        <Suspense fallback=|| view! { <p>"…"</p> }>
            {move || {
                children.get().map(|res| match res {
                    Ok(list) if list.is_empty() => {
                        view! { "" }.into_any()
                    }
                    Ok(list) => {
                        view! {
                            <div class="eyebrow">"Підлеглі підрозділи"</div>
                            <div class="card">
                                <div class="org-children">
                                    {list.into_iter().map(|(id, label)| {
                                        let href = format!("/org/{id}");
                                        view! {
                                            <a href=href class="org-children__link">{label}</a>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    }
                    Err(_) => view! { "" }.into_any(),
                })
            }}
        </Suspense>
    }
}

#[component]
fn GroupsSection(
    groups: Resource<Result<Vec<AdminGroupRow>, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="eyebrow">"Групи на навчанні"</div>
        <div class="card">
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    groups.get().map(|res| match res {
                        Ok(list) if list.is_empty() => {
                            view! { <p class="card__desc">"Груп не знайдено."</p> }.into_any()
                        }
                        Ok(list) => {
                            view! {
                                <div class="table-scroll">
                                    <table>
                                        <thead>
                                            <tr>
                                                <th>"Підрозділ"</th>
                                                <th>"Вид"</th>
                                                <th>"ВОС / посада / курс"</th>
                                                <th>"Місце"</th>
                                                <th>"Початок"</th>
                                                <th>"Кінець"</th>
                                                <th>"План"</th>
                                                <th>"Прибуло"</th>
                                                <th>"Навч."</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|g| {
                                                view! {
                                                    <tr>
                                                        <td>{g.org_label}</td>
                                                        <td>{g.training_kind}</td>
                                                        <td>{g.vos_label}</td>
                                                        <td>{g.site_label}</td>
                                                        <td>{g.planned_start}</td>
                                                        <td>{g.planned_end}</td>
                                                        <td>{g.planned_count}</td>
                                                        <td>{g.arrived_count}</td>
                                                        <td>{g.in_training_count}</td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }
                        Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn SubmissionsSection(
    submissions: Resource<Result<Vec<AdminSubmissionRow>, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="eyebrow">"Подання"</div>
        <div class="card">
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    submissions.get().map(|res| match res {
                        Ok(list) if list.is_empty() => {
                            view! { <p class="card__desc">"Подань не знайдено."</p> }.into_any()
                        }
                        Ok(list) => {
                            view! {
                                <div class="table-scroll">
                                    <table>
                                        <thead>
                                            <tr>
                                                <th>"Підрозділ"</th>
                                                <th>"Тип"</th>
                                                <th>"Статус"</th>
                                                <th>"Станом на"</th>
                                                <th>"Оновлено"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|s| {
                                                view! {
                                                    <tr>
                                                        <td>{s.org_label}</td>
                                                        <td>{s.source_type}</td>
                                                        <td>{s.status}</td>
                                                        <td>{s.as_of_date}</td>
                                                        <td>{s.updated_at}</td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }
                        Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn UsersSection(
    users: Resource<Result<Vec<AdminUserRow>, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="eyebrow">"Користувачі"</div>
        <div class="card">
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    users.get().map(|res| match res {
                        Ok(list) if list.is_empty() => {
                            view! { <p class="card__desc">"Користувачів не знайдено."</p> }.into_any()
                        }
                        Ok(list) => {
                            view! {
                                <table>
                                    <thead>
                                        <tr>
                                            <th>"Логін"</th>
                                            <th>"Ім'я"</th>
                                            <th>"Ролі"</th>
                                            <th>"Створено"</th>
                                            <th>"Стан"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {list.into_iter().map(|u| {
                                            let roles_str = u.roles.iter()
                                                .map(|r| format!("{} ({})", r.role, r.org_label))
                                                .collect::<Vec<_>>()
                                                .join(", ");
                                            let status_text = if u.must_change_password {
                                                "тимч. пароль"
                                            } else if u.is_active {
                                                "активний"
                                            } else {
                                                "деактивовано"
                                            };
                                            let status_class = if u.is_active { "" } else { "status-error" };
                                            view! {
                                                <tr>
                                                    <td>{u.login}</td>
                                                    <td>{u.display_name.unwrap_or_else(|| "—".to_string())}</td>
                                                    <td>{roles_str}</td>
                                                    <td>{u.created_at}</td>
                                                    <td class=status_class>{status_text}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }.into_any()
                        }
                        Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                    })
                }}
            </Suspense>
        </div>
    }
}
