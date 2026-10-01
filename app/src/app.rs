use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::Router;
use leptos_router::hooks::use_location;

use crate::layout::Header;
use crate::routes::AppRoutes;
use crate::services::auth::get_current_user;
use crate::types::actor::{Actor, Role};
use crate::types::auth::AuthMode;

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

    let actor: RwSignal<Option<Actor>> = RwSignal::new(None);
    provide_context(actor);

    let auth_mode: RwSignal<AuthMode> = RwSignal::new(AuthMode::Dev);
    provide_context(auth_mode);

    let auth_display_name: RwSignal<Option<String>> = RwSignal::new(None);
    provide_context(auth_display_name);

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(Some(session)) = get_current_user().await {
                auth_display_name.set(session.display_name.clone());
                if let (Some(org_id), Some(role_str)) = (session.active_org_id, session.active_role.as_deref()) {
                    let role = Role::parse(role_str).unwrap_or(Role::Admin);
                    actor.set(Some(Actor { org_id, role }));
                }
                auth_mode.set(AuthMode::Auth);
            }
        });
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/taktoblik.css"/>
        <Title text="Taktoblik — облік підготовки"/>
        <Router>
            <AppShell/>
        </Router>
    }
}

#[component]
fn AppShell() -> impl IntoView {
    let location = use_location();
    let is_login = Memo::new(move |_| location.pathname.get() == "/login");

    view! {
        <Show when=move || !is_login.get()>
            <Header/>
        </Show>
        <main>
            <AppRoutes/>
        </main>
    }
}
