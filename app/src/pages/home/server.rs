use leptos::prelude::*;

use crate::types::actor::Actor;
use crate::types::dashboard::{DashboardStats, RecentSubmission};
use crate::types::org::OrgTreeRow;

#[server(GetSubordinationTree, "/api")]
pub async fn get_subordination_tree(
    actor: Option<Actor>,
    as_of: String,
    axis: String,
) -> Result<Vec<OrgTreeRow>, ServerFnError> {
    use crate::backend::{policy, repo};
    use crate::services::auth::resolve_actor;

    let Ok(actor) = resolve_actor(actor).await else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let visible = policy::visible_org_ids(&db, actor)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let rows = repo::orgs::subordination_tree(&db, &as_of, &axis)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(match visible {
        None => rows,
        Some(ids) => policy::restrict_tree(rows, &ids),
    })
}

#[server(GetDashboardStats, "/api")]
pub async fn get_dashboard_stats(
    actor: Option<Actor>,
) -> Result<DashboardStats, ServerFnError> {
    use crate::services::auth::resolve_actor;
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let Ok(_actor) = resolve_actor(actor).await else {
        return Ok(DashboardStats {
            org_count: 0,
            training_group_count: 0,
            committed_submission_count: 0,
            draft_submission_count: 0,
            open_discrepancy_count: 0,
            notification_count: 0,
        });
    };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    #[derive(FromQueryResult)]
    struct Counts {
        org_count: i64,
        training_group_count: i64,
        committed_submission_count: i64,
        draft_submission_count: i64,
        open_discrepancy_count: i64,
        notification_count: i64,
    }

    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT \
            (SELECT count(*) FROM org WHERE is_active) AS org_count, \
            (SELECT count(*) FROM training_group) AS training_group_count, \
            (SELECT count(*) FROM submission WHERE status = 'committed') AS committed_submission_count, \
            (SELECT count(*) FROM submission WHERE status = 'draft') AS draft_submission_count, \
            (SELECT count(*) FROM discrepancy WHERE status = 'open') AS open_discrepancy_count, \
            (SELECT count(*) FROM notification WHERE NOT is_read) AS notification_count"
            .to_string(),
    );

    let row = Counts::find_by_statement(stmt)
        .one(&db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .unwrap_or(Counts {
            org_count: 0,
            training_group_count: 0,
            committed_submission_count: 0,
            draft_submission_count: 0,
            open_discrepancy_count: 0,
            notification_count: 0,
        });

    Ok(DashboardStats {
        org_count: row.org_count,
        training_group_count: row.training_group_count,
        committed_submission_count: row.committed_submission_count,
        draft_submission_count: row.draft_submission_count,
        open_discrepancy_count: row.open_discrepancy_count,
        notification_count: row.notification_count,
    })
}

#[server(GetRecentSubmissions, "/api")]
pub async fn get_recent_submissions(
    actor: Option<Actor>,
) -> Result<Vec<RecentSubmission>, ServerFnError> {
    use crate::backend::policy;
    use crate::services::auth::resolve_actor;
    use sea_orm::{ConnectionTrait, FromQueryResult, Statement};

    let Ok(actor) = resolve_actor(actor).await else { return Ok(Vec::new()) };
    let db = expect_context::<sea_orm::DatabaseConnection>();

    let visible = policy::visible_org_ids(&db, actor)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    #[derive(FromQueryResult)]
    struct Row {
        id: i32,
        reporting_org_id: i32,
        org_label: String,
        source_type: String,
        status: String,
        updated_at: String,
    }

    let rows = Row::find_by_statement(Statement::from_string(
        db.get_database_backend(),
        "SELECT s.id, s.reporting_org_id, \
                coalesce(o.short_name, '#' || s.reporting_org_id) AS org_label, \
                s.source_type, s.status, \
                to_char(s.updated_at, 'DD.MM.YYYY HH24:MI') AS updated_at \
         FROM submission s \
         LEFT JOIN org o ON o.id = s.reporting_org_id \
         ORDER BY s.updated_at DESC \
         LIMIT 20"
            .to_string(),
    ))
    .all(&db)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let filtered: Vec<_> = match visible {
        None => rows,
        Some(ref ids) => rows
            .into_iter()
            .filter(|r| ids.contains(&r.reporting_org_id))
            .collect(),
    };

    Ok(filtered
        .into_iter()
        .take(10)
        .map(|r| RecentSubmission {
            id: r.id,
            org_label: r.org_label,
            source_type: r.source_type,
            status: r.status,
            updated_at: r.updated_at,
        })
        .collect())
}
