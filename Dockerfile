# syntax=docker/dockerfile:1

# ---------- builder ----------
# rust:slim замість повного rust:latest — усі build-тули (cargo, rustc) ті самі,
# але без непотрібних для збірки GUI/doc-пакетів образу за замовчуванням.
FROM rust:1-slim-bookworm AS builder

# rust:slim не має навіть make/gcc "з коробки" (лише сам rustc/cargo) — тому build-essential,
# а не набір окремих пакетів по одному: якась транзитивна залежність cargo-leptos тягне
# openssl-sys/vendored, чий build-скрипт компілює OpenSSL через Configure+make (Perl + C toolchain).
# сама БД-комунікація йде через rustls (runtime-tokio-rustls), тому в рантайм-образі openssl не знадобиться.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    perl \
    && rm -rf /var/lib/apt/lists/*

# wasm-opt (Binaryen) кладемо в образ заздалегідь із локально завантаженого й перевіреного (sha256)
# архіву, а не даємо cargo-leptos качати його з GitHub під час білду: цей конкретний download
# виявився дуже нестабільним/повільним у мережі. apt дає лише стару binaryen 108, несумісну з
# форматом, який очікує саме ця версія cargo-leptos (version_123) — тому саме цю версію, з файлу.
COPY .docker-tools/binaryen.tar.gz /tmp/binaryen.tar.gz
RUN tar -xzf /tmp/binaryen.tar.gz -C /usr/local --strip-components=1 binaryen-version_123/bin/wasm-opt && \
    rm /tmp/binaryen.tar.gz

# wasm32-unknown-unknown — таргет для frontend-крейта: без нього cargo-leptos не збере wasm-частину.
RUN rustup target add wasm32-unknown-unknown

# Кеш-маунти (BuildKit) — без них КОЖЕН повторний білд перекачував реєстр crates.io і
# перекомпільовував увесь граф залежностей з нуля (COPY . . нижче все одно зкидає кеш цього шару).
# /root/.cache/cargo-leptos — де cargo-leptos тримає завантажені wasm-bindgen/wasm-opt/sass;
# без цього кеша навіть невдалий (обірваний мережею) закачування wasm-opt повторювався щоразу.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/root/.cache/cargo-leptos \
    cargo install --locked cargo-leptos

WORKDIR /app
COPY . .

# --release використовує профіль з opt-level=z/lto/panic=abort із кореневого Cargo.toml —
# саме там, а не тут, живе вся логіка "менший бінарник ціною часу збірки".
# Свідомо БЕЗ cache-mount на /app/target: на цій машині кеш-маунти показали дуже погану
# продуктивність I/O (cargo "зависав" без CPU/мережевої активності на fingerprint-скані
# великого кеш-маунта) — краще передбачувані ~3.5 хв повної компіляції щоразу, ніж непередбачуваний stall.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/root/.cache/cargo-leptos \
    cargo leptos build --release

# ---------- runtime ----------
# distroless/cc: немає shell, package manager і зайвих бібліотек — лише glibc/libgcc,
# необхідні динамічно лінкованому release-бінарнику; менша поверхня атаки і менший образ, ніж повний debian.
FROM gcr.io/distroless/cc-debian12 AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/server ./server
COPY --from=builder /app/target/site ./site

# LEPTOS_SITE_ROOT відносний до WORKDIR — сюди cargo-leptos поклав JS/WASM/CSS бандл.
ENV LEPTOS_SITE_ROOT=site
ENV LEPTOS_SITE_ADDR=0.0.0.0:3000
ENV RUST_LOG=info,sqlx=warn

EXPOSE 3000

# Exec-форма обов'язкова: у distroless немає /bin/sh, щоб інтерпретувати shell-форму CMD.
ENTRYPOINT ["./server"]
