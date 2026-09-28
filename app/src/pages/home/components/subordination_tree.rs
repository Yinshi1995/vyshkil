use leptos::prelude::*;

use crate::hooks::use_actor::use_actor;
use crate::pages::home::server::get_subordination_tree;
use crate::types::org::OrgTreeRow;
use crate::widgets::ActorNotice;

/// Дерево підпорядкування з перемикачем осі й дати. За замовчуванням — сьогодні; щоб побачити
/// сценарій переходу (142/154/61/5/92/225 омбр/ошбр з 17 АК → 7 КШР, серпень 2026), можна
/// підставити 2026-07-20 і 2026-09-20. Дерево обрізане до видимого поточному актору піддерева
/// (backend::policy) — без обраного актора показуємо `<ActorNotice/>`, а не порожнє дерево.
#[component]
pub fn SubordinationTree() -> impl IntoView {
    let actor = use_actor();
    let axis = RwSignal::new("staff".to_string());
    let as_of = RwSignal::new("2026-09-28".to_string());
    let tree = Resource::new(
        move || (actor.get(), axis.get(), as_of.get()),
        |(actor, axis, as_of)| async move { get_subordination_tree(actor, as_of, axis).await },
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
                    if actor.get().is_none() {
                        return Some(view! { <ActorNotice/> }.into_any());
                    }
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
