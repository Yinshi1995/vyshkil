mod components;
mod server;

use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::widgets::ActorNotice;
use components::{OrgSearch, SubordinationTree};
use server::{get_dashboard_stats, get_recent_submissions};

#[component]
pub fn HomePage() -> impl IntoView {
    let actor = use_actor();

    let stats = Resource::new(move || actor.get(), |a| async move { get_dashboard_stats(a).await });

    let recent = Resource::new(
        move || actor.get(),
        |a| async move { get_recent_submissions(a).await },
    );

    view! {
        <PageHeader
            title="Taktoblik".to_string()
            subtitle="Облік заходів підготовки військових частин".to_string()
        />
        <PageContent width=ContentWidth::Detail>
            <Show when=move || actor.get().is_none()>
                <ActorNotice/>
            </Show>
            <Show when=move || actor.get().is_some()>
                // Stats row
                <div class="dash-stats">
                    <Suspense fallback=|| view! { <DashStatSkeleton/> }>
                        {move || {
                            stats
                                .get()
                                .map(|res| match res {
                                    Ok(s) => {
                                        view! {
                                            <div class="dash-stat">
                                                <div class="dash-stat__value">{s.org_count}</div>
                                                <div class="dash-stat__label">"Частин"</div>
                                            </div>
                                            <div class="dash-stat">
                                                <div class="dash-stat__value">{s.training_group_count}</div>
                                                <div class="dash-stat__label">"Груп навчання"</div>
                                            </div>
                                            <div class="dash-stat">
                                                <div class="dash-stat__value">{s.committed_submission_count}</div>
                                                <div class="dash-stat__label">"Подань"</div>
                                            </div>
                                            <div class="dash-stat">
                                                <div class="dash-stat__value">{s.open_discrepancy_count}</div>
                                                <div class="dash-stat__label">"Розбіжностей"</div>
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
                </div>

                // Quick actions
                <div class="dash-actions">
                    <a href="/training-form" class="dash-action">
                        <div class="dash-action__icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                <path d="M12 6.042A8.967 8.967 0 006 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 016 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 016-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0018 18a8.967 8.967 0 00-6 2.292m0-14.25v14.25"/>
                            </svg>
                        </div>
                        <div class="dash-action__text">
                            <div class="dash-action__title">"Навчання"</div>
                            <div class="dash-action__desc">"Внести або імпортувати дані груп навчання"</div>
                        </div>
                    </a>
                    <a href="/import" class="dash-action">
                        <div class="dash-action__icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                <path d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5"/>
                            </svg>
                        </div>
                        <div class="dash-action__text">
                            <div class="dash-action__title">"Імпорт"</div>
                            <div class="dash-action__desc">"Завантажити дані з файлів КВід, ІВС, Архів ВЧ"</div>
                        </div>
                    </a>
                    <a href="/documents" class="dash-action">
                        <div class="dash-action__icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                <path d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z"/>
                            </svg>
                        </div>
                        <div class="dash-action__text">
                            <div class="dash-action__title">"Документи"</div>
                            <div class="dash-action__desc">"Згенерувати звітні документи D1–D6"</div>
                        </div>
                    </a>
                    <a href="/discrepancies" class="dash-action">
                        <div class="dash-action__icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                <path d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"/>
                            </svg>
                        </div>
                        <div class="dash-action__text">
                            <div class="dash-action__title">"Розбіжності"</div>
                            <div class="dash-action__desc">"Переглянути та вирішити розбіжності між поданнями"</div>
                        </div>
                    </a>
                </div>

                // Recent submissions
                <div class="eyebrow">"Останні подання"</div>
                <div class="card">
                    <Suspense fallback=|| view! { <p>"Завантаження…"</p> }>
                        {move || {
                            recent
                                .get()
                                .map(|res| match res {
                                    Ok(list) if list.is_empty() => {
                                        view! {
                                            <div class="dash-empty">
                                                <p>"Поки що подань немає."</p>
                                                <a href="/training-form" class="btn btn--accent btn--sm">
                                                    "Створити перше подання"
                                                </a>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    Ok(list) => {
                                        view! {
                                            <table class="dash-table">
                                                <thead>
                                                    <tr>
                                                        <th>"Частина"</th>
                                                        <th>"Тип"</th>
                                                        <th>"Статус"</th>
                                                        <th>"Оновлено"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {list
                                                        .into_iter()
                                                        .map(|s| {
                                                            let status_cls = match s.status.as_str() {
                                                                "committed" => "badge badge--ok",
                                                                "draft" => "badge badge--warn",
                                                                _ => "badge",
                                                            };
                                                            let type_label = match s.source_type.as_str() {
                                                                "form" => "Форма",
                                                                "table" => "Таблиця",
                                                                _ => &s.source_type,
                                                            };
                                                            let status_label = match s.status.as_str() {
                                                                "committed" => "Зафіксовано",
                                                                "draft" => "Чернетка",
                                                                _ => &s.status,
                                                            };
                                                            view! {
                                                                <tr>
                                                                    <td>{s.org_label}</td>
                                                                    <td>{type_label.to_string()}</td>
                                                                    <td>
                                                                        <span class=status_cls>{status_label.to_string()}</span>
                                                                    </td>
                                                                    <td>{s.updated_at}</td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        }
                                            .into_any()
                                    }
                                    Err(e) => {
                                        view! { <p class="status-error">{e.to_string()}</p> }.into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </Show>

            // Org search and tree — always shown
            <OrgSearch/>
            <SubordinationTree/>
        </PageContent>
    }
}

#[component]
fn DashStatSkeleton() -> impl IntoView {
    view! {
        <div class="dash-stat"><div class="dash-stat__value skeleton">"—"</div><div class="dash-stat__label skeleton">" "</div></div>
        <div class="dash-stat"><div class="dash-stat__value skeleton">"—"</div><div class="dash-stat__label skeleton">" "</div></div>
        <div class="dash-stat"><div class="dash-stat__value skeleton">"—"</div><div class="dash-stat__label skeleton">" "</div></div>
        <div class="dash-stat"><div class="dash-stat__value skeleton">"—"</div><div class="dash-stat__label skeleton">" "</div></div>
    }
}
