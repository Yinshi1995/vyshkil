use std::collections::HashSet;

use leptos::prelude::*;

use crate::components::DatePicker;
use crate::hooks::use_actor::use_actor;
use crate::pages::home::server::get_subordination_tree;
use crate::types::org::OrgTreeRow;
use crate::widgets::ActorNotice;

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
                    <DatePicker
                        value=Signal::derive(move || {
                            chrono::NaiveDate::parse_from_str(&as_of.get(), "%Y-%m-%d").ok()
                        })
                        on_change=Callback::new(move |d: Option<chrono::NaiveDate>| {
                            as_of.set(d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
                        })
                        placeholder="дд.мм.рррр".to_string()
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
                            Ok(rows) => {
                                let collapsed: RwSignal<HashSet<i32>> = RwSignal::new(HashSet::new());
                                view! { <CollapsibleTree rows=rows collapsed=collapsed/> }.into_any()
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

#[component]
fn CollapsibleTree(rows: Vec<OrgTreeRow>, collapsed: RwSignal<HashSet<i32>>) -> impl IntoView {
    render_level(&rows, None, 0, collapsed)
}

fn render_level(
    rows: &[OrgTreeRow],
    parent_id: Option<i32>,
    depth: usize,
    collapsed: RwSignal<HashSet<i32>>,
) -> impl IntoView {
    let children: Vec<_> = rows.iter().filter(|r| r.parent_id == parent_id).collect();
    if children.is_empty() {
        return ().into_any();
    }
    let nodes: Vec<_> = children
        .into_iter()
        .map(|node| {
            let node_id = node.id;
            let has_children = rows.iter().any(|r| r.parent_id == Some(node_id));
            let child_rows: Vec<OrgTreeRow> = rows.to_vec();
            let label = node.label.clone();
            let full_name = node.full_name.clone();
            let is_collapsed = Signal::derive(move || collapsed.get().contains(&node_id));
            let child_depth = depth + 1;

            let toggle = move |_: web_sys::MouseEvent| {
                collapsed.update(|set| {
                    if !set.remove(&node_id) {
                        set.insert(node_id);
                    }
                });
            };

            let depth_class = match depth {
                0 => " org-tree__node--root",
                1 => " org-tree__node--l1",
                _ => "",
            };

            view! {
                <li class=format!("org-tree__node{depth_class}")>
                    <span class="org-tree__row">
                        {if has_children {
                            view! {
                                <button
                                    class="org-tree__toggle"
                                    on:click=toggle
                                    aria-expanded=move || (!is_collapsed.get()).to_string()
                                    aria-label="Розгорнути/згорнути"
                                >
                                    <svg
                                        class="org-tree__chevron"
                                        class:org-tree__chevron--collapsed=is_collapsed
                                        width="12" height="12" viewBox="0 0 12 12"
                                    >
                                        <path d="M4 2 L8 6 L4 10" fill="none" stroke="currentColor"
                                              stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                                    </svg>
                                </button>
                            }.into_any()
                        } else {
                            view! { <span class="org-tree__leaf-indent"></span> }.into_any()
                        }}
                        <a href=format!("/org/{node_id}") class="org-tree__label">
                            {label}
                            {full_name.map(|fn_| view! {
                                <span class="org-tree__fullname">{fn_}</span>
                            })}
                        </a>
                    </span>
                    {move || {
                        if is_collapsed.get() {
                            ().into_any()
                        } else {
                            render_level(&child_rows, Some(node_id), child_depth, collapsed).into_any()
                        }
                    }}
                </li>
            }
        })
        .collect();

    view! {
        <ul class=if parent_id.is_none() { "org-tree org-tree--root" } else { "org-tree" }>
            {nodes}
        </ul>
    }
    .into_any()
}
