use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::staffing::{InstructorStaffingRow, StaffingRow};
use crate::types::submission::{CommitOutcome, DraftPayload, DraftState, GroupFormRow};

const SOURCE_TYPE: &str = "table";
/// "КВід" = командири відділень — `staffing_snapshot.category` (01 §4).
const KVID_CATEGORY: &str = "squad_leaders";
/// Архів ВЧ (Етап 6) — одноразовий перенос, інший `submission.source_type` за решту цієї
/// сторінки; тому окремий сталий рядок і окремий тріо `get_archive_draft`/`commit_archive_grid`
/// (не runtime-параметр у спільних `get_draft`/`commit_grid` — джерело подання типізоване
/// константою на боці клієнта в `FileKind::source_type()`, не приходить з форми).
const ARCHIVE_SOURCE_TYPE: &str = "archive_seed";

/// Розбір файлу "КВід" — інша структура ЗНОВУ (одноrівневий заголовок, але це не group-подібні
/// дані узагалі: `staffing_snapshot`/`staffing_metric`, 01 §4). Превʼю для цього типу — не
/// `widgets::group_grid` (форма даних інша), проста таблиця в `pages/import/mod.rs`.
#[server(ParseKvidFile, "/api")]
pub async fn parse_kvid_file(
    actor: Option<Actor>,
    bytes: Vec<u8>,
) -> Result<Vec<StaffingRow>, ServerFnError> {
    use crate::backend::{import, policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не імпортує дані"));
    }

    let raw_rows =
        import::kvid::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if raw_rows.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл «КВід» \
             (заголовок № з/п/Підрозділ/За штатом/За списком/В наявності/…)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::imports_kvid::resolve_rows(&db, raw_rows)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Розбір файлу "ІВС" — єдиний тип, що дає ОДРАЗУ два різних результати з одного файлу
/// (`backend/import/ivs.rs` пояснює чому): укомплектованість (`InstructorStaffingRow`, проста
/// таблиця, як КВід) і стажування+курси РАЗОМ (`GroupFormRow`, та сама сітка) — саме тому
/// лишився на `/import`, а не переїхав з Фах/БпС/Терміни на `/training-form`
/// ([[unified-training-form-source-type]]).
#[server(ParseIvsFile, "/api")]
pub async fn parse_ivs_file(
    actor: Option<Actor>,
    bytes: Vec<u8>,
) -> Result<(Vec<InstructorStaffingRow>, Vec<GroupFormRow>), ServerFnError> {
    use crate::backend::{import, policy, repo};
    use crate::services::auth::resolve_actor;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не імпортує дані"));
    }

    let extract = import::ivs::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if extract.staffing.is_empty() && extract.internships.is_empty() && extract.courses.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл «ІВС» \
             («ВІДОМІСТЬ укомплектованості та навченості груп інструкторів» + «ВІДОМІСТЬ \
             проходження підготовки інструкторами» на одному аркуші)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    let resolved = repo::imports_ivs::resolve_rows(&db, extract)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok((resolved.staffing, resolved.groups))
}

/// Розбір архіву ВЧ (Етап 6, `source_type='archive_seed'`) — **лише реальний файл**
/// (`20 АК/ОблікФаховоїПідготовкиАК.xlsx`, рішення користувача: решта `Дельта/` синтетична,
/// `docs/source-analysis.md` §1). Той самий шлях, що й Фах (однорівневий заголовок), дати вже
/// справжні `datetime`.
#[server(ParseVchArchiveFile, "/api")]
pub async fn parse_vch_archive_file(
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
        import::vch_archive::extract(&bytes).map_err(|e| ServerFnError::new(e.to_string()))?;
    if raw_rows.is_empty() {
        return Err(ServerFnError::new(
            "у файлі не знайдено жодного рядка даних — перевірте, що це файл архіву ВЧ \
             (аркуш «Записи», заголовок Військова частина/Місце проведення/Спеціальність/ВОС/ОВТ/\
             Термін з/Термін по/План/Фактично навчається)",
        ));
    }

    let db = expect_context::<sea_orm::DatabaseConnection>();
    repo::imports_vch_archive::resolve_rows(&db, raw_rows)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Чернетка/фіксація архіву ВЧ — той самий `services::submission_grid`, що й решта сторінки,
/// але `ARCHIVE_SOURCE_TYPE` замість `SOURCE_TYPE` (окреме тріо, не runtime-параметр — див.
/// коментар при `ARCHIVE_SOURCE_TYPE`).
#[server(GetArchiveDraft, "/api")]
pub async fn get_archive_draft(actor: Option<Actor>) -> Result<Option<DraftState>, ServerFnError> {
    use crate::services::submission_grid::get_draft_impl;

    get_draft_impl(actor, ARCHIVE_SOURCE_TYPE).await.map_err(ServerFnError::new)
}

#[server(SaveArchiveDraft, "/api")]
pub async fn save_archive_draft(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<i32, ServerFnError> {
    use crate::services::submission_grid::save_draft_impl;

    save_draft_impl(actor, submission_id, ARCHIVE_SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}

#[server(CommitArchiveGrid, "/api")]
pub async fn commit_archive_grid(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<CommitOutcome, ServerFnError> {
    use crate::services::submission_grid::commit_grid_impl;

    commit_grid_impl(actor, submission_id, ARCHIVE_SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}

/// Фіксація укомплектованості — усе-або-нічого, як і `commit_grid`: перевіряє право редагування
/// й резолюцію організації ДО запису, пише `submission` (`status='committed'` одразу — тут нема
/// проміжного стану "чернетка", превʼю не автозберігається) + `staffing_snapshot`/`_metric` на
/// кожен рядок.
#[server(CommitStaffing, "/api")]
pub async fn commit_staffing(
    actor: Option<Actor>,
    as_of_date: String,
    rows: Vec<StaffingRow>,
) -> Result<usize, ServerFnError> {
    use crate::backend::{db, policy, repo};
    use crate::services::auth::resolve_actor;
    use sea_orm::FromQueryResult;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не зберігає дані"));
    }
    if rows.is_empty() {
        return Err(ServerFnError::new("немає жодного рядка для збереження"));
    }
    if chrono::NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d").is_err() {
        return Err(ServerFnError::new("«станом на»: неможлива дата"));
    }
    for row in &rows {
        let Some(org_id) = row.org_id else {
            return Err(ServerFnError::new(format!(
                "«{}»: частину не розпізнано — оберіть вручну перед фіксацією",
                row.org_label
            )));
        };
        if !policy::can_edit_org(actor, org_id) {
            return Err(ServerFnError::new(format!(
                "немає права вносити дані за «{}»",
                row.org_label
            )));
        }
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let submission = NewId::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::ConnectionTrait::get_database_backend(&txn),
        "INSERT INTO submission (source_type, reporting_org_id, as_of_date, status) \
         VALUES ('table', $1, $2::date, 'committed') RETURNING id",
        [actor.org_id.into(), as_of_date.clone().into()],
    ))
    .one(&txn)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("INSERT submission не повернув id"))?;

    let mut count = 0;
    for row in &rows {
        let org_id = row.org_id.expect("перевірено вище");
        repo::staffing::insert_snapshot(&txn, org_id, &as_of_date, KVID_CATEGORY, submission.id, row)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        count += 1;
    }

    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(count)
}

/// Те саме для ІВС (`category='instructors'`) — окрема функція, бо `InstructorStaffingRow` —
/// інший тип за `StaffingRow` (`types/staffing.rs` пояснює чому: різні набори метрик, 01 §4).
#[server(CommitInstructorStaffing, "/api")]
pub async fn commit_instructor_staffing(
    actor: Option<Actor>,
    as_of_date: String,
    rows: Vec<InstructorStaffingRow>,
) -> Result<usize, ServerFnError> {
    use crate::backend::{db, policy, repo};
    use crate::services::auth::resolve_actor;
    use sea_orm::FromQueryResult;

    let actor = resolve_actor(actor).await?;
    if actor.role == policy::Role::Viewer {
        return Err(ServerFnError::new("перегляд не зберігає дані"));
    }
    if rows.is_empty() {
        return Err(ServerFnError::new("немає жодного рядка для збереження"));
    }
    if chrono::NaiveDate::parse_from_str(&as_of_date, "%Y-%m-%d").is_err() {
        return Err(ServerFnError::new("«станом на»: неможлива дата"));
    }
    for row in &rows {
        let Some(org_id) = row.org_id else {
            return Err(ServerFnError::new(format!(
                "«{}»: частину не розпізнано — оберіть вручну перед фіксацією",
                row.org_label
            )));
        };
        if !policy::can_edit_org(actor, org_id) {
            return Err(ServerFnError::new(format!(
                "немає права вносити дані за «{}»",
                row.org_label
            )));
        }
    }

    let txn = db::actor_transaction(actor).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    #[derive(FromQueryResult)]
    struct NewId {
        id: i32,
    }
    let submission = NewId::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::ConnectionTrait::get_database_backend(&txn),
        "INSERT INTO submission (source_type, reporting_org_id, as_of_date, status) \
         VALUES ('table', $1, $2::date, 'committed') RETURNING id",
        [actor.org_id.into(), as_of_date.clone().into()],
    ))
    .one(&txn)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("INSERT submission не повернув id"))?;

    let mut count = 0;
    for row in &rows {
        let org_id = row.org_id.expect("перевірено вище");
        repo::staffing::insert_instructor_snapshot(&txn, org_id, &as_of_date, submission.id, row)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        count += 1;
    }

    txn.commit().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(count)
}

/// Чернетка превʼю імпорту, якщо є (та сама логіка автозбереження, що й форма, 02 §6).
#[server(GetImportDraft, "/api")]
pub async fn get_draft(actor: Option<Actor>) -> Result<Option<DraftState>, ServerFnError> {
    use crate::services::submission_grid::get_draft_impl;

    get_draft_impl(actor, SOURCE_TYPE).await.map_err(ServerFnError::new)
}

#[server(SaveImportDraft, "/api")]
pub async fn save_draft(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<i32, ServerFnError> {
    use crate::services::submission_grid::save_draft_impl;

    save_draft_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}

/// Фіксація превʼю (`Ctrl+Enter`) — та сама валідація й запис у `training_group`/`group_event`,
/// що й форма ручного введення (02 §5); джерело подання відрізняється лише `source_type`.
#[server(CommitImport, "/api")]
pub async fn commit_grid(
    actor: Option<Actor>,
    submission_id: Option<i32>,
    payload: DraftPayload,
) -> Result<CommitOutcome, ServerFnError> {
    use crate::services::submission_grid::commit_grid_impl;

    commit_grid_impl(actor, submission_id, SOURCE_TYPE, payload).await.map_err(ServerFnError::new)
}
