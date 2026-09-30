# syntax=docker/dockerfile:1

# Системний Chromium (не puppeteer-власний завантажений білд, §3: "Puppeteer вказує на нього,
# свій Chromium при install не качає") -- debian-slim має пакет `chromium` у стандартних репах.
FROM node:22-slim AS runtime

# chromium — сам браузер; решта — бібліотеки, яких chromium потребує в headless-режимі
# (шрифти для коректного рендеру сторінок, що містять кирилицю/емодзі в повідомленнях).
RUN apt-get update && apt-get install -y --no-install-recommends \
    chromium \
    fonts-liberation \
    fonts-noto-color-emoji \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Непривілейований користувач (§3: "запуск від непривілейованого користувача, headless") --
# node:22-slim уже має користувача `node` (uid 1000), використовуємо його, а не root.
WORKDIR /app
RUN mkdir -p /app/wa-session && chown -R node:node /app
USER node

COPY --chown=node:node package.json package-lock.json ./
# Без цього puppeteer's postinstall намагається скачати ВЛАСНИЙ Chromium (§3: свідомо НЕ робимо
# цього -- системний пакет вище, WA_PUPPETEER_EXECUTABLE_PATH нижче на нього вказує).
ENV PUPPETEER_SKIP_DOWNLOAD=true
# `--omit=dev`: typescript/@types/json-schema-to-typescript потрібні лише на етапі розробки
# (gen-types/typecheck), не в рантаймі контейнера.
RUN npm ci --omit=dev

COPY --chown=node:node src ./src
COPY --chown=node:node migrations ./migrations

ENV NODE_ENV=production
# Явно вказуємо системний Chromium -- без цього puppeteer/whatsapp-web.js спробували б шукати
# власний завантажений білд, якого тут немає (install script свідомо не качає його -- дивись
# package.json, postinstall блокується на верхньому рівні workspace, тут явний executablePath
# досягає того самого результату для самого контейнера).
ENV WA_PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium
ENV WA_SESSION_PATH=/app/wa-session
ENV HEALTH_PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD node -e "fetch('http://127.0.0.1:8080/healthz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"

CMD ["node", "--experimental-strip-types", "src/index.ts"]
