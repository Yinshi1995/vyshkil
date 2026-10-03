---
tags: [spec, architecture, messaging, microservices, notifications]
date: 2026-09-30
status: затверджено напрям; деталі уточнює агент у Фазі 0
---

# 09. Обмін повідомленнями між сервісами (брокер)

## 1. Навіщо і чого НЕ робимо через брокер

Брокер потрібен для **асинхронної** взаємодії між окремими процесами: основний застосунок →
сервіс сповіщень (WhatsApp і далі інші канали), а згодом — важкі воркери (OCR, генерація
документів) і сервіс автентифікації (події на кшталт "користувача створено").

Через брокер **не** йде:
- відповідь на запит з UI (server functions лишаються синхронними, читають Postgres напряму);
- перевірка автентифікації/прав на кожен запит — токен (JWT/PASETO) перевіряється локально
  публічним ключем; брокер для auth — тільки події (відкликано ключ, змінено роль).
Правило: якщо викликач чекає відповідь, щоб показати її користувачу, — це не брокер.

## 2. Вибір брокера

| Варіант | Пам'ять на одному сервері | Рішення |
|---|---|---|
| Apache Kafka (KRaft) | JVM, реально 1–2 ГБ+ | ні — промисловий масштаб, якого нема; з'їдає бюджет 8 ГБ, вимагає адміністрування |
| Redpanda | C++, розрахований на ≥ 1–2 ГБ/ядро | ні — ті самі причини |
| RabbitMQ | Erlang, ~150–300 МБ | можливо, але важчий, стріми — не рідна модель, Rust-клієнт менш зрілий |
| Лише Postgres (SKIP LOCKED / pgmq) | 0 | достатньо для одного споживача, але не дає ізоляції сервісів (нотифікатору потрібен доступ до БД застосунку) |
| **NATS + JetStream** | **один бінарник, ~50–150 МБ** | **так**: персистентні стріми, work-queue, повтори, DLQ, дедуплікація, права на теми, офіційний `async-nats` |

Бюджет пам'яті на сервері (8 ГБ): Postgres ~2–3 ГБ, застосунок ~200–400 МБ, NATS ≤ 256 МБ (ліміт
контейнера), нотифікатор ≤ 64 МБ. Лишається запас під ОС, кеш і фонові задачі.

## 3. Архітектура

```
 [app/server] ── транзакція: дані + рядок outbox ──► Postgres (схема app)
      │
      └─ relay (задача в server) ── SKIP LOCKED ──► NATS JetStream ── stream NOTIFY_CMD ──► [notifier]
                                                       ▲                                     │
 [app/server] ◄── consumer (результати) ── stream NOTIFY_RESULT ◄── publish ────────────────┘
                                                                         notifier: власна схема Postgres
                                                                         (контакти, журнал доставки, inbox)
                                                                         + єдиний, хто має вихід в інтернет
```

### 3.1. Transactional outbox (обов'язково)
- Таблиця `outbox` у БД застосунку: `id (uuid v7), subject, payload jsonb, headers jsonb,
  created_at, published_at null, attempts, last_error`.
- Доменна дія пише свої дані **і** рядок outbox **в одній транзакції**. Прямий publish з
  обробника запиту заборонено.
- Relay (фонова задача в `server`): бере пачку `WHERE published_at IS NULL ORDER BY id
  FOR UPDATE SKIP LOCKED LIMIT n`, публікує в JetStream з заголовком `Nats-Msg-Id = id`
  (дедуплікація на боці брокера), чекає ack, ставить `published_at`. Збій → `attempts++`,
  backoff. Будить його `LISTEN/NOTIFY` з тригера на outbox + таймер-страховка.
- NATS недоступний → застосунок працює як завжди, outbox накопичується, потім доганяє.

### 3.2. Inbox (ідемпотентність споживачів)
Доставка — "щонайменше один раз". Кожен споживач тримає таблицю `inbox(message_id primary key,
processed_at)` і обробляє повідомлення в транзакції разом зі вставкою в inbox: дубль →
пропустити й підтвердити.

### 3.3. Теми (subjects) і стріми
Іменування: `vyshkil.<контекст>.<подія|команда>.v<N>`.

| Subject | Тип | Хто публікує | Хто споживає | Stream / retention |
|---|---|---|---|---|
| `vyshkil.notify.send.v1` | команда | app (relay) | notifier | `NOTIFY_CMD`, work-queue, file storage |
| `vyshkil.notify.result.v1` | подія (delivered / failed / suppressed) | notifier | app | `NOTIFY_RESULT`, limits 30 днів |
| `vyshkil.discrepancy.opened.v1`, `.resolved.v1` | доменна подія | app | (поки ніхто; майбутні аналітика/аудит) | `EVENTS`, limits 30 днів |
| `vyshkil.dlq.>` | мертві листи | брокер/споживачі після `max_deliver` | адмін-екран | `DLQ`, limits 90 днів |

Рішення, **кого і коли** сповіщати (групування розбіжностей, антиспам "не частіше N годин"),
приймає основний застосунок — це доменна логіка (04 §5). Нотифікатор виконує команду "доставити",
плюс має власний технічний rate-limit і повтори.

### 3.4. Конверт повідомлення (спільний крейт `contracts`)
```
Envelope<T> { id: Uuid(v7), type: &'static str, version: u16, occurred_at, producer,
              correlation_id, causation_id, payload: T }
```
Формат — JSON (serde). Типи повідомлень — Rust-структури в `contracts/`, які використовують обидві
сторони: зміна контракту = зміна в одному місці + збільшення версії (`v2` — новий subject,
старий живе, поки є споживачі). Споживач ігнорує невідомі поля.

### 3.5. Приватність — вбудована в контракт
`NotifySend.v1 { recipient_org_id: i64, template: NotifyTemplate (enum), dedupe_key }` — і все.
**Ні телефонів, ні назв частин, ні кількостей, ні вільного тексту.** Нотифікатор сам знає
контакти організацій (`org_contact` переїжджає в його схему, адмінка контактів — через його
API або події) і рендерить знеособлений шаблон. Тест у `contracts`: жоден тип у `notify.*` не
має полів `String` з довільним текстом (тільки enum/ідентифікатори).

### 3.6. Безпека
- NATS з автентифікацією (user/password на сервіс, `docker/nats-server.conf`,
  `.claude/decisions/broker-nats-jetstream.md` §Фаза 3 — обране рішення й чому не nkeys/окремі
  accounts) і **правами на subject-и**: app — publish `vyshkil.notify.send.>`, `vyshkil.*.*.v*`
  (свої події), subscribe `vyshkil.notify.result.>`; notifier — subscribe
  `vyshkil.notify.send.>`, publish `vyshkil.notify.result.>`, `vyshkil.dlq.notify.>`. Перевірено
  проти реального сервера (дозволені/заборонені дії, невірний пароль).
- NATS слухає лише внутрішню docker-мережу; моніторинговий порт 8222 — тільки localhost.
- Вихід в інтернет (файрвол) — **тільки контейнеру notifier**, на адреси провайдера WhatsApp.
  Основний застосунок і NATS в інтернет не ходять.
- Секрети провайдера (токен WhatsApp Business API) — тільки в середовищі notifier.

### 3.7. Сервіс `notifier`
- **TypeScript, окремий контейнер, НЕ Rust** (`.claude/decisions/notifier-ts-whatsapp-web-js.md`
  — замінює попереднє рішення на цю секцію): зріла екосистема WhatsApp-клієнтів існує лише в npm.
  Рантайм — **Node 22 LTS** (не Bun — перевірено емпірично, Bun не дав жодної переваги й не
  усунув знайдену проблему сумісності Puppeteer/Chromium, рішення документує тест).
  `whatsapp-web.js` (Apache-2.0) — бібліотека клієнта (реверс-інжиніринг WhatsApp Web, не
  офіційне API — звідси ризик блокування номера, мітигований окремим службовим номером і
  технічним rate-limit'ом).
- `services/notifier/` — своя БД/схема в тому ж Postgres з окремим користувачем (не бачить
  таблиць застосунку): `org_contact`, `delivery_log`, `inbox`. Прості SQL-міграції (не ORM —
  3 таблиці, `postgres` (porsager)-клієнт напряму).
- NATS-клієнт — `@nats-io/transport-node`+`@nats-io/jetstream` (nats.js v3). Контракти з Rust
  `contracts`-крейту — `schemars` генерує JSON Schema (`contracts/schema/*.json`, тест на
  актуальність), `json-schema-to-typescript` генерує TS-типи з них + `zod`-валідація на вході
  (невалідне повідомлення → одразу DLQ з причиною, без повторів).
- Pull-consumer на `NOTIFY_CMD` (durable, `ack_wait`, `max_deliver` → DLQ, backoff).
- Трейт-подібний інтерфейс `Channel` (WhatsApp — перша реалізація; далі Signal/e-mail/SMS) —
  вибір каналу за налаштуваннями контакту.
- **Стан прив'язки WhatsApp-сесії** — власна машина станів (`starting → pairing (qr|code) →
  connected → needs_pairing → pairing …`), публікується в NATS KV bucket `notifier_status`
  (TTL 60с) — адмін-екран `/admin/whatsapp` (лише `admin`, `policy`) підписується й показує QR/
  pairing-код, що сам оновлюється, без "сирого" WhatsApp-Business API токена ніде в
  БД/JetStream-стрімах (QR/код НІКОЛИ не пишуться в логи/сховище). Сесія (`LocalAuth`) — окремий
  іменований docker-volume, повторний старт контейнера не вимагає нового сканування.
- Журнал доставок (що, кому (org_id), коли, статус, помилка) — без повного номера в логах
  (маскування, `+380*****4567`).
- Публікує `notify.result` → застосунок показує статус біля розбіжності ("сповіщено 14:02",
  "не доставлено: …").
- Мережа: вихід в інтернет — тільки цей контейнер, тільки на домени WhatsApp
  (egress-allowlist/проксі, не голий firewall-виняток на весь інтернет).
- Health: `/healthz` (живий), `/readyz` (є з'єднання з NATS і БД).

## 3.8. Адреси доставки (notification destinations)

Сповіщення з системи мають потрапляти на конкретний WhatsApp-номер. Розрізняються два режими
прав і три типи адрес:

### Ролі

| Роль | Можливості |
|---|---|
| **Адмін** | Додати бот-номер (службовий WhatsApp для організації), свій особистий номер, створити групу розсилки, підписатися на будь-які типи сповіщень |
| **Користувач** | Додати лише свій особистий номер для отримання сповіщень |

### Типи адрес (`whatsapp_destination.kind`)

| Тип | Опис | Хто створює |
|---|---|---|
| `personal` | Особистий телефон користувача | сам користувач або адмін |
| `bot` | Службовий номер бота WhatsApp (підключається через QR/pairing) | тільки адмін |

### Типи сповіщень (`notification_type`)

| Код | Назва | Область |
|---|---|---|
| `discrepancy_opened` | Розбіжність відкрита | org |
| `discrepancy_resolved` | Розбіжність закрита | org |
| `submission_committed` | Подання зафіксовано | org |
| `import_completed` | Імпорт завершено | org |
| `system` | Системне повідомлення | personal |

Кожна адреса підписується на обрані типи через `notification_subscription`.

### Групи розсилки (`notification_group`)

Адмін може створювати іменовані групи в контексті організації. Група об'єднує кілька адрес
(`notification_group_member`). Коли система генерує сповіщення типу, що прив'язаний до групи,
повідомлення надсилається кожному учаснику групи.

### Маршрутизація

Коли виникає доменна подія (04 §5 — розбіжність, фіксація подання тощо):
1. Система визначає `org_id` і `notification_type`.
2. Знаходить усі активні підписки на цей тип для цього `org_id`.
3. Для кожної підписки формує `NotifySend.v1` команду з `recipient_org_id` і `template`.
4. Нотифікатор за `org_id` знаходить відповідний `whatsapp_destination` (bot або personal)
   і доставляє знеособлене повідомлення.

Телефони зберігаються маскованими (`+380*****4567`) в таблиці застосунку; повний номер — лише в
нотифікаторі (`org_contact` у його окремій схемі, §3.7). Це узгоджується з правилом приватності §3.5.

### Таблиці (схема `app`)

```sql
whatsapp_destination (id, user_id?, org_id, kind, phone_masked, is_active, created_at)
notification_type    (code PK, name, scope)
notification_subscription (id, destination_id FK, notification_type FK, is_active)
notification_group   (id, org_id, name, created_by FK, created_at, deleted_at)
notification_group_member (group_id FK, destination_id FK, PK)
```

## 4. Структура воркспейсу (доповнення до 07)

```
contracts/          типи повідомлень + Envelope + subjects-константи; без tokio, збирається й у wasm
                    (+ contracts/schema/*.json — schemars, джерело для TS-типів notifier'а)
bus/                тонка обгортка над async-nats: publish з Msg-Id, pull-consumer з inbox-ідемпотентністю,
                    повтори/DLQ, трасування correlation_id; outbox-relay як бібліотечна задача
services/notifier/  TypeScript (Node 22 LTS), ОКРЕМИЙ Docker-образ — не Cargo workspace member
                    (своя міграція, своя конфігурація, package.json)
server/             + запуск relay і consumer результатів як фонових задач
app/src/backend/    + repo/outbox.rs (запис у outbox у транзакції доменної дії)
```

`services/notifier/` — єдиний НЕ-Rust код у цьому репозиторії: правила для нього (07
"Інші крейти") — свій `tsconfig`/lockfile, свій `Dockerfile`, власна карта `CLAUDE.md`, ізольований
від Cargo workspace (не заважає `cargo build --workspace`, не додається до жодного `Cargo.toml`).

## 5. Експлуатація
- `docker-compose.yml`: сервіси `nats` (`nats:2-alpine -js -sd /data`, volume, `mem_limit`),
  `notifier` (`mem_limit 1g`, `shm_size 512m`, `restart unless-stopped`, healthcheck); для
  розробки на Windows — той самий compose.
- `notifier` — єдиний контейнер з виходом в інтернет, і лише на домени WhatsApp (`web.whatsapp.
  com`, `*.whatsapp.net`, `*.whatsapp.com`) через egress-проксі з allowlist доменів (IP
  провайдера змінюються, голе firewall-правило на IP не працює довго) — спосіб реалізації
  фіксується в MEMORY.md разом із конкретною інфраструктурою розгортання.
- Сесія WhatsApp (`LocalAuth`) — окремий іменований том, права лише для користувача сервісу, не
  в git/бекапах (сесія = повний доступ до акаунта).
- Адмін-екран "Черги": відставання outbox (непубліковані, найстаріший), стан стрімів і
  consumer-ів (pending, redelivered), DLQ з кнопкою "повторити". Окремий адмін-екран
  `/admin/whatsapp` — стан прив'язки, QR/pairing-код, "Відв'язати"/"Надіслати тестове" (§3.7).
- Трасування: `correlation_id` проходить від дії користувача через outbox, NATS і notifier до
  результату — у логах усіх сервісів.

## 6. Тести
- Контрактні: серіалізація/десеріалізація кожного типу, зворотна сумісність версій; TS-типи
  згенеровані з актуальної `contracts/schema/*.json` (тест на розсинхрон).
- Інтеграційні з реальним `nats-server` у Docker: транзакція відкочена → повідомлення нема;
  NATS вимкнено → outbox копиться → увімкнено → усе доставлено рівно раз (inbox); повтор після
  збою провайдера; `max_deliver` → DLQ; невалідне (по zod) повідомлення → одразу DLQ без повторів;
  `needs_pairing` → команди чекають (nak із затримкою), доставляються після повторної прив'язки.
  Для `notifier` — `FakeChannel` замість реального WhatsApp-клієнта в автотестах.
- Приватність: тест контракту (3.5) + тест, що в логах notifier нема повних номерів (маскування).
- Ручний сценарій з реальним службовим номером (не автоматизується — потребує фізичного
  телефону): чистий старт → QR в адмінці → сканування → `connected` → тестове повідомлення →
  `docker compose restart notifier` без повторного сканування → відв'язка з телефону →
  `needs_pairing` + сповіщення в адмінці → повторна прив'язка. Результат і RSS контейнера
  (спокій/під розсилкою) — у MEMORY.md.
