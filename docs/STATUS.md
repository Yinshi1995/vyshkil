# Статус автономної сесії (поточний цикл)

Коротко оновлюється самою автономною сесією (tmux Remote Control, нічний раннер) — що робиться
ЗАРАЗ і що щойно завершено. Не лог усіх подій (те — в git-історії й `.claude/memory/MEMORY.md`),
лише поточний стан на момент останнього оновлення.

**Останнє оновлення**: 2026-10-03
**Поточна ціль**: усі пункти GOALS.md закриті — **DONE**

## Нещодавні зміни (2026-10-03)

- **Frontend мігровано на React 19** (SPA, Vite + TypeScript + shadcn/ui base-ui)
  — `web/` директорія, сервер автоматично знаходить `web/dist/` (spa.rs)
- **Dockerfile оновлено**: 3-stage build (Node frontend → Rust server → distroless runtime),
  cargo-leptos/wasm-opt/wasm32 більше не потрібні
- **Org hierarchy admin page**: master-detail tree layout, 3 таби (Основне/Код ВЧ/Підпорядкування),
  11 REST API endpoints, повний CRUD
- **Training site CRUD** в налаштуваннях
- **WhatsApp notification routing** (09-messaging.md §2): 4 міграції, NATS JetStream, pairing SSE
- **Comprehensive dev seed** (migration 000050)
