---
title: "ДСК: номер і назва частини не показуються одночасно"
date: 2026-10-03
status: accepted
---

# Рішення: розділити відображення номеру та назви частин (ДСК)

## Контекст

Номер частини (А7384) і назва (128 овмбр) одночасно на одному екрані/документі — інформація
з грифом ДСК (Для службового користування). Користувач вимагає: ці дві одиниці інформації
НІКОЛИ не повинні з'являтися разом.

## Рішення

- `org_label` скрізь = лише `short_name` (без `(number_kind + number)` у дужках)
- `OrgDetail` API більше не повертає поле `number` клієнту
- Імпорт-шаблон xlsx: прихований аркуш "Довідники" містить лише коди ВЧ (`org_codes`),
  без назв — резолюція код→назва відбувається лише на сервері при імпорті
- Номер частини зберігається в БД і використовується для пошуку/резолюції (alias + pg_trgm),
  але не відображається поряд з назвою

## Обсяг змін

- `app/src/backend/repo/orgs.rs` — 6 функцій: list_orgs, search_orgs, default_org_listing,
  resolve_org, subordination_tree, direct_children, org_detail
- `app/src/backend/repo/auth.rs` — user_roles, list_submissions, list_training_groups
- `app/src/backend/repo/groups.rs` — list_groups_extended (org_label + organizer_label)
- `app/src/backend/repo/documents.rs` — grouped_org_ids, top_level_orgs, org_label,
  group_detail_for_corps, transferred_orgs_report
- `app/src/backend/repo/dictionaries.rs` — export_enums_for_template (OrgLabel → org_codes)
- `app/src/backend/documents/import_template.rs` — прибрана колонка "Назва ВЧ"
- `app/src/types/org.rs` — видалено `number` з `OrgDetail`
- `server/src/api.rs` — me_handler SQL
- `web/src/api/types.ts` — видалено `number` з `OrgDetail`
- `web/src/pages/org-detail.tsx` — orgLabel() → short_name only
- `web/src/api/mock.ts` — видалено number з mock data
- `app/src/pages/org_detail/mod.rs` — label без number
