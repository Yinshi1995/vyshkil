---
tags: [decisions-index]
date: 2026-09-27
---

# Журнал архітектурних рішень

Індекс — по одному рядку на рішення. Повний запис (чому, альтернативи, ціна) — окремий атомарний файл у `.claude/decisions/`.

- [SeaORM замість Diesel](../decisions/seaorm-vs-diesel.md) — async-first ORM без блокуючого мосту, природно лягає на tokio/Axum/Leptos.
- [mimalloc як глобальний аллокатор](../decisions/mimalloc-allocator.md) — менший RSS і краще повернення пам'яті ОС на слабкому сервері.
- [Ручний tokio runtime замість #[tokio::main]](../decisions/manual-tokio-runtime.md) — `worker_threads` читається з env у рантаймі, а не запікається в бінарник.
- [opt-level=z + lto + panic=abort у релізі](../decisions/release-profile-size.md) — розмір бінарника важливіший за пікову швидкодію на цьому сервері.
- [distroless/cc, а не scratch, для рантайм-образу](../decisions/docker-distroless-cc.md) — бінарник динамічно лінкований проти glibc, scratch був би без libc.
- [rustls замість native-tls/openssl](../decisions/rustls-over-openssl.md) — прибирає OpenSSL з рантайм-образу, сумісно з distroless.
- [cargo-modules замість Graphify](../decisions/cargo-modules-over-graphify.md) — Graphify виявився свіжим нерозкрученим інструментом з auto-exec hook (supply-chain ризик); cargo-modules — перевірена Rust-специфічна заміна.
- [Smart App Control блокує локальну збірку → перевірка через Docker](../decisions/smart-app-control-blocks-local-build.md) — Windows блокує непідписані build-script бінарники; вимикати не стали (одностороння дія), збірку перевіряємо в Docker.

Рішення специфікації домену (`docs/spec/`, затверджено замовником 27.09.2026):

- [Людей поіменно не зберігаємо](../decisions/no-personal-data.md) — облік групами й кількостями; ПІБ з імпорту агрегуються і вичищаються.
- [Кількості тільки подіями, бітемпорально](../decisions/event-sourced-counts-bitemporal.md) — `occurred_on` + `recorded_at`, ніяких полів-лічильників.
- [Дві осі підпорядкування з історією + замикання](../decisions/temporal-subordination-closure.md) — матеріалізоване `subordination_closure`, не рекурсивні CTE.
- ["Як подали" vs "як вважаємо"](../decisions/reported-vs-canonical.md) — два шари даних і функція визначення канону з пріоритетом.
- [Нормалізація синонімів в `app`](../decisions/alias-normalization-in-app.md) — одна функція для SSR і WASM, порівняння по `alias.norm`.
- [text+CHECK замість Postgres ENUM](../decisions/text-enums-not-pg-enums.md) — додавання значення не має вимагати міграції типу.
- [Знеособлена черга сповіщень](../decisions/depersonalized-notification-outbox.md) — `notification_outbox` + трейт `NotificationChannel`, шаблон без доменних даних.
- [Транслітерація normalize() — лише Latin→Cyrillic](../decisions/normalize-direction-one-way.md) — зворотний напрямок ("Vampire"↔"Вампір") через seed-рядки alias, не героїзм в одній функції.
- [Джерело візуального стилю UI](../decisions/ui-visual-style-source.md) — палітра з 05-documents.md (панелі/акцент/текст), референс-pptx лише підтвердив напрямок.
- [CSS-колокація (07) відкладена](../decisions/css-colocation-deferred.md) — cargo-leptos не розгортає `@import` і обробляє стилі до збірки Rust-крейта, тож і `build.rs`-фолбек не встигає; `main.css` лишається одним файлом.
- [Структура app/src: спільне в корені, специфічне поруч](../decisions/code-layout-colocation.md) — правило двох + карти CLAUDE.md + `app/tests/architecture.rs`; заодно `policy` переїхав у `app/src/backend` (був недосяжний для `#[server]`-функцій) і server fn почали реально перевіряти права.
- [`source_files/` — читати дозволено](../decisions/source-files-read-access.md) — рішення користувача 2026-09-28; звужена зміна: читати можна, комітити файли/цитувати сирий текст дослівно — досі не можна.
- [`chrono` у `domain/` (і в WASM)](../decisions/chrono-in-domain.md) — розбір/валідація дат (02 §4, 03 §4) потребує реальної календарної арифметики, не лише порівняння готових `YYYY-MM-DD` рядків; без `"clock"`/`"wasmbind"`, перевірено компілюється під wasm32.
