# services/notifier — сервіс сповіщень (TypeScript, Node 22 LTS, `whatsapp-web.js`)

**Єдиний НЕ-Rust код репозиторію** (`.claude/decisions/notifier-ts-whatsapp-web-js.md`,
09-messaging.md §3.7) — не Cargo workspace member, окремий Docker-образ, власна БД/роль
Postgres (`notifier`/`notifier_test`, не бачить таблиць застосунку).

- Рантайм — Node 22 LTS, **не Bun** (перевірено емпірично — див. рішення). Виконання без
  окремого compile-кроку: `node --experimental-strip-types src/*.ts` — тому жодних TS-фіч, які
  не стираються (enum, namespace, parameter properties): `tsconfig.json`'s
  `erasableSyntaxOnly: true` ловить це на `npm run typecheck`, роби його ПЕРЕД запуском нового коду.
- Без ORM (3 таблиці) — `postgres` (porsager) напряму, SQL-міграції `migrations/*.NNN.sql` +
  раннер `src/migrate.ts`.
- Типи повідомлень — НЕ ручні дублікати: `contracts/schema/*.json` (Rust `schemars`) →
  `npm run gen-types` (`json-schema-to-typescript`) → `src/generated/*.d.ts` (не редагувати
  вручну, перезаписується). Рантайм-перевірка вхідних повідомлень — окремо, вручну підтримувані
  zod-схеми в `schemas.ts` (мають описувати ТУ САМУ форму — розсинхрон ловиться лише вручну).
- Тести — вбудований `node:test` (`npm test`), без окремого фреймворку. Інтеграційні
  (`delivery.test.ts`) — проти реального Postgres, `FakeChannel` замість реального WhatsApp
  (потребують `NOTIFIER_DATABASE_URL`, інакше пропускаються — той самий підхід, що Rust-бік).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `config.ts` | Плоска конфігурація з env (`NOTIFIER_DATABASE_URL`, `NATS_URL`, `WA_*`) | увесь сервіс |
| `db.ts` | `postgres`-клієнт + `isAlreadyProcessed`/`markProcessed`/`contactsForOrg`/`insertDeliveryLog` | `delivery.ts`, `commands.ts` |
| `migrate.ts` | `runMigrations` — простий SQL-раннер (трек у `_migrations`) | `index.ts`, `npm run migrate` |
| `nats.ts` | `connectNats`/`bindNotifyCmdConsumer`/`publishNotifyResult`/`publishStatus` — JetStream + KV (`notifier_status`) | `index.ts` |
| `schemas.ts` | zod-схеми (валідація вхідних `NotifySend`) | `delivery.ts` |
| `templates.ts` | Єдиний знеособлений шаблон (04 §5) | `delivery.ts`, `commands.ts` (test-надсилання) |
| `mask.ts` | Маскування номера перед журналом/логами | `delivery.ts`, `pairing.ts` |
| `delivery.ts` | `processNotifySend` — валідація→inbox→рендер→надсилання→журнал, одна транзакція | `index.ts` |
| `pairing.ts` | `PairingStateMachine` — стан прив'язки WhatsApp, публікує в NATS KV | `index.ts`, `commands.ts` |
| `commands.ts` | 4 NATS request-reply команди адмінки (`pair.qr`/`pair.code`/`logout`/`test`) | `index.ts` |
| `health.ts` | `/healthz`/`/readyz` | `index.ts` |
| `whatsapp/client.ts` | `createWhatsAppClient` — реальний `whatsapp-web.js` Client, `LocalAuth`, системний Chromium | `index.ts` |
| `whatsapp/channel.ts` | `Channel`/`WhatsAppChannel`/`FakeChannel` (тести) | `delivery.ts`, `commands.ts`, тести |
| `index.ts` | Точка входу — з'єднує все, consumer-цикл із watchdog | — |
| `migrations/*.sql` | Схема нотифікатора (`org_contact`/`delivery_log`/`inbox`) | `migrate.ts` |
| `src/generated/*.d.ts` | Згенеровані TS-типи з `contracts/schema/*.json` — НЕ редагувати вручну | усі модулі, що працюють з конвертами |
