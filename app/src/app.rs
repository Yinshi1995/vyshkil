use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::actor::{Actor, Role};

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
                <Routes fallback=|| view! { <p>"Сторінку не знайдено."</p> }>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="app-header">
            <div class="app-header__brand">
                <span class="app-header__mark">"Т"</span>
                <span class="app-header__title">"Taktoblik"</span>
            </div>
            <ActorSwitcher/>
        </header>
    }
}

/// Перемикач актора: список організацій із `org` + вибір ролі. Тільки для розробки —
/// пізніше цю пару (org, роль) віддаватиме мікросервіс автентифікації (01 §6).
#[component]
fn ActorSwitcher() -> impl IntoView {
    let actor = expect_context::<RwSignal<Option<Actor>>>();
    let orgs = Resource::new(|| (), |_| list_orgs());

    view! {
        <div class="actor-switcher">
            <label>"Актор:"</label>
            <Suspense fallback=|| view! { <span>"..."</span> }>
                {move || {
                    orgs.get()
                        .map(|res| match res {
                            Ok(list) if list.is_empty() => {
                                view! { <span class="status-error">"немає організацій (сід ще не завантажено)"</span> }
                                    .into_any()
                            }
                            Ok(list) => {
                                let on_org_change = move |ev| {
                                    let org_id: i32 = event_target_value(&ev).parse().unwrap_or_default();
                                    let role = actor.get().map(|a| a.role).unwrap_or(Role::Admin);
                                    actor.set(Some(Actor { org_id, role }));
                                };
                                let on_role_change = move |ev| {
                                    let role = Role::parse(&event_target_value(&ev)).unwrap_or(Role::Admin);
                                    if let Some(a) = actor.get() {
                                        actor.set(Some(Actor { org_id: a.org_id, role }));
                                    }
                                };
                                let current_org = actor.get().map(|a| a.org_id);
                                view! {
                                    <select on:change=on_org_change>
                                        {list.iter()
                                            .map(|(id, name)| {
                                                let selected = current_org == Some(*id);
                                                view! {
                                                    <option value=id.to_string() selected=selected>
                                                        {name.clone()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                    <select on:change=on_role_change>
                                        {Role::ALL
                                            .iter()
                                            .map(|r| {
                                                view! {
                                                    <option value=r.as_str()>{r.label()}</option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <span class="status-error">{format!("помилка: {e}")}</span> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let db_status = Resource::new(|| (), |_| health_check());
    let orgs = Resource::new(|| (), |_| list_orgs());

    view! {
        <h1>"Taktoblik"</h1>
        <p>
            "Облік заходів підготовки військових частин: збір даних, нормалізація, звірка між "
            "рівнями підпорядкування, звітні документи."
        </p>
        <div class="card-row">
            <div class="card">
                <div class="icon-box">
                    <svg viewBox="0 0 24 24" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                        <ellipse cx="12" cy="5" rx="8" ry="3"></ellipse>
                        <path d="M4 5v6c0 1.66 3.58 3 8 3s8-1.34 8-3V5"></path>
                        <path d="M4 11v6c0 1.66 3.58 3 8 3s8-1.34 8-3v-6"></path>
                    </svg>
                </div>
                <div class="card__label">"З'єднання з БД"</div>
                <Suspense fallback=|| view! { <div class="card__value">"…"</div> }>
                    {move || {
                        db_status
                            .get()
                            .map(|res| match res {
                                Ok(status) => {
                                    view! { <div class="card__value status-ok">{status}</div> }.into_any()
                                }
                                Err(e) => {
                                    view! {
                                        <div class="card__value status-error">"помилка"</div>
                                        <div class="card__desc">{e.to_string()}</div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </div>
            <div class="card">
                <div class="icon-box">
                    <svg viewBox="0 0 24 24" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 21h18"></path>
                        <path d="M6 21V7l6-4 6 4v14"></path>
                        <path d="M10 21v-6h4v6"></path>
                        <path d="M10 11h.01M14 11h.01M10 15h.01M14 15h.01"></path>
                    </svg>
                </div>
                <div class="card__label">"Організацій у довіднику"</div>
                <Suspense fallback=|| view! { <div class="card__value">"…"</div> }>
                    {move || {
                        orgs.get()
                            .map(|res| match res {
                                Ok(list) => view! { <div class="card__value">{list.len()}</div> }.into_any(),
                                Err(e) => {
                                    view! {
                                        <div class="card__value status-error">"помилка"</div>
                                        <div class="card__desc">{e.to_string()}</div>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
                <div class="card__desc">"Dev-сід Етапу 1 (зі specи, не з source_files): органи + приклад переходу 17 АК → 7 КШР."</div>
            </div>
        </div>
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

/// Список організацій для перемикача актора: (id, короткий вигляд "назва (номер)").
/// Прямий SQL, без entity — Stage 1 ще не заводить повноцінні sea-orm entity для org.
#[server(ListOrgs, "/api")]
pub async fn list_orgs() -> Result<Vec<(i32, String)>, ServerFnError> {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    #[derive(FromQueryResult)]
    struct OrgRow {
        id: i32,
        short_name: String,
        number: Option<String>,
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT id, short_name, number FROM org WHERE deleted_at IS NULL ORDER BY short_name",
    );
    let rows = OrgRow::find_by_statement(stmt)
        .all(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let label = match r.number {
                Some(n) => format!("{} ({n})", r.short_name),
                None => r.short_name,
            };
            (r.id, label)
        })
        .collect())
}
