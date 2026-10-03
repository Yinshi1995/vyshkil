use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::services::auth::change_password;

#[component]
pub fn ChangePasswordPage() -> impl IntoView {
    let old_pw = RwSignal::new(String::new());
    let new_pw = RwSignal::new(String::new());
    let confirm_pw = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let success = RwSignal::new(false);
    let loading = RwSignal::new(false);

    let do_change = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let old = old_pw.get();
        let new = new_pw.get();
        let confirm = confirm_pw.get();

        if old.is_empty() || new.is_empty() {
            error.set(Some("Заповніть усі поля".to_string()));
            return;
        }
        if new.len() < 6 {
            error.set(Some("Новий пароль занадто короткий (мін. 6 символів)".to_string()));
            return;
        }
        if new != confirm {
            error.set(Some("Паролі не збігаються".to_string()));
            return;
        }

        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match change_password(old, new).await {
                Ok(()) => {
                    success.set(true);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
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
                    <h1 class="login-card__title">"Зміна пароля"</h1>
                    <p class="login-card__subtitle">"Встановіть постійний пароль для вашого облікового запису"</p>
                </div>
                <Show when=move || success.get()
                    fallback=move || view! {
                        <form class="login-card__form" on:submit=do_change>
                            {move || error.get().map(|e| view! {
                                <div class="login-card__error">{e}</div>
                            })}
                            <div class="login-card__field">
                                <label for="old_password">"Поточний (тимчасовий) пароль"</label>
                                <input
                                    id="old_password"
                                    type="password"
                                    autocomplete="current-password"
                                    prop:value=move || old_pw.get()
                                    on:input=move |ev| old_pw.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <div class="login-card__field">
                                <label for="new_password">"Новий пароль"</label>
                                <input
                                    id="new_password"
                                    type="password"
                                    autocomplete="new-password"
                                    prop:value=move || new_pw.get()
                                    on:input=move |ev| new_pw.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <div class="login-card__field">
                                <label for="confirm_password">"Підтвердіть новий пароль"</label>
                                <input
                                    id="confirm_password"
                                    type="password"
                                    autocomplete="new-password"
                                    prop:value=move || confirm_pw.get()
                                    on:input=move |ev| confirm_pw.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <button
                                type="submit"
                                class="btn btn--primary login-card__submit"
                                disabled=move || loading.get()
                            >
                                {move || if loading.get() { "Збереження…" } else { "Зберегти пароль" }}
                            </button>
                        </form>
                    }
                >
                    <div class="login-card__form">
                        <div class="login-card__success">"Пароль успішно змінено!"</div>
                        <a href="/" class="btn btn--primary login-card__submit">"Перейти до системи"</a>
                    </div>
                </Show>
            </div>
        </div>
    }
}
