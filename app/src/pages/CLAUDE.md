# app/src/pages — сторінки-маршрути

- Можна: `widgets`, `layout`, `components`, `services`, `types`, `domain`, `hooks`, `state`.
  Не можна: інша сторінка (`crate::pages::<b>` всередині `pages::<a>` — забороняє приватність
  модулів + перевіряє `app/tests/architecture.rs`).
- Нова сторінка → тека `pages/<p>/` (або один файл `pages/<p>.rs`, якщо тривіальна) + рядок
  у `routes.rs` + рядок тут (07 §3.10).
- Класи — атоми `style`-крейту (skill `styling`), не інлайн-стилі й не новий BEM-CSS.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `discrepancies/` | екран розбіжностей — горизонтальна звірка, лише перегляд (04, Етап 8 зріз 1, своя карта) | `routes.rs` (`/discrepancies`) |
| `documents/` | генерація документів — D1 (05, Етап 7, своя карта) | `routes.rs` (`/documents`) |
| `home/` | головна: лічильники, пошук, дерево підпорядкування (своя карта) | `routes.rs` (`/`) |
| `import/` | превʼю імпорту КВід/ІВС/Архів ВЧ — інша форма даних або окремий `source_type` (03, Етап 5-6, своя карта) | `routes.rs` (`/import`) |
| `login/` | сторінка входу — форма логін+пароль → `auth_login` → cookie → redirect (12-auth.md, своя карта) | `routes.rs` (`/login`) |
| `org_detail/` | картка частини з історією (своя карта) | `routes.rs` (`/org/:id`) |
| `settings/` | налаштування: теми, WhatsApp, черги, learned-синоніми (своя карта) | `routes.rs` (`/settings`) |
| `training_form/` | сітка введення груп: вручну АБО файлом Фах/БпС/Терміни, одна чернетка (02, своя карта) | `routes.rs` (`/training-form`) |
| `change_password/` | примусова зміна пароля після першого входу (12-auth.md) | `routes.rs` (`/change-password`) |
| `request_account/` | запит на створення облікового запису (12-auth.md) | `routes.rs` (`/request-account`) |
