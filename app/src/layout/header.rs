use leptos::prelude::*;

use super::actor_switcher::ActorSwitcher;
use crate::hooks::use_actor::use_actor;
use crate::services::notifications::{get_unread_count, get_notifications, mark_all_notifications_read};
use crate::types::notification::NotificationRow;

#[component]
pub fn Header() -> impl IntoView {
    let menu_open = RwSignal::new(false);

    view! {
        <header class="app-header">
            <div class="app-header__left">
                <button
                    class="app-header__burger"
                    on:click=move |_| menu_open.update(|v| *v = !*v)
                    aria-label="Меню"
                >
                    <span class=move || if menu_open.get() { "burger-icon burger-icon--open" } else { "burger-icon" }></span>
                </button>
                <a href="/" class="app-header__brand">
                    <span class="app-header__mark">"Т"</span>
                    <span class="app-header__title">"Taktoblik"</span>
                </a>
            </div>
            <nav class="app-header__nav">
                <a href="/training-form">"Навчання"</a>
                <a href="/discrepancies">"Розбіжності"</a>
                <a href="/documents">"Документи"</a>
                <a href="/import">"Імпорт"</a>
            </nav>
            <div class="app-header__actions">
                <NotificationBell/>
                <ActorSwitcher/>
            </div>
        </header>
        <Show when=move || menu_open.get()>
            <div class="mobile-menu" on:click=move |_| menu_open.set(false)>
                <nav class="mobile-menu__nav">
                    <a href="/" class="mobile-menu__link">"Головна"</a>
                    <a href="/training-form" class="mobile-menu__link">"Навчання"</a>
                    <a href="/discrepancies" class="mobile-menu__link">"Розбіжності"</a>
                    <a href="/documents" class="mobile-menu__link">"Документи"</a>
                    <a href="/import" class="mobile-menu__link">"Імпорт"</a>
                    <div class="mobile-menu__divider"></div>
                    <a href="/dictionaries" class="mobile-menu__link mobile-menu__link--secondary">"Довідники"</a>
                    <a href="/vos-lookup" class="mobile-menu__link mobile-menu__link--secondary">"ВОС за ОВТ"</a>
                </nav>
            </div>
        </Show>
    }
}

#[component]
fn NotificationBell() -> impl IntoView {
    let actor = use_actor();
    let open = RwSignal::new(false);
    let refresh = RwSignal::new(0u32);

    let count = Resource::new(
        move || (actor.get(), refresh.get()),
        |(actor, _)| async move { get_unread_count(actor).await.unwrap_or(0) },
    );

    let notifications = Resource::new(
        move || (actor.get(), open.get(), refresh.get()),
        |(actor, is_open, _)| async move {
            if !is_open {
                return Ok(Vec::new());
            }
            get_notifications(actor).await
        },
    );

    let on_mark_all_read = move |_| {
        let actor_val = actor.get();
        leptos::task::spawn_local(async move {
            let _ = mark_all_notifications_read(actor_val).await;
            refresh.update(|c| *c += 1);
        });
    };

    view! {
        <div class="notification-bell">
            <button
                class="notification-bell__btn"
                on:click=move |_| open.update(|v| *v = !*v)
                title="Сповіщення"
            >
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                    <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
                </svg>
                <Suspense>
                    {move || {
                        count
                            .get()
                            .map(|c| {
                                if c > 0 {
                                    Some(view! { <span class="notification-bell__badge">{c}</span> })
                                } else {
                                    None
                                }
                            })
                    }}
                </Suspense>
            </button>
            <Show when=move || open.get()>
                <div class="notification-bell__dropdown">
                    <div class="notification-bell__header">
                        <strong>"Сповіщення"</strong>
                        <button class="btn btn--ghost btn--sm" on:click=on_mark_all_read>
                            "Прочитати всі"
                        </button>
                    </div>
                    <Suspense fallback=|| view! { <p>"…"</p> }>
                        {move || {
                            notifications
                                .get()
                                .map(|res| match res {
                                    Ok(list) if list.is_empty() => {
                                        view! {
                                            <p class="notification-bell__empty">"Сповіщень нема"</p>
                                        }
                                            .into_any()
                                    }
                                    Ok(list) => {
                                        view! { <NotificationList items=list/> }.into_any()
                                    }
                                    Err(_) => view! { <p>"Помилка"</p> }.into_any(),
                                })
                        }}
                    </Suspense>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn NotificationList(items: Vec<NotificationRow>) -> impl IntoView {
    view! {
        <ul class="notification-bell__list">
            {items
                .into_iter()
                .map(|n| {
                    let unread_class = if n.is_read { "" } else { "notification-bell__item--unread" };
                    let title = n.title.clone();
                    let title2 = title.clone();
                    let body = n.body.clone();
                    let created = n.created_at.clone();
                    view! {
                        <li class=format!("notification-bell__item {unread_class}")>
                            {if let Some(ref link) = n.link {
                                view! {
                                    <a href=link.clone() class="notification-bell__link">
                                        <strong>{title}</strong>
                                    </a>
                                }
                                    .into_any()
                            } else {
                                view! { <strong>{title2}</strong> }.into_any()
                            }}
                            {body.map(|b| view! { <p>{b}</p> })}
                            <time>{created}</time>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}
