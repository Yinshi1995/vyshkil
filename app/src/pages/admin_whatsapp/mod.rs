mod server;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::types::actor::Role;
use crate::widgets::ActorNotice;
use server::{logout_whatsapp, request_pairing_code, send_test_notification};

/// `/admin/whatsapp` (09-messaging.md §4) — стан прив'язки WhatsApp-сесії нотифікатора, лише
/// `admin`. Статус/QR — `EventSource` (SSE, `server::admin_sse`), НЕ Leptos `Resource` (той не
/// стрімить) — підписка/парсинг живуть у цьому файлі, не в окремому хуку (єдиний споживач).
#[component]
pub fn AdminWhatsappPage() -> impl IntoView {
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
        onmessage.forget(); // живе, доки живий EventSource (сторінка) -- той самий компроміс, що інші web_sys-обробники в проєкті.
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
        <PageHeader title="WhatsApp — прив'язка".to_string()/>
        <PageContent width=ContentWidth::Detail>
            {move || {
                let is_admin = actor.get().map(|a| a.role == Role::Admin).unwrap_or(false);
                if actor.get().is_none() {
                    view! { <ActorNotice/> }.into_any()
                } else if !is_admin {
                    view! { <p class="status-error">"Лише адміністратор бачить цю сторінку."</p> }.into_any()
                } else {
                    view! {
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

                        <div class="eyebrow">"Прив'язати за номером"</div>
                        <input
                            type="text"
                            placeholder="380501234567"
                            prop:value=move || phone_input.get()
                            on:input=move |ev| phone_input.set(event_target_value(&ev))
                        />
                        <button class="btn" on:click=do_pair_by_code>"Отримати код"</button>

                        <div class="eyebrow">"Дії"</div>
                        <button class="btn btn--outline" on:click=do_logout>"Відв'язати"</button>
                        <input
                            type="text"
                            placeholder="org_id"
                            prop:value=move || test_org_input.get()
                            on:input=move |ev| test_org_input.set(event_target_value(&ev))
                        />
                        <button class="btn btn--outline" on:click=do_test>"Надіслати тестове"</button>

                        <p>{move || action_status.get()}</p>
                    }
                        .into_any()
                }
            }}
        </PageContent>
    }
}
