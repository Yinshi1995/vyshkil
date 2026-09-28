use leptos::prelude::*;

use super::actor_switcher::ActorSwitcher;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="app-header">
            <div class="app-header__brand">
                <span class="app-header__mark">"Т"</span>
                <span class="app-header__title">"Taktoblik"</span>
            </div>
            <nav class="app-header__nav">
                <a href="/">"Головна"</a>
                <a href="/vos-lookup">"ВОС за ОВТ"</a>
            </nav>
            <ActorSwitcher/>
        </header>
    }
}
