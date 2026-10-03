use leptos::prelude::*;
use leptos::task::spawn_local;

mod server;
pub use server::request_account;

#[component]
pub fn RequestAccountPage() -> impl IntoView {
    let contact = RwSignal::new(String::new());
    let unit = RwSignal::new(String::new());
    let message = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let sent = RwSignal::new(false);
    let loading = RwSignal::new(false);

    let do_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let c = contact.get();
        if c.trim().is_empty() {
            error.set(Some("Вкажіть контактні дані".to_string()));
            return;
        }
        loading.set(true);
        error.set(None);
        let u = unit.get();
        let m = message.get();
        spawn_local(async move {
            match request_account(c, u, m).await {
                Ok(()) => sent.set(true),
                Err(e) => {
                    error.set(Some(format!("{e}")));
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
                    <h1 class="login-card__title">"Запит облікового запису"</h1>
                    <p class="login-card__subtitle">"Повідомлення буде надіслано адміністратору"</p>
                </div>
                <Show
                    when=move || sent.get()
                    fallback=move || view! {
                        <form class="login-card__form" on:submit=do_submit>
                            {move || error.get().map(|e| view! {
                                <div class="login-card__error">{e}</div>
                            })}
                            <div class="login-card__field">
                                <label for="contact">"Контактні дані (телефон, позивний)"</label>
                                <input
                                    id="contact"
                                    type="text"
                                    prop:value=move || contact.get()
                                    on:input=move |ev| contact.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <div class="login-card__field">
                                <label for="unit">"Частина (необов'язково)"</label>
                                <input
                                    id="unit"
                                    type="text"
                                    prop:value=move || unit.get()
                                    on:input=move |ev| unit.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <div class="login-card__field">
                                <label for="message">"Коментар (необов'язково)"</label>
                                <textarea
                                    id="message"
                                    rows="3"
                                    prop:value=move || message.get()
                                    on:input=move |ev| message.set(event_target_value(&ev))
                                    disabled=move || loading.get()
                                />
                            </div>
                            <button
                                type="submit"
                                class="btn btn--primary login-card__submit"
                                disabled=move || loading.get()
                            >
                                {move || if loading.get() { "Надсилання…" } else { "Надіслати запит" }}
                            </button>
                        </form>
                    }
                >
                    <div class="login-card__success">
                        <p>"Запит надіслано. Адміністратор зв'яжеться з вами."</p>
                    </div>
                </Show>
                <div class="login-card__footer">
                    <p class="login-card__hint">
                        <a href="/login" class="login-card__link">"← Повернутися до входу"</a>
                    </p>
                </div>
            </div>
        </div>
    }
}
