//! Інтеграційні тести доступу (`backend::policy`) проти реальної БД — окрема тестова база,
//! міграції з нуля (той самий підхід, що й `app/tests/orgs.rs`, див. `common`). Лише під `ssr`.
#![cfg(feature = "ssr")]

mod common;

use app::backend::policy::{can_view_org, restrict_tree, visible_org_ids};
use app::backend::repo::orgs::subordination_tree;
use app::types::actor::{Actor, Role};
use common::fresh_test_db;

/// "org_editor не бачить чужого піддерева" — org_editor з 17 АК бачить свою організацію й
/// власне піддерево (241 обр ТрО), але НЕ бачить сусіднє командування 7 КШР і те, що під ним
/// (142 омбр — саме та частина, яку сід переносить 17 АК → 7 КШР у серпні 2026, 06-roadmap.md).
#[tokio::test]
async fn org_editor_does_not_see_foreign_subtree() {
    let Some(db) = fresh_test_db().await else { return };

    let all = subordination_tree(&db, "2026-09-20", "staff").await.unwrap();
    let org_id = |label: &str| all.iter().find(|r| r.label == label).unwrap().id;

    let seventeenth_ak = org_id("17 АК");
    let seventh_kshr = org_id("7 КШР");
    let unit_under_17 = org_id("241 обр ТрО (4076)");
    let unit_under_kshr = org_id("142 омбр");

    let org_editor_17 = Actor { org_id: seventeenth_ak, role: Role::OrgEditor };

    // --- can_view_org: точковий доступ ---
    assert!(
        can_view_org(&db, org_editor_17, seventeenth_ak).await.unwrap(),
        "org_editor бачить свою організацію"
    );
    assert!(
        can_view_org(&db, org_editor_17, unit_under_17).await.unwrap(),
        "org_editor бачить власне піддерево"
    );
    assert!(
        !can_view_org(&db, org_editor_17, seventh_kshr).await.unwrap(),
        "org_editor з 17 АК НЕ бачить сусіднє командування 7 КШР"
    );
    assert!(
        !can_view_org(&db, org_editor_17, unit_under_kshr).await.unwrap(),
        "org_editor з 17 АК НЕ бачить піддерево 7 КШР"
    );

    // --- visible_org_ids + restrict_tree: те саме на рівні дерева (як реально фільтрує
    // get_subordination_tree, pages/home/server.rs) ---
    let visible = visible_org_ids(&db, org_editor_17).await.unwrap().expect("org_editor не admin");
    let restricted = restrict_tree(all.clone(), &visible);

    assert!(restricted.iter().any(|r| r.id == seventeenth_ak), "своя організація лишається в дереві");
    assert!(restricted.iter().any(|r| r.id == unit_under_17), "власне піддерево лишається в дереві");
    assert!(
        !restricted.iter().any(|r| r.id == seventh_kshr),
        "7 КШР не має бути у видимому org_editor'у 17 АК дереві: {restricted:?}"
    );
    assert!(
        !restricted.iter().any(|r| r.id == unit_under_kshr),
        "піддерево 7 КШР не має бути видимим: {restricted:?}"
    );

    // --- admin бачить усе ---
    let admin = Actor { org_id: seventeenth_ak, role: Role::Admin };
    assert!(can_view_org(&db, admin, seventh_kshr).await.unwrap(), "admin бачить будь-яку org");
    assert!(visible_org_ids(&db, admin).await.unwrap().is_none(), "admin — без обмежень (None)");
}
