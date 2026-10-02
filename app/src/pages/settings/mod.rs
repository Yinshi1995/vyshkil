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
    confirm_learned_alias, delete_passkey, get_learned_aliases, get_queue_status, list_passkeys,
    logout_whatsapp, reject_learned_alias, request_pairing_code, retry_dlq_entry,
    send_test_notification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    Appearance,
    Security,
    Whatsapp,
    Queues,
    Aliases,
}

impl SettingsTab {
    fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Вигляд",
            Self::Security => "Безпека",
            Self::Whatsapp => "WhatsApp",
            Self::Queues => "Черги",
            Self::Aliases => "Синоніми",
        }
    }

    fn icon_path(self) -> &'static str {
        match self {
            Self::Appearance => "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z",
            Self::Security => "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z",
            Self::Whatsapp => "M21 11.5a8.38 8.38 0 01-.9 3.8 8.5 8.5 0 01-7.6 4.7 8.38 8.38 0 01-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 01-.9-3.8 8.5 8.5 0 014.7-7.6 8.38 8.38 0 013.8-.9h.5a8.48 8.48 0 018 8v.5z",
            Self::Queues => "M4 6h16M4 12h16M4 18h16",
            Self::Aliases => "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2",
        }
    }

    fn all() -> &'static [SettingsTab] {
        &[Self::Appearance, Self::Security, Self::Whatsapp, Self::Queues, Self::Aliases]
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let actor = use_actor();
    let auth_mode = expect_context::<RwSignal<AuthMode>>();
    let active_tab = RwSignal::new(SettingsTab::Appearance);
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
                                let admin_only = matches!(tab, SettingsTab::Whatsapp | SettingsTab::Queues | SettingsTab::Aliases);
                        let auth_only = matches!(tab, SettingsTab::Security);
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
                        <Show when=move || active_tab.get() == SettingsTab::Security && is_authenticated()>
                            <SecuritySection/>
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

fn set_theme(name: &str) {
    if !cfg!(target_arch = "wasm32") {
        return;
    }
    if let Some(el) = document().document_element() {
        let _ = el.set_attribute("data-theme", name);
    }
}

#[component]
fn AppearanceSection() -> impl IntoView {
    let current_theme = RwSignal::new("night");
    Effect::new(move |_| set_theme(current_theme.get()));

    view! {
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
        <div class="eyebrow">"Learned-синоніми на підтвердження"</div>
        <div class="card">
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    queue
                        .get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Немає learned-синонімів на підтвердження."</p> }
                                    .into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <table>
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
    }
}
