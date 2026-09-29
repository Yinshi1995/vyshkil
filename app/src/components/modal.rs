//! Модальне вікно (feedback користувача — HeroUI-референс). Клік-поза-панеллю НЕ закриває
//! навмисно — `training_form::CheatSheet` (перший модал у проєкті, тепер на тих самих
//! `.modal__*`-класах) вже спіймав race: `Show` синхронно демонтує панель ДО завершення
//! спливання click-події, зовнішній `mousedown`-обробник встигає спрацювати на вже скинутому
//! closure ("invoked after being dropped"). Закриття — лише кнопкою "×" або `Escape`.

use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;

#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    children: ChildrenFn,
) -> impl IntoView {
    let handle = window_event_listener(ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" {
            on_close.run(());
        }
    });
    on_cleanup(move || handle.remove());

    view! {
        <Show when=move || open.get()>
            <div class="modal__overlay">
                <div class="modal__panel">
                    <button
                        type="button"
                        class="modal__close"
                        aria-label="Закрити"
                        on:click=move |_| on_close.run(())
                    >
                        <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true">
                            <path d="M1 1 13 13M13 1 1 13" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
                        </svg>
                    </button>
                    {children()}
                </div>
            </div>
        </Show>
    }
}

/// Модальне ДІАЛОГОВЕ вікно — `Modal` + заголовок/опис/рядок дій (підтвердження, попередження).
/// `children` тут — саме кнопки дій ("Підтвердити"/"Скасувати"), не довільний вміст.
#[component]
pub fn Dialog(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] title: String,
    #[prop(optional, into)] description: String,
    children: ChildrenFn,
) -> impl IntoView {
    let has_description = !description.is_empty();
    view! {
        <Modal open=open on_close=on_close>
            <h2 class="modal__title">{title.clone()}</h2>
            {has_description.then(|| view! { <p class="modal__description">{description.clone()}</p> })}
            <div class="modal__actions">{children()}</div>
        </Modal>
    }
}
