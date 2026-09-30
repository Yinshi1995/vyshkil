mod server;

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::hooks::use_actor::use_actor;
use crate::layout::{ContentWidth, PageContent, PageHeader};
use crate::types::actor::Role;
use crate::widgets::ActorNotice;
use server::{get_queue_status, retry_dlq_entry};

/// `/admin/queues` (09-messaging.md §5, Фаза 4) — відставання outbox, стан стрімів, DLQ з
/// повторною відправкою. Лише `admin`. Звичайний `Resource` (не SSE, на відміну від
/// `/admin/whatsapp` — тут немає "живого" стану, що постійно змінюється, ручне оновлення досить).
#[component]
pub fn AdminQueuesPage() -> impl IntoView {
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
        <PageHeader title="Черги".to_string()/>
        <PageContent width=ContentWidth::Detail>
            {move || {
                let is_admin = actor.get().map(|a| a.role == Role::Admin).unwrap_or(false);
                if actor.get().is_none() {
                    view! { <ActorNotice/> }.into_any()
                } else if !is_admin {
                    view! { <p class="status-error">"Лише адміністратор бачить цю сторінку."</p> }.into_any()
                } else {
                    view! {
                        <button class="btn btn--outline" on:click=move |_| refresh.update(|n| *n += 1)>
                            "Оновити"
                        </button>
                        <Suspense fallback=|| view! { <p>"…"</p> }>
                            {move || {
                                status.get().map(|res| match res {
                                    Ok(s) => {
                                        if !s.nats_connected {
                                            view! { <p class="status-error">"NATS недоступний."</p> }.into_any()
                                        } else {
                                            view! {
                                                <div class="eyebrow">"Outbox"</div>
                                                <p>
                                                    "Непубліковано: " <strong>{s.outbox.unpublished_count}</strong>
                                                    {s.outbox.oldest_unpublished_at.map(|t| format!(" · найстаріший: {t}"))}
                                                </p>

                                                <div class="eyebrow">"Стріми"</div>
                                                <ul>
                                                    {s.streams.into_iter().map(|st| {
                                                        let label = match st.messages {
                                                            Some(n) => format!("{}: {} повідомлень", st.name, n),
                                                            None => format!("{}: ще не створено", st.name),
                                                        };
                                                        view! { <li>{label}</li> }
                                                    }).collect_view()}
                                                </ul>

                                                <div class="eyebrow">{format!("DLQ ({})", s.dlq.len())}</div>
                                                <ul>
                                                    {s.dlq.into_iter().map(|entry| {
                                                        let seq = entry.seq;
                                                        view! {
                                                            <li>
                                                                {format!("#{} {} — {} ({})", entry.seq, entry.original_subject, entry.reason, entry.failed_at)}
                                                                <button class="btn btn--outline" on:click=move |_| do_retry(seq)>
                                                                    "Повторити"
                                                                </button>
                                                            </li>
                                                        }
                                                    }).collect_view()}
                                                </ul>
                                            }
                                                .into_any()
                                        }
                                    }
                                    Err(e) => view! { <p class="status-error">{e.to_string()}</p> }.into_any(),
                                })
                            }}
                        </Suspense>
                        <p>{move || action_status.get()}</p>
                    }
                        .into_any()
                }
            }}
        </PageContent>
    }
}
