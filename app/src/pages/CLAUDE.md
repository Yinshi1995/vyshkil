# app/src/pages — сторінки-маршрути

- Можна: `widgets`, `layout`, `components`, `services`, `types`, `domain`, `hooks`, `state`.
  Не можна: інша сторінка (`crate::pages::<b>` всередині `pages::<a>` — забороняє приватність
  модулів + перевіряє `app/tests/architecture.rs`).
- Нова сторінка → тека `pages/<p>/` (або один файл `pages/<p>.rs`, якщо тривіальна) + рядок
  у `routes.rs` + рядок тут (07 §3.10).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `dictionaries/` | довідники Етапу 2 + черга learned-синонімів (своя карта) | `routes.rs` (`/dictionaries`) |
| `home/` | головна: лічильники, пошук, дерево підпорядкування (своя карта) | `routes.rs` (`/`) |
| `org_detail/` | картка частини з історією (своя карта) | `routes.rs` (`/org/:id`) |
| `training_form/` | сітка введення груп (02, своя карта) | `routes.rs` (`/training-form`) |
| `vos_lookup/` | підказка "ОВТ/сленг → ВОС" (02 §3, своя карта) | `routes.rs` (`/vos-lookup`) |
