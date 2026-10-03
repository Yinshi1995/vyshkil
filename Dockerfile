# syntax=docker/dockerfile:1

# ---------- frontend (React SPA) ----------
FROM node:22-alpine AS frontend

WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npx vite build

# ---------- builder (Rust server) ----------
# rust:slim замість повного rust:latest — усі build-тули (cargo, rustc) ті самі,
# але без непотрібних для збірки GUI/doc-пакетів образу за замовчуванням.
FROM rust:1-slim-bookworm AS builder

# build-essential + mold: mold — лінкер із .cargo/config.toml (infra/ansible/roles/rust_toolchain);
# pkg-config/libssl-dev — на випадок транзитивних залежностей, що тягнуть openssl-sys.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    mold \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# --release використовує профіль з opt-level=z/lto/panic=abort із кореневого Cargo.toml —
# саме там, а не тут, живе вся логіка "менший бінарник ціною часу збірки".
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build -p server --release

# ---------- runtime ----------
# distroless/cc: немає shell, package manager і зайвих бібліотек — лише glibc/libgcc,
# необхідні динамічно лінкованому release-бінарнику; менша поверхня атаки і менший образ, ніж повний debian.
FROM gcr.io/distroless/cc-debian12 AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/server ./server
COPY --from=frontend /app/web/dist ./web/dist

# LEPTOS_SITE_ADDR — адреса, на якій сервер слухає (leptos get_configuration читає з env).
# web/dist/ — React SPA, сервер автоматично його знаходить (server/src/spa.rs).
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000
ENV RUST_LOG=info,sqlx=warn

EXPOSE 3000

# Exec-форма обов'язкова: у distroless немає /bin/sh, щоб інтерпретувати shell-форму CMD.
ENTRYPOINT ["./server"]
