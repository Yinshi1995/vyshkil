# app/src/services — `#[server]`-функції, потрібні ≥ 2 місцям

- Можна: `backend`, `types`, `domain`. Не можна: `pages`, `layout`, `widgets` (напрямок
  залежностей — services нижче за pages/widgets, 07 §2.3).
- Функція, яку використовує лише одна сторінка → переносити в `pages/<p>/server.rs` (07 §2.1).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `health.rs` | `health_check` — пінг БД, smoke-test | `pages/home` |
| `orgs.rs` | `list_orgs` (без прав), `search_orgs` (нечіткий пошук, звужений до видимого піддерева) | `layout::ActorSwitcher`, `pages/home`, `pages/training_form` |
| `dictionaries.rs` | `get_dictionaries_overview` — усі прості довідники Етапу 2 одним викликом | `pages/dictionaries`, `pages/training_form` |
