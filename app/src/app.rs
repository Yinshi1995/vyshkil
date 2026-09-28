use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    hooks::use_params_map,
    ParamSegment, StaticSegment,
};

use crate::actor::{Actor, Role};
use crate::normalize::normalize;

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
                    <Route path=(StaticSegment("org"), ParamSegment("id")) view=OrgDetailPage/>
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
        <OrgSearch/>
        <SubordinationTree/>
    }
}

/// Нечіткий пошук організацій (02 §3): стійкий до опечаток/розкладки/скорочень
/// ("152НЦ", "а4896", "польша" — усі знаходять канонічну організацію).
#[component]
fn OrgSearch() -> impl IntoView {
    let query = RwSignal::new(String::new());
    let results = Resource::new(
        move || query.get(),
        |q| async move {
            if q.trim().is_empty() {
                Ok(Vec::new())
            } else {
                search_orgs(q).await
            }
        },
    );

    view! {
        <div class="eyebrow">"Пошук організацій"</div>
        <div class="card">
            <input
                type="text"
                class="org-search__input"
                placeholder="152НЦ, а4896, польша…"
                prop:value=move || query.get()
                on:input=move |ev| query.set(event_target_value(&ev))
            />
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    results
                        .get()
                        .map(|res| match res {
                            Ok(_) if query.get().trim().is_empty() => {
                                view! { <p class="card__desc">"Почніть вводити номер, назву або синонім."</p> }
                                    .into_any()
                            }
                            Ok(list) if list.is_empty() => {
                                view! { <p class="card__desc">"Нічого не знайдено."</p> }.into_any()
                            }
                            Ok(list) => {
                                view! {
                                    <ul class="org-search__results">
                                        {list
                                            .into_iter()
                                            .map(|r| {
                                                view! {
                                                    <li class="org-search__result">
                                                        <span class="org-search__label">{r.label}</span>
                                                        <span class="org-search__matched">
                                                            "збіг: \""{r.matched_raw}"\""
                                                        </span>
                                                        <a href=format!("/org/{}", r.org_id) class="org-search__link">
                                                            "картка →"
                                                        </a>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <p class="card__desc status-error">{e.to_string()}</p> }.into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

/// Один вузол дерева підпорядкування: пряма (`depth = 1`) ланка з `subordination_closure` на дату.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrgTreeRow {
    pub id: i32,
    pub label: String,
    pub parent_id: Option<i32>,
}

/// Дерево підпорядкування на дату (06-roadmap.md, Етап 1): перемикач осі штатне/оперативне.
/// Гарячий запит — через `subordination_closure` (`depth = 1`, матеріалізоване замикання),
/// **не** рекурсивний CTE (server/CLAUDE.md).
#[server(GetSubordinationTree, "/api")]
pub async fn get_subordination_tree(
    as_of: String,
    axis: String,
) -> Result<Vec<OrgTreeRow>, ServerFnError> {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        short_name: String,
        number: Option<String>,
        parent_id: Option<i32>,
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        SELECT o.id, o.short_name, o.number, sc.ancestor_id AS parent_id
        FROM org o
        LEFT JOIN subordination_closure sc
            ON sc.descendant_id = o.id
           AND sc.axis = $1
           AND sc.depth = 1
           AND daterange(sc.valid_from, sc.valid_to, '[)') @> $2::date
        WHERE o.deleted_at IS NULL
        ORDER BY o.short_name
        "#,
        [axis.into(), as_of.into()],
    );

    let rows = Row::find_by_statement(stmt)
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
            OrgTreeRow { id: r.id, label, parent_id: r.parent_id }
        })
        .collect())
}

/// Дерево підпорядкування з перемикачем осі й дати. За замовчуванням — сьогодні; щоб побачити
/// сценарій переходу (142/154/61/5/92/225 омбр/ошбр з 17 АК → 7 КШР, серпень 2026), можна
/// підставити 2026-07-20 і 2026-09-20.
#[component]
fn SubordinationTree() -> impl IntoView {
    let axis = RwSignal::new("staff".to_string());
    let as_of = RwSignal::new("2026-09-28".to_string());
    let tree = Resource::new(
        move || (axis.get(), as_of.get()),
        |(axis, as_of)| async move { get_subordination_tree(as_of, axis).await },
    );

    view! {
        <div class="eyebrow">"Дерево підпорядкування"</div>
        <div class="card">
            <div class="tree-controls">
                <label>
                    <input
                        type="radio"
                        name="axis"
                        value="staff"
                        checked=move || axis.get() == "staff"
                        on:change=move |_| axis.set("staff".to_string())
                    />
                    " Штатне"
                </label>
                <label>
                    <input
                        type="radio"
                        name="axis"
                        value="operational"
                        checked=move || axis.get() == "operational"
                        on:change=move |_| axis.set("operational".to_string())
                    />
                    " Оперативне"
                </label>
                <label class="tree-controls__date">
                    " на дату "
                    <input
                        type="date"
                        prop:value=move || as_of.get()
                        on:input=move |ev| as_of.set(event_target_value(&ev))
                    />
                </label>
            </div>
            <Suspense fallback=|| view! { <p>"…"</p> }>
                {move || {
                    tree.get()
                        .map(|res| match res {
                            Ok(rows) => render_tree(&rows, None).into_any(),
                            Err(e) => {
                                view! { <p class="card__desc status-error">{e.to_string()}</p> }.into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

/// Рекурсивно рендерить дітей вузла `parent_id` (None = корені — органи без батька на цю дату+вісь).
fn render_tree(rows: &[OrgTreeRow], parent_id: Option<i32>) -> impl IntoView {
    let children: Vec<_> = rows.iter().filter(|r| r.parent_id == parent_id).collect();
    if children.is_empty() {
        return ().into_any();
    }
    view! {
        <ul class="org-tree">
            {children
                .into_iter()
                .map(|node| {
                    let sub = render_tree(rows, Some(node.id));
                    view! {
                        <li class="org-tree__node">
                            <a href=format!("/org/{}", node.id)>{node.label.clone()}</a>
                            {sub}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
        .into_any()
}

/// Картка частини з історією назв і статусів (06-roadmap.md, Етап 1).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrgDetail {
    pub id: i32,
    pub short_name: String,
    pub full_name: Option<String>,
    pub number: Option<String>,
    pub kind: String,
    pub is_active: bool,
    pub name_history: Vec<(String, String, Option<String>)>,
    pub status_history: Vec<(String, String, Option<String>, Option<String>)>,
}

#[server(GetOrgDetail, "/api")]
pub async fn get_org_detail(org_id: i32) -> Result<OrgDetail, ServerFnError> {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    #[derive(FromQueryResult)]
    struct OrgRow {
        id: i32,
        short_name: String,
        full_name: Option<String>,
        number: Option<String>,
        kind: String,
        is_active: bool,
    }
    #[derive(FromQueryResult)]
    struct NameHistoryRow {
        short_name: String,
        valid_from: String,
        valid_to: Option<String>,
    }
    #[derive(FromQueryResult)]
    struct StatusHistoryRow {
        status: String,
        valid_from: String,
        valid_to: Option<String>,
        note: Option<String>,
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let backend = db.get_database_backend();

    let org = OrgRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT id, short_name, full_name, number, kind, is_active \
         FROM org WHERE id = $1 AND deleted_at IS NULL",
        [org_id.into()],
    ))
    .one(&db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("організацію не знайдено"))?;

    let name_history = NameHistoryRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT short_name, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(valid_to, 'YYYY-MM-DD') AS valid_to \
         FROM org_name_history WHERE org_id = $1 ORDER BY valid_from",
        [org_id.into()],
    ))
    .all(&db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let status_history = StatusHistoryRow::find_by_statement(Statement::from_sql_and_values(
        backend,
        "SELECT status, to_char(valid_from, 'YYYY-MM-DD') AS valid_from, \
                to_char(valid_to, 'YYYY-MM-DD') AS valid_to, note \
         FROM org_status WHERE org_id = $1 ORDER BY valid_from",
        [org_id.into()],
    ))
    .all(&db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(OrgDetail {
        id: org.id,
        short_name: org.short_name,
        full_name: org.full_name,
        number: org.number,
        kind: org.kind,
        is_active: org.is_active,
        name_history: name_history
            .into_iter()
            .map(|r| (r.short_name, r.valid_from, r.valid_to))
            .collect(),
        status_history: status_history
            .into_iter()
            .map(|r| (r.status, r.valid_from, r.valid_to, r.note))
            .collect(),
    })
}

/// Сторінка `/org/:id`: поточні дані + історія назв/статусів (порожня історія — легітимний стан,
/// поки в частини не було жодного перейменування чи зміни статусу).
#[component]
fn OrgDetailPage() -> impl IntoView {
    let params = use_params_map();
    let org_id = move || {
        params
            .read()
            .get("id")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or_default()
    };
    let detail = Resource::new(org_id, |id| async move { get_org_detail(id).await });

    view! {
        <Suspense fallback=|| view! { <p>"…"</p> }>
            {move || {
                detail
                    .get()
                    .map(|res| match res {
                        Ok(d) => {
                            let label = match &d.number {
                                Some(n) => format!("{} ({n})", d.short_name),
                                None => d.short_name.clone(),
                            };
                            view! {
                                <h1>{label}</h1>
                                <p>
                                    {d.full_name.clone().unwrap_or_default()} " · " {d.kind.clone()}
                                    " · " {if d.is_active { "діюча" } else { "неактивна" }}
                                </p>
                                <div class="eyebrow">"Історія назв"</div>
                                <div class="card">
                                    {if d.name_history.is_empty() {
                                        view! { <p class="card__desc">"Перейменувань не було."</p> }.into_any()
                                    } else {
                                        view! {
                                            <table>
                                                <thead>
                                                    <tr>
                                                        <th>"Назва"</th>
                                                        <th>"Діє з"</th>
                                                        <th>"Діє до"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {d.name_history
                                                        .into_iter()
                                                        .map(|(name, from, to)| {
                                                            view! {
                                                                <tr>
                                                                    <td>{name}</td>
                                                                    <td>{from}</td>
                                                                    <td>{to.unwrap_or_else(|| "—".to_string())}</td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        }
                                            .into_any()
                                    }}
                                </div>
                                <div class="eyebrow">"Історія статусів"</div>
                                <div class="card">
                                    {if d.status_history.is_empty() {
                                        view! { <p class="card__desc">"Змін статусу не було."</p> }.into_any()
                                    } else {
                                        view! {
                                            <table>
                                                <thead>
                                                    <tr>
                                                        <th>"Статус"</th>
                                                        <th>"Діє з"</th>
                                                        <th>"Діє до"</th>
                                                        <th>"Примітка"</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {d.status_history
                                                        .into_iter()
                                                        .map(|(status, from, to, note)| {
                                                            view! {
                                                                <tr>
                                                                    <td>{status}</td>
                                                                    <td>{from}</td>
                                                                    <td>{to.unwrap_or_else(|| "—".to_string())}</td>
                                                                    <td>{note.unwrap_or_default()}</td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        }
                                            .into_any()
                                    }}
                                </div>
                            }
                                .into_any()
                        }
                        Err(e) => {
                            view! { <p class="status-error">{e.to_string()}</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// Один результат нечіткого пошуку організацій: канонічна форма + який саме синонім збігся.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrgSearchResult {
    pub org_id: i32,
    pub label: String,
    pub matched_raw: String,
    pub is_exact: bool,
}

/// Нечіткий пошук організацій по `alias.norm` (02 §3, критерій готовності Етапу 1:
/// "152НЦ"/"а4896"/"польша" знаходять канонічні організації).
/// Запит нормалізується тією ж функцією, що й alias.norm при сіді/введенні (01 §"alias") —
/// інакше "152НЦ" (з великими літерами) не збігся б з засіяним норм-рядком "152нц".
/// Ранжування — за 02 §3: точний збіг синоніма → частота використання цією організацією → схожість (pg_trgm).
#[server(SearchOrgs, "/api")]
pub async fn search_orgs(query: String) -> Result<Vec<OrgSearchResult>, ServerFnError> {
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let norm_query = normalize(&query);
    if norm_query.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(FromQueryResult)]
    struct Row {
        org_id: i32,
        short_name: String,
        number: Option<String>,
        matched_raw: String,
        is_exact: bool,
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"
        WITH matches AS (
            SELECT
                o.id AS org_id,
                o.short_name,
                o.number,
                a.raw AS matched_raw,
                a.uses_count,
                (a.norm = $1) AS is_exact,
                similarity(a.norm, $1) AS sim
            FROM alias a
            JOIN org o ON o.id = a.target_id AND a.target_type = 'org'
            WHERE o.deleted_at IS NULL
              AND (a.norm = $1 OR a.norm % $1)
        ),
        ranked AS (
            SELECT
                *,
                ROW_NUMBER() OVER (
                    PARTITION BY org_id
                    ORDER BY is_exact DESC, uses_count DESC, sim DESC
                ) AS rn
            FROM matches
        )
        SELECT org_id, short_name, number, matched_raw, is_exact
        FROM ranked
        WHERE rn = 1
        ORDER BY is_exact DESC, uses_count DESC, sim DESC
        LIMIT 10
        "#,
        [norm_query.into()],
    );

    let rows = Row::find_by_statement(stmt)
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
            OrgSearchResult {
                org_id: r.org_id,
                label,
                matched_raw: r.matched_raw,
                is_exact: r.is_exact,
            }
        })
        .collect())
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
