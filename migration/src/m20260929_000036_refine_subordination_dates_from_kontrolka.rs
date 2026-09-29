use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Уточнення дат переходу 17 АК → 7 КШР (Етап 6): dev-сід m20260927_000011 використав ОДНУ умовну
// дату "01.08.2026" для всіх 7 частин (докладних дат тоді не було — спека будувалась з фрагментів
// docs/, не з source_files/). Тепер є реальне джерело — Контролька (нвдпвднст).xlsx
// (`source_files/Зразок/26.09/`, читання дозволено, самих даних не цитуємо дослівно, лише
// агрегований факт "частина X перейшла під Y дата Z", 8 фактів разом): для кожної частини й дня
// вона перелічена (жирна "шапка" підрозділу над списком) під СВОЇМ поточним органом — перший
// день під НОВИМ органом і є реальною датою переходу. Сам факт переходу (які 7 частин, які органи)
// лишається правильним — коригуються лише дати. "5 омбр" тут — Контролька пише "5 ошбр"
// (можлива реорганізація типу частини за час переходу); резолюція лишається на існуючий org
// "5 омбр" (уже засіяний), перейменування — окреме питання, не це.
const REAL_TRANSITION_DATES: &[(&str, &str, &str)] = &[
    ("5 омбр", "7 КШР", "2026-08-21"),
    ("154 омбр", "7 КШР", "2026-08-23"),
    ("92 ошбр", "7 КШР", "2026-08-23"),
    ("44 оабр", "7 КШР", "2026-08-23"),
    ("61 омбр", "7 КШР", "2026-08-24"),
    ("142 омбр", "7 КШР", "2026-08-26"),
    ("225 ошп", "7 КШР", "2026-08-29"),
    ("67 омбр", "17 АК", "2026-08-03"),
];

const PLACEHOLDER_DATE: &str = "2026-08-01";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Спершу зсунути ПОЧАТОК нового періоду вперед (усі реальні дати пізніші за умовну
        // 01.08.2026), лише ПОТІМ подовжити старий період до тієї самої дати — інакше між двома
        // UPDATE на мить існує перетин (EXCLUDE-обмеження перевіряється одразу на кожному
        // операторі, не відкладено).
        for (unit, new_parent, real_date) in REAL_TRANSITION_DATES {
            db.execute_unprepared(&format!(
                "UPDATE subordination SET valid_from = '{real_date}'::date \
                 WHERE axis = 'staff' AND valid_from = '{PLACEHOLDER_DATE}'::date \
                   AND child_org_id = (SELECT id FROM org WHERE short_name = '{unit}') \
                   AND parent_org_id = (SELECT id FROM org WHERE short_name = '{new_parent}')"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "UPDATE subordination SET valid_to = '{real_date}'::date \
                 WHERE axis = 'staff' AND valid_to = '{PLACEHOLDER_DATE}'::date \
                   AND child_org_id = (SELECT id FROM org WHERE short_name = '{unit}')"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Зворотний порядок: спершу відсунути старий період назад до умовної дати (звільняє
        // проміжок), лише ПОТІМ повернути новий період на ту саму умовну дату.
        for (unit, new_parent, real_date) in REAL_TRANSITION_DATES {
            db.execute_unprepared(&format!(
                "UPDATE subordination SET valid_to = '{PLACEHOLDER_DATE}'::date \
                 WHERE axis = 'staff' AND valid_to = '{real_date}'::date \
                   AND child_org_id = (SELECT id FROM org WHERE short_name = '{unit}')"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "UPDATE subordination SET valid_from = '{PLACEHOLDER_DATE}'::date \
                 WHERE axis = 'staff' AND valid_from = '{real_date}'::date \
                   AND child_org_id = (SELECT id FROM org WHERE short_name = '{unit}') \
                   AND parent_org_id = (SELECT id FROM org WHERE short_name = '{new_parent}')"
            ))
            .await?;
        }
        Ok(())
    }
}
