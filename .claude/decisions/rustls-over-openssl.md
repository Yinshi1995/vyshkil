---
tags: [decision, tls, security]
date: 2026-09-27
---

# rustls замість native-tls/openssl

**Чому:** sea-orm/sqlx налаштовані з фічею `runtime-tokio-rustls` замість `runtime-tokio-native-tls`.
rustls — чиста Rust-реалізація TLS без залежності від системного OpenSSL, що узгоджується з
рішенням [[docker-distroless-cc]] — рантайм-образ не тягне libssl-dev/openssl як окремий системний пакет.

**Де:** `server/Cargo.toml`, `migration/Cargo.toml` — фіча `runtime-tokio-rustls` у `sea-orm`/`sea-orm-migration`.

**Ціна:** якщо колись знадобиться специфічна OpenSSL-фіча (напр. клієнтські сертифікати в
нестандартному форматі) — доведеться переходити на native-tls.
