use leptos::prelude::*;

use super::actor_switcher::ActorSwitcher;
use crate::hooks::use_actor::use_actor;
use crate::hooks::use_theme::{next_theme, use_theme};
use crate::services::notifications::{get_unread_count, get_notifications, mark_all_notifications_read};
use crate::types::notification::NotificationRow;

#[component]
pub fn Header() -> impl IntoView {
    let actor = use_actor();
    let menu_open = RwSignal::new(false);
    let org_href = move || {
        actor.get().map(|a| format!("/org/{}", a.org_id)).unwrap_or_default()
    };

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
                <Show when=move || actor.get().is_some()>
                    <a href=org_href>"Мій підрозділ"</a>
                </Show>
                <a href="/training-form">"Навчання"</a>
                <a href="/discrepancies">"Розбіжності"</a>
                <a href="/documents">"Документи"</a>
                <a href="/import">"Імпорт"</a>
            </nav>
            <div class="app-header__actions">
                <NotificationBell/>
                <ThemeToggle/>
                <a href="/settings" class="app-header__settings" title="Налаштування">
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <circle cx="12" cy="12" r="3"/>
                        <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.32 9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
                    </svg>
                </a>
                <ActorSwitcher/>
            </div>
        </header>
        <Show when=move || menu_open.get()>
            <div class="mobile-menu" on:click=move |_| menu_open.set(false)>
                <nav class="mobile-menu__nav">
                    <a href="/" class="mobile-menu__link">"Головна"</a>
                    <Show when=move || actor.get().is_some()>
                        <a href=org_href class="mobile-menu__link">"Мій підрозділ"</a>
                    </Show>
                    <a href="/training-form" class="mobile-menu__link">"Навчання"</a>
                    <a href="/discrepancies" class="mobile-menu__link">"Розбіжності"</a>
                    <a href="/documents" class="mobile-menu__link">"Документи"</a>
                    <a href="/import" class="mobile-menu__link">"Імпорт"</a>
                    <a href="/settings" class="mobile-menu__link">"Налаштування"</a>
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

/// Кнопка перемикання теми (night → day → night).
#[component]
fn ThemeToggle() -> impl IntoView {
    let theme = use_theme();

    let icon_title = move || match theme.get() {
        "night" => "Тема: нічна",
        "day" => "Тема: денна",
        _ => "Тема",
    };

    let icon_path = move || match theme.get() {
        "night" => "M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z",
        _ => "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z",
    };

    view! {
        <button
            class="theme-toggle"
            title=icon_title
            on:click=move |_| theme.set(next_theme(theme.get()))
        >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d=icon_path/>
            </svg>
        </button>
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
