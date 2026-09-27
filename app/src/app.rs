use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

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

    view! {
        <Stylesheet id="leptos" href="/pkg/taktoblik.css"/>
        <Title text="Taktoblik — облік бойової підготовки"/>
        <Router>
            <main>
                <Routes fallback=|| view! { <p>"Сторінку не знайдено."</p> }>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let db_status = Resource::new(|| (), |_| health_check());

    view! {
        <h1>"Taktoblik"</h1>
        <p>"Система збору та обробки даних з бойової підготовки батальйон-бригада."</p>
        <p>
            "Стан підключення до БД: "
            <Suspense fallback=|| view! { "перевіряю..." }>
                {move || {
                    db_status
                        .get()
                        .map(|res| match res {
                            Ok(status) => status,
                            Err(e) => format!("помилка: {e}"),
                        })
                }}
            </Suspense>
        </p>
    }
}

// #[server] генерує однакову сигнатуру для обох таргетів: на клієнті це виклик по мережі,
// на сервері (під feature "ssr") — реальне тіло, що бере DatabaseConnection з контексту Leptos-роуту.
#[server(HealthCheck, "/api")]
pub async fn health_check() -> Result<String, ServerFnError> {
    let db = expect_context::<sea_orm::DatabaseConnection>();
    db.ping().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok("з'єднано".to_string())
}
