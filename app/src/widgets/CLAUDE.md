# app/src/widgets — доменні компоненти, які використовують ≥ 2 сторінки

- Можна: `services`, `hooks`, `types`, `domain`. Не можна: `pages`, `backend`.
- Використовується лише однією сторінкою → назад у `pages/<p>/components/` (07 §2.1).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `actor_notice.rs` | `ActorNotice` — "оберіть актора вгорі" замість мовчазного порожнього результату | `pages::home` (пошук, дерево), `pages::org_detail` |
| `group_grid/` | `Grid` — сітка рядків-груп з клавіатурою й автокомплітом; `OrgAutocomplete` реекспортовано й поза `Grid` (своя карта) | `pages::training_form`, `pages::import`, `pages::documents` (лише `OrgAutocomplete`) |
