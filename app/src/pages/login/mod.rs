use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::services::auth::auth_login;
use crate::types::actor::{Actor, Role};
use crate::types::auth::AuthMode;

#[component]
pub fn LoginPage() -> impl IntoView {
    let login = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let do_login = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let login_val = login.get();
        let password_val = password.get();
        if login_val.is_empty() || password_val.is_empty() {
            error.set(Some("Введіть логін і пароль".to_string()));
            return;
        }
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match auth_login(login_val, password_val).await {
                Ok(resp) if resp.success => {
                    if let Some(auth_mode) = use_context::<RwSignal<AuthMode>>() {
                        auth_mode.set(AuthMode::Auth);
                    }
                    if let Some(display_name_signal) = use_context::<RwSignal<Option<String>>>() {
                        display_name_signal.set(resp.display_name);
                    }
                    if let Some(actor_signal) = use_context::<RwSignal<Option<Actor>>>() {
                        if let Some(first) = resp.roles.first() {
                            let role = Role::parse(&first.role).unwrap_or(Role::Admin);
                            actor_signal.set(Some(Actor { org_id: first.org_id, role }));
                        }
                    }
                    if cfg!(target_arch = "wasm32") {
                        let dest = if resp.must_change_password { "/change-password" } else { "/" };
                        let _ = window().location().set_href(dest);
                    }
                }
                Ok(resp) => {
                    error.set(resp.error.or(Some("Помилка входу".to_string())));
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Помилка: {e}")));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-page">
            <div class="login-card">
                <div class="login-card__header">
                    <span class="login-card__mark">"Т"</span>
                    <h1 class="login-card__title">"Taktoblik"</h1>
                    <p class="login-card__subtitle">"Облік заходів підготовки"</p>
                </div>
                <form class="login-card__form" on:submit=do_login>
                    {move || error.get().map(|e| view! {
                        <div class="login-card__error">{e}</div>
                    })}
                    <div class="login-card__field">
                        <label for="login">"Логін"</label>
                        <input
                            id="login"
                            type="text"
                            autocomplete="username"
                            prop:value=move || login.get()
                            on:input=move |ev| login.set(event_target_value(&ev))
                            disabled=move || loading.get()
                        />
                    </div>
                    <div class="login-card__field">
                        <label for="password">"Пароль"</label>
                        <input
                            id="password"
                            type="password"
                            autocomplete="current-password"
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                            disabled=move || loading.get()
                        />
                    </div>
                    <button
                        type="submit"
                        class="btn btn--primary login-card__submit"
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "Вхід…" } else { "Увійти" }}
                    </button>
                </form>
                <div class="login-card__footer">
                    <p class="login-card__hint">
                        "Немає облікового запису? "
                        <a href="/request-account" class="login-card__link">"Надіслати запит адміністратору"</a>
                    </p>
                </div>
            </div>
        </div>
    }
}
