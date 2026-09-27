---
tags: [decision, docker, deployment]
date: 2026-09-27
---

# distroless/cc, а не scratch, для рантайм-образу

**Чому:** сервер збирається на `rust:1-slim-bookworm` (glibc-таргет `x86_64-unknown-linux-gnu`),
release-бінарник динамічно лінкований проти glibc/libgcc. `FROM scratch` не містить жодних
системних бібліотек — бінарник просто не запуститься. `gcr.io/distroless/cc-debian12` дає
рівно потрібний мінімум (glibc, libgcc, libssl-runtime), без shell і пакетного менеджера.

**Де:** `Dockerfile`, стадія `runtime`.

**Альтернатива на майбутнє:** якщо перейти на статичну лінковку через
`x86_64-unknown-linux-musl`, можна буде повернутись до `scratch` і зменшити образ ще сильніше.
