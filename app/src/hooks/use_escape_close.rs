use leptos::ev;
use leptos::leptos_dom::helpers::window_event_listener;
use leptos::prelude::*;

/// `Escape` закриває, поки `open` — спільний шматок `components::Modal`/`Drawer` (обидва мають
/// той самий "клік-поза НЕ закриває навмисно, лише Escape/кнопка" контракт).
pub fn use_escape_close(open: Signal<bool>, on_close: Callback<()>) {
    let handle = window_event_listener(ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" {
            on_close.run(());
        }
    });
    on_cleanup(move || handle.remove());
}
