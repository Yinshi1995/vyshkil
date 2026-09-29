//! Стилізований чекбокс (feedback користувача — HeroUI-референс, своя тема: різкіший радіус,
//! золото замість синього). Справжній `<input type="checkbox">` під візуально прихованим (не
//! `display:none` — лишається фокусованим/клавіатурним) станом, стилізована `.checkbox__box`
//! реагує на `:checked`/`:focus-visible` сусіда через CSS — жодної ручної ARIA-логіки не треба,
//! нативний інпут вже дає клавіатуру/скрінрідер безкоштовно.

use leptos::prelude::*;

#[component]
pub fn Checkbox(
    #[prop(into)] checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
    #[prop(optional, into)] label: String,
) -> impl IntoView {
    view! {
        <label class="checkbox">
            <input
                type="checkbox"
                class="checkbox__input"
                prop:checked=move || checked.get()
                on:change=move |ev| on_change.run(event_target_checked(&ev))
            />
            <span class="checkbox__box">
                <svg class="checkbox__check" viewBox="0 0 12 10" width="12" height="10" aria-hidden="true">
                    <path d="M1 5 4.5 8.5 11 1.5" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
            </span>
            {(!label.is_empty()).then(|| view! { <span class="checkbox__label">{label}</span> })}
        </label>
    }
}
