mod server;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::types::actor::Role;
use crate::types::auth::AuthMode;
use crate::widgets::ActorNotice;
use server::{
    admin_create_user, admin_list_groups, admin_list_submissions, admin_list_users,
    admin_reset_password, admin_toggle_active, confirm_learned_alias, delete_passkey,
    get_learned_aliases, get_my_account, get_queue_status, list_passkeys, logout_whatsapp,
    reject_learned_alias, request_pairing_code, retry_dlq_entry, send_test_notification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Submissions,
    Groups,
    Users,
    Appearance,
    Account,
    Security,
    Whatsapp,
    Queues,
    Aliases,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::Submissions => "Подання",
            Self::Groups => "Групи",
            Self::Users => "Користувачі",
            Self::Appearance => "Вигляд",
            Self::Account => "Обліковий запис",
            Self::Security => "Безпека",
            Self::Whatsapp => "WhatsApp",
            Self::Queues => "Черги",
            Self::Aliases => "Синоніми",
        }
    }

    fn icon_path(self) -> &'static str {
        match self {
            Self::Submissions => "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
            Self::Groups => "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z",
            Self::Users => "M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z",
            Self::Appearance => "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z",
            Self::Account => "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z",
            Self::Security => "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z",
            Self::Whatsapp => "M21 11.5a8.38 8.38 0 01-.9 3.8 8.5 8.5 0 01-7.6 4.7 8.38 8.38 0 01-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 01-.9-3.8 8.5 8.5 0 014.7-7.6 8.38 8.38 0 013.8-.9h.5a8.48 8.48 0 018 8v.5z",
            Self::Queues => "M4 6h16M4 12h16M4 18h16",
            Self::Aliases => "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2",
        }
    }

    fn all() -> &'static [SettingsTab] {
        &[Self::Submissions, Self::Groups, Self::Users, Self::Appearance, Self::Account, Self::Security, Self::Whatsapp, Self::Queues, Self::Aliases]
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let actor = use_actor();
    let auth_mode = expect_context::<RwSignal<AuthMode>>();
    let default_tab = if actor.get().map(|a| a.role == Role::Admin).unwrap_or(false) {
        SettingsTab::Submissions
    } else {
        SettingsTab::Appearance
    };
    let active_tab = RwSignal::new(default_tab);
    let is_admin = move || actor.get().map(|a| a.role == Role::Admin).unwrap_or(false);
    let is_authenticated = move || auth_mode.get() == AuthMode::Auth;

    view! {
        <PageHeader title="Налаштування".to_string()/>
        <PageContent width=ContentWidth::Detail>
            {move || {
                if actor.get().is_none() {
                    return view! { <ActorNotice/> }.into_any();
                }
                view! {
                    <nav class="settings-tabs">
                        {SettingsTab::all()
                            .iter()
                            .map(|&tab| {
                                let is_active = move || active_tab.get() == tab;
                                let admin_only = matches!(tab, SettingsTab::Submissions | SettingsTab::Groups | SettingsTab::Users | SettingsTab::Whatsapp | SettingsTab::Queues | SettingsTab::Aliases);
                        let auth_only = matches!(tab, SettingsTab::Security | SettingsTab::Account);
                                view! {
                                    <Show when=move || (!admin_only && !auth_only) || (admin_only && is_admin()) || (auth_only && is_authenticated())>
                                        <button
                                            class=move || {
                                                if is_active() {
                                                    "settings-tabs__tab settings-tabs__tab--active"
                                                } else {
                                                    "settings-tabs__tab"
                                                }
                                            }
                                            on:click=move |_| active_tab.set(tab)
                                        >
                                            <svg class="settings-tabs__icon" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                <path d=tab.icon_path()/>
                                            </svg>
                                            {tab.label()}
                                        </button>
                                    </Show>
                                }
                            })
                            .collect_view()}
                    </nav>
                    <div class="settings-panel">
                        <Show when=move || active_tab.get() == SettingsTab::Appearance>
                            <AppearanceSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Account && is_authenticated()>
                            <AccountSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Security && is_authenticated()>
                            <SecuritySection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Submissions && is_admin()>
                            <SubmissionsSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Groups && is_admin()>
                            <GroupsSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Users && is_admin()>
                            <UsersSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Whatsapp && is_admin()>
                            <WhatsappSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Queues && is_admin()>
                            <QueuesSection/>
                        </Show>
                        <Show when=move || active_tab.get() == SettingsTab::Aliases && is_admin()>
                            <LearnedAliasSection/>
                        </Show>
                    </div>
                }
                    .into_any()
            }}
        </PageContent>
    }
}

// ---------------------------------------------------------------------------
// Вигляд (теми)
// ---------------------------------------------------------------------------

#[component]
fn AppearanceSection() -> impl IntoView {
    let current_theme = crate::hooks::use_theme::use_theme();

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Тема оформлення"</div>
            <p class="settings-hint">"Оберіть кольорову схему застосунку."</p>
            <div class="settings-theme-grid">
                {style::theme_names()
                    .into_iter()
                    .map(|name| {
                        let is_active = move || current_theme.get() == name;
                        view! {
                            <button
                                class=move || {
                                    if is_active() {
                                        "settings-theme-btn settings-theme-btn--active"
                                    } else {
                                        "settings-theme-btn"
                                    }
                                }
                                on:click=move |_| current_theme.set(name)
                            >
                                {name}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Обліковий запис
// ---------------------------------------------------------------------------

#[component]
fn AccountSection() -> impl IntoView {
    let account = Resource::new(|| (), |_| get_my_account());

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Мій обліковий запис"</div>
            <p class="settings-hint">"Інформація про ваш обліковий запис у системі."</p>
            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        account.get().map(|res| match res {
                            Ok(info) => {
                                view! {
                                    <table class="settings-account-table">
                                        <tbody>
                                            <tr>
                                                <td class="settings-account-table__label">"Логін"</td>
                                                <td>{info.login}</td>
                                            </tr>
                                            <tr>
                                                <td class="settings-account-table__label">"Повне ім'я"</td>
                                                <td>{info.full_name.unwrap_or_else(|| "—".to_string())}</td>
                                            </tr>
                                            <tr>
                                                <td class="settings-account-table__label">"Ролі"</td>
                                                <td>
                                                    {if info.roles.is_empty() {
                                                        "—".to_string()
                                                    } else {
                                                        info.roles.join(", ")
                                                    }}
                                                </td>
                                            </tr>
                                            <tr>
                                                <td class="settings-account-table__label">"Створено"</td>
                                                <td>{info.created_at}</td>
                                            </tr>
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                            Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                        })
                    }}
                </Suspense>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Безпека (Passkeys / FIDO2) — 12-auth.md §1.3, Étap 10b
// ---------------------------------------------------------------------------

#[component]
fn SecuritySection() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let passkeys = Resource::new(move || refresh.get(), |_| list_passkeys());
    let action_status = RwSignal::new(String::new());

    let do_delete = move |id: i32| {
        spawn_local(async move {
            match delete_passkey(id).await {
                Ok(true) => {
                    action_status.set("Ключ видалено.".to_string());
                    refresh.update(|n| *n += 1);
                }
                Ok(false) => action_status.set("Ключ не знайдено.".to_string()),
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Апаратні ключі (Passkey / FIDO2)"</div>
            <p class="settings-hint">"Апаратний ключ дозволяє входити без пароля — через USB-ключ, телефон або біометрію пристрою."</p>
            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        passkeys.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Немає зареєстрованих ключів."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <table>
                                        <thead>
                                            <tr>
                                                <th>"Назва"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|pk| {
                                                let id = pk.id;
                                                let name = pk.name.unwrap_or_else(|| "Без назви".to_string());
                                                view! {
                                                    <tr>
                                                        <td>{name}</td>
                                                        <td>
                                                            <button
                                                                class="btn btn--outline btn--sm"
                                                                on:click=move |_| do_delete(id)
                                                            >
                                                                "Видалити"
                                                            </button>
                                                        </td>
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

            <div class="card">
                <button
                    class="btn btn--primary"
                    disabled=true
                    title="Потребує webauthn-rs (Étap 10b, ще не реалізовано)"
                >
                    "Додати апаратний ключ"
                </button>
                <p class="settings-hint">"Реєстрація нового ключа буде доступна після інтеграції WebAuthn."</p>
            </div>

            <p class="settings-hint">{move || action_status.get()}</p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// WhatsApp
// ---------------------------------------------------------------------------

#[component]
fn WhatsappSection() -> impl IntoView {
    let actor = use_actor();

    let state = RwSignal::new(String::from("starting"));
    let qr_svg = RwSignal::new(None::<String>);
    let pairing_code = RwSignal::new(None::<String>);
    let phone_masked = RwSignal::new(None::<String>);
    let updated_at = RwSignal::new(String::new());

    Effect::new(move |_| {
        let Some(a) = actor.get() else { return };
        if !cfg!(target_arch = "wasm32") {
            return;
        }
        let url = format!(
            "/api/admin/whatsapp/events?org_id={}&role={}",
            a.org_id,
            a.role.as_str()
        );
        let Ok(es) = web_sys::EventSource::new(&url) else { return };
        let onmessage = Closure::<dyn FnMut(_)>::new(move |ev: web_sys::MessageEvent| {
            let Some(text) = ev.data().as_string() else { return };
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return };
            state.set(v.get("state").and_then(|x| x.as_str()).unwrap_or("").to_string());
            qr_svg.set(v.get("qr_svg").and_then(|x| x.as_str()).map(str::to_string));
            pairing_code.set(v.get("pairing_code").and_then(|x| x.as_str()).map(str::to_string));
            phone_masked.set(v.get("phone_masked").and_then(|x| x.as_str()).map(str::to_string));
            updated_at.set(v.get("updated_at").and_then(|x| x.as_str()).unwrap_or("").to_string());
        });
        es.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    });

    let phone_input = RwSignal::new(String::new());
    let test_org_input = RwSignal::new(String::new());
    let action_status = RwSignal::new(String::new());

    let do_pair_by_code = move |_| {
        let Some(a) = actor.get() else { return };
        let phone = phone_input.get();
        spawn_local(async move {
            match request_pairing_code(Some(a), phone).await {
                Ok(code) => action_status.set(format!("Код прив'язки: {code}")),
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    let do_logout = move |_| {
        let Some(a) = actor.get() else { return };
        spawn_local(async move {
            match logout_whatsapp(Some(a)).await {
                Ok(()) => action_status.set("Відв'язано.".to_string()),
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    let do_test = move |_| {
        let Some(a) = actor.get() else { return };
        let Ok(org_id) = test_org_input.get().parse::<i32>() else {
            action_status.set("Введіть числовий org_id".to_string());
            return;
        };
        spawn_local(async move {
            match send_test_notification(Some(a), org_id).await {
                Ok(r) => action_status.set(format!("Надіслано: {r}")),
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Стан з'єднання"</div>
            <div class="card">
                <p>"Стан: " <strong>{move || state.get()}</strong>
                    " · оновлено " {move || updated_at.get()}
                </p>
                {move || phone_masked.get().map(|p| view! { <p>"Підключено: " {p}</p> })}
                {move || {
                    qr_svg.get().map(|svg| view! {
                        <div class="whatsapp-qr" inner_html=svg></div>
                    })
                }}
                {move || pairing_code.get().map(|c| view! { <p>"Код прив'язки: " <strong>{c}</strong></p> })}
            </div>

            <div class="eyebrow">"Прив'язати за номером"</div>
            <div class="card">
                <div class="settings-form-row">
                    <input
                        type="text"
                        placeholder="380501234567"
                        prop:value=move || phone_input.get()
                        on:input=move |ev| phone_input.set(event_target_value(&ev))
                    />
                    <button class="btn btn--primary" on:click=do_pair_by_code>"Отримати код"</button>
                </div>
            </div>

            <div class="eyebrow">"Дії"</div>
            <div class="card">
                <div class="settings-form-row">
                    <button class="btn btn--outline" on:click=do_logout>"Відв'язати"</button>
                    <input
                        type="text"
                        placeholder="org_id"
                        prop:value=move || test_org_input.get()
                        on:input=move |ev| test_org_input.set(event_target_value(&ev))
                    />
                    <button class="btn btn--outline" on:click=do_test>"Надіслати тестове"</button>
                </div>
                <p class="settings-hint">{move || action_status.get()}</p>
            </div>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Черги
// ---------------------------------------------------------------------------

#[component]
fn QueuesSection() -> impl IntoView {
    let actor = use_actor();
    let refresh = RwSignal::new(0u32);
    let status = Resource::new(move || (actor.get(), refresh.get()), |(a, _)| get_queue_status(a));
    let action_status = RwSignal::new(String::new());

    let do_retry = move |seq: u64| {
        let Some(a) = actor.get() else { return };
        spawn_local(async move {
            match retry_dlq_entry(Some(a), seq).await {
                Ok(()) => {
                    action_status.set(format!("Повторено запис #{seq}."));
                    refresh.update(|n| *n += 1);
                }
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="settings-form-row">
                <button class="btn btn--outline" on:click=move |_| refresh.update(|n| *n += 1)>
                    "Оновити"
                </button>
            </div>
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    status.get().map(|res| match res {
                        Ok(s) => {
                            if !s.nats_connected {
                                view! { <p class="status-error">"NATS недоступний."</p> }.into_any()
                            } else {
                                view! {
                                    <div class="eyebrow">"Outbox"</div>
                                    <div class="card">
                                        <p>
                                            "Непубліковано: " <strong>{s.outbox.unpublished_count}</strong>
                                            {s.outbox.oldest_unpublished_at.map(|t| format!(" · найстаріший: {t}"))}
                                        </p>
                                    </div>

                                    <div class="eyebrow">"Стріми"</div>
                                    <div class="card">
                                        <ul>
                                            {s.streams.into_iter().map(|st| {
                                                let label = match st.messages {
                                                    Some(n) => format!("{}: {} повідомлень", st.name, n),
                                                    None => format!("{}: ще не створено", st.name),
                                                };
                                                view! { <li>{label}</li> }
                                            }).collect_view()}
                                        </ul>
                                    </div>

                                    <div class="eyebrow">{format!("DLQ ({})", s.dlq.len())}</div>
                                    <div class="card">
                                        <ul>
                                            {s.dlq.into_iter().map(|entry| {
                                                let seq = entry.seq;
                                                view! {
                                                    <li>
                                                        {format!("#{} {} — {} ({})", entry.seq, entry.original_subject, entry.reason, entry.failed_at)}
                                                        " "
                                                        <button class="btn btn--outline btn--sm" on:click=move |_| do_retry(seq)>
                                                            "Повторити"
                                                        </button>
                                                    </li>
                                                }
                                            }).collect_view()}
                                        </ul>
                                    </div>
                                }
                                    .into_any()
                            }
                        }
                        Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                    })
                }}
            </Suspense>
            <p class="settings-hint">{move || action_status.get()}</p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Подання (admin)
// ---------------------------------------------------------------------------

#[component]
fn SubmissionsSection() -> impl IntoView {
    let actor = use_actor();
    let submissions = Resource::new(
        move || actor.get(),
        admin_list_submissions,
    );

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Подання підрозділів"</div>
            <p class="settings-hint">"Усі зафіксовані подання вашого підрозділу та підпорядкованих частин."</p>
            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        submissions.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Подань немає."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
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
                                                let source_label = match s.source_type.as_str() {
                                                    "form" => "Форма",
                                                    "table" => "Таблиця",
                                                    "official_letter" => "Лист",
                                                    "scan" => "Скан",
                                                    "archive_seed" => "Архів",
                                                    _ => "?",
                                                };
                                                let status_class = match s.status.as_str() {
                                                    "committed" | "validated" => "status-ok",
                                                    "rejected" => "status-error",
                                                    _ => "",
                                                };
                                                let status_label = match s.status.as_str() {
                                                    "committed" => "Зафіксовано",
                                                    "validated" => "Перевірено",
                                                    "parsed" => "Розібрано",
                                                    "rejected" => "Відхилено",
                                                    _ => "?",
                                                };
                                                view! {
                                                    <tr>
                                                        <td>{s.org_label}</td>
                                                        <td>{source_label}</td>
                                                        <td class=status_class>{status_label}</td>
                                                        <td>{s.as_of_date}</td>
                                                        <td>{s.updated_at}</td>
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
        </div>
    }
}

// ---------------------------------------------------------------------------
// Групи на навчанні (admin)
// ---------------------------------------------------------------------------

#[component]
fn GroupsSection() -> impl IntoView {
    let actor = use_actor();
    let groups = Resource::new(
        move || actor.get(),
        admin_list_groups,
    );

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Групи на навчанні"</div>
            <p class="settings-hint">"Усі групи підготовки вашого підрозділу та підпорядкованих частин."</p>
            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        groups.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Груп немає."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <table>
                                        <thead>
                                            <tr>
                                                <th>"Підрозділ"</th>
                                                <th>"Вид"</th>
                                                <th>"ВОС/Посада/Курс"</th>
                                                <th>"Місце"</th>
                                                <th>"Термін"</th>
                                                <th>"План"</th>
                                                <th>"Приб."</th>
                                                <th>"Навч."</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|g| {
                                                let period = if g.planned_start.is_empty() && g.planned_end.is_empty() {
                                                    "—".to_string()
                                                } else {
                                                    format!("{} — {}", g.planned_start, g.planned_end)
                                                };
                                                view! {
                                                    <tr>
                                                        <td>{g.org_label}</td>
                                                        <td>{g.training_kind}</td>
                                                        <td>{g.vos_label}</td>
                                                        <td>{g.site_label}</td>
                                                        <td>{period}</td>
                                                        <td>{g.planned_count}</td>
                                                        <td>{g.arrived_count}</td>
                                                        <td>{g.in_training_count}</td>
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
        </div>
    }
}

// ---------------------------------------------------------------------------
// Користувачі (admin)
// ---------------------------------------------------------------------------

#[component]
fn UsersSection() -> impl IntoView {
    let actor = use_actor();
    let refresh = RwSignal::new(0u32);
    let users = Resource::new(
        move || (actor.get(), refresh.get()),
        |(a, _)| admin_list_users(a),
    );
    let action_status = RwSignal::new(String::new());
    let show_create = RwSignal::new(false);
    let reset_user_id = RwSignal::new(None::<i32>);
    let reset_password_val = RwSignal::new(String::new());

    let new_login = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_display = RwSignal::new(String::new());
    let new_org_id = RwSignal::new(String::new());
    let new_role = RwSignal::new("org_editor".to_string());

    let orgs = Resource::new(|| (), |_| crate::services::orgs::list_orgs());

    let do_create = move |_| {
        let Some(a) = actor.get() else { return };
        let login = new_login.get();
        let password = new_password.get();
        let display = new_display.get();
        let Ok(org_id) = new_org_id.get().parse::<i32>() else {
            action_status.set("Оберіть підрозділ".to_string());
            return;
        };
        let role = new_role.get();
        spawn_local(async move {
            match admin_create_user(Some(a), login, password, display, org_id, role).await {
                Ok(id) => {
                    action_status.set(format!("Користувача створено (id={id})"));
                    show_create.set(false);
                    new_login.set(String::new());
                    new_password.set(String::new());
                    new_display.set(String::new());
                    refresh.update(|n| *n += 1);
                }
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    let do_reset = move |_| {
        let Some(a) = actor.get() else { return };
        let Some(uid) = reset_user_id.get() else { return };
        let pw = reset_password_val.get();
        spawn_local(async move {
            match admin_reset_password(Some(a), uid, pw).await {
                Ok(()) => {
                    action_status.set("Пароль скинуто. Користувач повинен змінити його при вході.".to_string());
                    reset_user_id.set(None);
                    reset_password_val.set(String::new());
                    refresh.update(|n| *n += 1);
                }
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    let do_toggle = move |uid: i32, active: bool| {
        let Some(a) = actor.get() else { return };
        spawn_local(async move {
            match admin_toggle_active(Some(a), uid, active).await {
                Ok(()) => {
                    refresh.update(|n| *n += 1);
                }
                Err(e) => action_status.set(format!("Помилка: {e}")),
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Користувачі системи"</div>
            <p class="settings-hint">"Перегляд та управління обліковими записами."</p>

            <div class="settings-form-row">
                <button class="btn btn--primary" on:click=move |_| show_create.update(|v| *v = !*v)>
                    {move || if show_create.get() { "Скасувати" } else { "Створити користувача" }}
                </button>
            </div>

            <Show when=move || show_create.get()>
                <div class="card">
                    <div class="eyebrow">"Новий користувач"</div>
                    <div class="settings-form-row">
                        <input type="text" placeholder="Логін"
                            prop:value=move || new_login.get()
                            on:input=move |ev| new_login.set(event_target_value(&ev))
                        />
                        <input type="password" placeholder="Тимчасовий пароль"
                            prop:value=move || new_password.get()
                            on:input=move |ev| new_password.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="settings-form-row">
                        <input type="text" placeholder="Відображуване ім'я (необов'язково)"
                            prop:value=move || new_display.get()
                            on:input=move |ev| new_display.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="settings-form-row">
                        <Suspense fallback=|| view! { <span>"…"</span> }>
                            {move || orgs.get().map(|res| match res {
                                Ok(list) => view! {
                                    <select
                                        prop:value=move || new_org_id.get()
                                        on:change=move |ev| new_org_id.set(event_target_value(&ev))
                                    >
                                        <option value="">"— Оберіть підрозділ —"</option>
                                        {list.into_iter().map(|(id, name)| {
                                            let id_str = id.to_string();
                                            view! { <option value=id_str.clone()>{name}</option> }
                                        }).collect_view()}
                                    </select>
                                }.into_any(),
                                Err(_) => view! { <span>"Помилка завантаження"</span> }.into_any(),
                            })}
                        </Suspense>
                        <select
                            prop:value=move || new_role.get()
                            on:change=move |ev| new_role.set(event_target_value(&ev))
                        >
                            <option value="org_editor">"Редактор"</option>
                            <option value="viewer">"Спостерігач"</option>
                            <option value="admin">"Адміністратор"</option>
                        </select>
                    </div>
                    <div class="settings-form-row">
                        <button class="btn btn--primary" on:click=do_create>"Створити"</button>
                    </div>
                </div>
            </Show>

            <Show when=move || reset_user_id.get().is_some()>
                <div class="card">
                    <div class="eyebrow">"Скидання пароля"</div>
                    <div class="settings-form-row">
                        <input type="password" placeholder="Новий тимчасовий пароль"
                            prop:value=move || reset_password_val.get()
                            on:input=move |ev| reset_password_val.set(event_target_value(&ev))
                        />
                        <button class="btn btn--primary" on:click=do_reset>"Скинути"</button>
                        <button class="btn btn--outline" on:click=move |_| reset_user_id.set(None)>"Скасувати"</button>
                    </div>
                </div>
            </Show>

            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        users.get().map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Немає користувачів."</p> }.into_any()
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
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list.into_iter().map(|u| {
                                                let uid = u.id;
                                                let is_active = u.is_active;
                                                let roles_str = u.roles.iter()
                                                    .map(|r| format!("{} ({})", r.role, r.org_label))
                                                    .collect::<Vec<_>>()
                                                    .join(", ");
                                                let status_class = if u.is_active { "" } else { "status-error" };
                                                let status_text = if u.must_change_password {
                                                    "тимч. пароль"
                                                } else if u.is_active {
                                                    "активний"
                                                } else {
                                                    "деактивовано"
                                                };
                                                view! {
                                                    <tr>
                                                        <td>{u.login}</td>
                                                        <td>{u.display_name.unwrap_or_else(|| "—".to_string())}</td>
                                                        <td>{roles_str}</td>
                                                        <td>{u.created_at}</td>
                                                        <td class=status_class>{status_text}</td>
                                                        <td>
                                                            <button
                                                                class="btn btn--outline btn--sm"
                                                                on:click=move |_| {
                                                                    reset_user_id.set(Some(uid));
                                                                    reset_password_val.set(String::new());
                                                                }
                                                            >
                                                                "Скинути пароль"
                                                            </button>
                                                            " "
                                                            <button
                                                                class="btn btn--outline btn--sm"
                                                                on:click=move |_| do_toggle(uid, !is_active)
                                                            >
                                                                {if is_active { "Деактивувати" } else { "Активувати" }}
                                                            </button>
                                                        </td>
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

            <p class="settings-hint">{move || action_status.get()}</p>
        </div>
    }
}

// ---------------------------------------------------------------------------
// Learned-синоніми
// ---------------------------------------------------------------------------

#[component]
fn LearnedAliasSection() -> impl IntoView {
    let actor = use_actor();
    let refresh = RwSignal::new(0u32);
    let queue = Resource::new(
        move || (actor.get(), refresh.get()),
        |(actor, _)| async move { get_learned_aliases(actor).await },
    );

    let on_confirm = move |id: i32| {
        spawn_local(async move {
            if confirm_learned_alias(actor.get(), id).await.is_ok() {
                refresh.update(|n| *n += 1);
            }
        });
    };
    let on_reject = move |id: i32| {
        spawn_local(async move {
            if reject_learned_alias(actor.get(), id).await.is_ok() {
                refresh.update(|n| *n += 1);
            }
        });
    };

    view! {
        <div class="settings-section">
            <div class="eyebrow">"Learned-синоніми на підтвердження"</div>
            <p class="settings-hint">"Синоніми, які система вивчила автоматично з імпортованих файлів. Підтвердіть або відхиліть кожен."</p>
            <div class="card">
                <Suspense fallback=|| view! { <p>"…"</p> }>
                    {move || {
                        queue
                            .get()
                            .map(|res| match res {
                                Ok(list) if list.is_empty() => {
                                    view! {
                                        <div class="settings-empty">
                                            <div class="settings-empty__icon">
                                                <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                                                    <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2"/>
                                                    <path d="M9 5a2 2 0 002 2h2a2 2 0 002-2"/>
                                                    <path d="M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                                                </svg>
                                            </div>
                                            <p class="settings-empty__text">"Черга порожня — усі синоніми оброблено."</p>
                                        </div>
                                    }
                                        .into_any()
                                }
                                Ok(list) => {
                                    view! {
                                        <table class="settings-aliases-table">
                                            <thead>
                                                <tr>
                                                    <th>"Тип"</th>
                                                    <th>"Сире значення"</th>
                                                    <th>"Використань"</th>
                                                    <th>"Створено"</th>
                                                    <th></th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {list
                                                    .into_iter()
                                                    .map(|a| {
                                                        let id = a.id;
                                                        view! {
                                                            <tr>
                                                                <td>{a.target_type}</td>
                                                                <td>{a.raw}</td>
                                                                <td>{a.uses_count}</td>
                                                                <td>{a.created_at}</td>
                                                                <td>
                                                                    <button
                                                                        class="btn btn--primary btn--sm"
                                                                        on:click=move |_| on_confirm(id)
                                                                    >
                                                                        "Підтвердити"
                                                                    </button>
                                                                    " "
                                                                    <button
                                                                        class="btn btn--outline btn--sm"
                                                                        on:click=move |_| on_reject(id)
                                                                    >
                                                                        "Відхилити"
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </tbody>
                                        </table>
                                    }
                                        .into_any()
                                }
                                Err(e) => view! { <p class="card__desc status-error">{e.to_string()}</p> }.into_any(),
                            })
                    }}
                </Suspense>
            </div>
        </div>
    }
}
