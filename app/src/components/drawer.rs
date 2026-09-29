//! Бічна панель (feedback користувача — "як Notion", HeroUI-референс) — той самий контракт, що
//! `Modal` (Escape/кнопка закривають, клік-поза — ні, той самий Show-демонтаж-до-bubble race), але
//! панель прикріплена до правого краю на всю висоту й виїжджає збоку, не спливає по центру.

use leptos::prelude::*;

use crate::hooks::use_escape_close::use_escape_close;

#[component]
pub fn Drawer(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    children: ChildrenFn,
) -> impl IntoView {
    use_escape_close(open, on_close);

    view! {
        <Show when=move || open.get()>
            <div class="drawer__overlay">
                <div class="drawer__panel">
                    <button
                        type="button"
                        class="drawer__close"
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
