# Taktoblik

Rust full-stack: Axum + Leptos 0.7 (SSR) + SeaORM + PostgreSQL.
Workspace: `app/` (спільні компоненти), `frontend/` (wasm-гідратація), `server/` (Axum SSR), `migration/` (SeaORM).

## Куди йти за контекстом

| Питання | Джерело |
|---|---|
| Що зроблено зараз / що в процесі | `.claude/memory/MEMORY.md` |
| Чому так, а не інакше (архітектурні рішення) | `.claude/memory/DECISIONS.md` → `.claude/decisions/*.md` |
| Контракт між app/frontend/server | `.claude/memory/INTERFACES.md` |
| "хто що викликає" / залежності символів | `cargo modules dependencies` — спробуй перед grep по всьому проєкту |
| Конкретна логіка в конкретному файлі | читай файл напряму |

Не дублюй деталі рішень і стану тут — вони в файлах вище (progressive disclosure).
