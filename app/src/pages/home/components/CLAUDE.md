# app/src/pages/home/components — компоненти, потрібні лише головній сторінці

- Можна: `pages::home::server` (через `crate::pages::home::server::...` — видимий, бо приватні
  модулі бачать одне одного всередині тієї самої сторінки), `types`, `domain`.
  Не можна: інші сторінки.
- Використовує ще й інша сторінка → переносити в `widgets/` (07 §2.1), не копіювати.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `org_search.rs` | `OrgSearch` — нечіткий пошук (02 §3) | `pages/home/mod.rs` |
| `subordination_tree.rs` | `SubordinationTree` — дерево на дату з перемикачем осі | `pages/home/mod.rs` |
