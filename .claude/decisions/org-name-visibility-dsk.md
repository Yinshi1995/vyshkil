# Видимість назви частини (ДСК)

**Дата**: 2026-10-05
**Контекст**: Назва частини (тип підрозділу — «омбр», «ошбр», «АК» тощо) поряд з заходами підготовки
є грифованою інформацією. На екрані з заходами звичайний користувач повинен бачити лише номер
в/ч (формат «А4955», «Т1234» — кирилиця), а не повну назву («153 омбр», «128 овмбр»).

## Рішення

1. `user_account.can_see_org_names BOOLEAN NOT NULL DEFAULT FALSE` — прапорець, чи бачить
   користувач назви частин.
2. Хто бачить назву:
   - Адміни — завжди (визначається з активної ролі, не з прапорця).
   - Користувачі з `can_see_org_names = true` — адмін вмикає в налаштуваннях для окремих акаунтів.
   - Решта — бачать номер в/ч (`org.number_kind || org.number` → «А4955»).
   - Якщо у підрозділу немає номера в/ч (`number IS NULL`), показується `short_name` як є.
3. Маскування — DB-backed пост-обробка на Rust-рівні у хендлерах `server/src/api.rs`:
   - `org_number_labels(db, &org_ids)`: SQL-запит `COALESCE(CASE number_kind WHEN 'A' THEN 'А'
     WHEN 'T' THEN 'Т' ELSE number_kind END || number, short_name)`. Повертає `HashMap<i32, String>`.
   - `org_number_label_one(db, org_id)`: обгортка для одного id.
   - Старий string-based `mask_org_label()` видалено.
4. Поля, що маскуються: `org_label` та `organizer_label` в DataGroupRow, org_label в DiscrepancyRow,
   DashboardStats (recent_submissions, recent_discrepancies), dashboard_by_org, dashboard_disc_chart,
   dashboard_staffing, directory, UserRoleRow (login response), actor org_label (/auth/me).
5. Адмін-ендпоінти (адмін-список подань/груп/юзерів) — без маскування (адмін завжди бачить).
6. `AuthUser.can_see_org_names: bool` — обчислюється при `require_auth`: `is_admin || ua.can_see_org_names`.
