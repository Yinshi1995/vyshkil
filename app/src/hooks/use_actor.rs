use leptos::prelude::*;

use crate::types::actor::Actor;

/// Поточний актор із контексту (`provide_context` у `app::app::App`). Централізує доступ, щоб
/// компоненти не смикали `expect_context::<RwSignal<Option<Actor>>>()` напряму — якщо механізм
/// провайдингу актора зміниться (напр. на реальну автентифікацію), правиться тільки тут.
pub fn use_actor() -> RwSignal<Option<Actor>> {
    expect_context::<RwSignal<Option<Actor>>>()
}
