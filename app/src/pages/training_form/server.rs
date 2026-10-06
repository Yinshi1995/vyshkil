use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::submission::{CommitOutcome, DraftPayload, DraftState, GroupFormRow};

const SOURCE_TYPE: &str = "form";

/// Розбір файлу "Фах" (03 §1-3): байти → структурні рядки (без БД) → резолюція org/vos/посада/
/// місце через довідники → та сама сітка, що й ручне введення (Етап 5, перенесено з
/// `pages::import` при об'єднанні сторінок — [[unified-training-form-source-type]]). Помилки
/// розбору (не той тип файлу, пошкоджений xlsx) зупиняють увесь імпорт; нерозпізнані клітинки в
/// межах одного рядка — ні, вони просто лишаються `*_id = None` для ручного підтвердження в сітці.
#[server(ParseFahFile, "/api")]
pub async fn parse_fah_file(
    actor: Option<Actor>,
    bytes: Vec<u8>,
) -> Result<Vec<GroupFormRow>, ServerFnError> {
    use crate::backend::{import, policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не імпортує дані"));
    }

    let raw_rows =
        import::fah::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if raw_rows.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл \"Фах\" \
             (аркуші \"Пройшли\"/\"Проходять\", заголовок Підрозділ/Місце/Посада/ВОС/ОВТ/Термін/Кількість)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::imports_fah::resolve_rows(&db, raw_rows)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Розбір файлу "БпС" — та сама ідея, інша структура джерела (`backend/import/CLAUDE.md`):
/// дворівневий заголовок, funnel-воронка ("Завершилась": Викликали→Прибуло до НЦ→Успішно
/// завершило; "Навчаються": Викликали→Проходять) замість однієї "Кількість".
#[server(ParseBpsFile, "/api")]
pub async fn parse_bps_file(
    actor: Option<Actor>,
    bytes: Vec<u8>,
) -> Result<Vec<GroupFormRow>, ServerFnError> {
    use crate::backend::{import, policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не імпортує дані"));
    }

    let raw_rows =
        import::bps::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if raw_rows.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл \"БпС\" \
             (аркуші \"Завершилась\"/\"Навчаються\", заголовок Тип БпАК/ВОС/Військова частина/…)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::imports_bps::resolve_rows(&db, raw_rows)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Розбір файлу "Терміни" — три паралельні списки (БЗВП/Фахова/Адаптація,
/// `backend/import/terminy.rs`) РАЗОМ у ту саму сітку, що й усі попередні.
#[server(ParseTerminyFile, "/api")]
pub async fn parse_terminy_file(
    actor: Option<Actor>,
    bytes: Vec<u8>,
) -> Result<Vec<GroupFormRow>, ServerFnError> {
    use crate::backend::{import, policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не імпортує дані"));
    }

    let extract = import::terminy::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if extract.bzvp.is_empty() && extract.special.is_empty() && extract.adapt.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл \"Терміни\" \
             (заголовок № з/п/Підрозділ/БЗВП/Фахова підготовка/Адаптація)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::imports_terminy::resolve_rows(&db, extract)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Чернетка організації актора, якщо є (02 §6: відновлення після обриву зв'язку/закритої вкладки).
#[server(GetDraft, "/api")]
pub async fn get_draft(actor: Option<Actor>) -> Result<Option<DraftState>, ServerFnError> {
    use crate::services::submission_grid::get_draft_impl;

    get_draft_impl(actor, SOURCE_TYPE).await.map_err(ServerFnError::new)
}

/// Автозбереження сітки (02 §6) — кожні кілька секунд з клієнта. `viewer` не редагує (01 §6),
/// тож і чернетку не зберігає.
#[server(SaveDraft, "/api")]
pub async fn save_draft(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<i32, ServerFnError> {
    use crate::services::submission_grid::save_draft_impl;

    save_draft_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}

/// Фіксація сітки (`Ctrl+Enter`, 02 §5) — усе-або-нічого: валідує кожен рядок (дати, порядок
/// воронки, обов'язкові поля) і право редагування (`org_editor` — лише власна організація, 01 §6)
/// ДО запису; перша помилка зупиняє коміт і повертається клітинкою, яку сітка підсвічує.
#[server(CommitGrid, "/api")]
pub async fn commit_grid(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<CommitOutcome, ServerFnError> {
    use crate::services::submission_grid::commit_grid_impl;

    commit_grid_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}
