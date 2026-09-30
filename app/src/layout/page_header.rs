//! Заголовок сторінки (Етап 7.5, `docs/spec/components/layout.md` §2) — компактний, з опційними
//! хлібними крихтами й підзаголовком-контекстом, замість нинішнього величезного `<h1>` без
//! структури. Дії сторінки (0-2 кнопки) — `children`, праворуч.

use leptos::prelude::*;

pub struct Breadcrumb {
    pub label: String,
    pub href: Option<String>,
}

impl Breadcrumb {
    pub fn link(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self { label: label.into(), href: Some(href.into()) }
    }

    pub fn text(label: impl Into<String>) -> Self {
        Self { label: label.into(), href: None }
    }
}

#[component]
pub fn PageHeader(
    #[prop(into)] title: String,
    #[prop(optional, into)] subtitle: String,
    #[prop(optional)] breadcrumbs: Vec<Breadcrumb>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let has_breadcrumbs = !breadcrumbs.is_empty();
    let has_subtitle = !subtitle.is_empty();

    view! {
        <div class="page-header">
            {has_breadcrumbs
                .then(|| {
                    view! {
                        <nav class="page-header__breadcrumbs" aria-label="Хлібні крихти">
                            {breadcrumbs
                                .into_iter()
                                .map(|b| match b.href {
                                    Some(href) => view! { <a href=href>{b.label}</a> }.into_any(),
                                    None => view! { <span>{b.label}</span> }.into_any(),
                                })
                                .collect_view()}
                        </nav>
                    }
                })}
            <div class="page-header__row">
                <div class="page-header__titles">
                    <h1 class="page-header__title">{title}</h1>
                    {has_subtitle
                        .then(|| view! { <p class="page-header__subtitle">{subtitle}</p> })}
                </div>
                {children.map(|c| view! { <div class="page-header__actions">{c()}</div> })}
            </div>
        </div>
    }
}
