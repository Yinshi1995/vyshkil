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
