# app/src/pages — сторінки-маршрути

- Можна: `widgets`, `layout`, `components`, `services`, `types`, `domain`, `hooks`, `state`.
  Не можна: інша сторінка (`crate::pages::<b>` всередині `pages::<a>` — забороняє приватність
  модулів + перевіряє `app/tests/architecture.rs`).
- Нова сторінка → тека `pages/<p>/` (або один файл `pages/<p>.rs`, якщо тривіальна) + рядок
  у `routes.rs` + рядок тут (07 §3.10).
- Класи — атоми `style`-крейту (skill `styling`), не інлайн-стилі й не новий BEM-CSS.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `admin_queues/` | outbox-відставання, стан стрімів, DLQ з повторною відправкою, лише `admin` (09 §5, Фаза 4, своя карта) | `routes.rs` (`/admin/queues`) |
| `admin_whatsapp/` | прив'язка WhatsApp-сесії нотифікатора — QR/pairing-код через SSE, лише `admin` (09 §4, своя карта) | `routes.rs` (`/admin/whatsapp`) |
| `dictionaries/` | довідники Етапу 2 + черга learned-синонімів (своя карта) | `routes.rs` (`/dictionaries`) |
| `discrepancies/` | екран розбіжностей — горизонтальна звірка, лише перегляд (04, Етап 8 зріз 1, своя карта) | `routes.rs` (`/discrepancies`) |
| `documents/` | генерація документів — D1 (05, Етап 7, своя карта) | `routes.rs` (`/documents`) |
| `home/` | головна: лічильники, пошук, дерево підпорядкування (своя карта) | `routes.rs` (`/`) |
| `import/` | превʼю імпорту КВід/ІВС/Архів ВЧ — інша форма даних або окремий `source_type` (03, Етап 5-6, своя карта) | `routes.rs` (`/import`) |
| `org_detail/` | картка частини з історією (своя карта) | `routes.rs` (`/org/:id`) |
| `styleguide.rs` | живий довідник атомів/тем стильової системи, лише `admin` (08 §Фаза 2) | `routes.rs` (`/styleguide`) |
| `training_form/` | сітка введення груп: вручну АБО файлом Фах/БпС/Терміни, одна чернетка (02, своя карта) | `routes.rs` (`/training-form`) |
| `vos_lookup/` | підказка "ОВТ/сленг → ВОС" (02 §3, своя карта) | `routes.rs` (`/vos-lookup`) |
