use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::Router;

use crate::layout::Header;
use crate::routes::AppRoutes;
use crate::types::actor::Actor;

// shell() генерує повний HTML-документ навколо <App/> — його викликає і SSR (перший рендер),
// і fallback-обробник помилок на сервері, тому він винесений окремо від самого <App/>.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="uk">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Актор (організація+роль) — dev-перемикач у шапці замість автентифікації (01 §6).
    // None, поки нема жодної організації в довіднику (сід ще не завантажено — Етап 1, далі).
    let actor: RwSignal<Option<Actor>> = RwSignal::new(None);
    provide_context(actor);

    view! {
        <Stylesheet id="leptos" href="/pkg/taktoblik.css"/>
        <Title text="Taktoblik — облік підготовки"/>
        <Router>
            <Header/>
            <main>
                <AppRoutes/>
            </main>
        </Router>
    }
}
