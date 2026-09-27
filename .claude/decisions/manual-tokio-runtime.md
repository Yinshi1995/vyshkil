---
tags: [decision, tokio, runtime]
date: 2026-09-27
---

# Ручний tokio runtime замість #[tokio::main]

**Чому:** `#[tokio::main(flavor = "multi_thread", worker_threads = N)]` вимагає, щоб N був літералом
на етапі компіляції. Кількість воркер-потоків має відповідати реальним ресурсам конкретного сервера,
а не бути захардкоджена в бінарнику — тому рантайм будується вручну через
`tokio::runtime::Builder::new_multi_thread().worker_threads(n)`, де `n` читається зі
`SERVER_WORKER_THREADS` (env) у `fn main()`, до запуску async-коду.

**Де:** `server/src/main.rs`.

**Ціна:** трохи більше boilerplate-коду в `main()` порівняно з атрибутом-макросом.
