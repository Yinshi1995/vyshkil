---
tags: [spec, infra, ansible, proxmox, production, security]
date: 2026-10-05
status: АКТИВНО — замовник дав команду починати прод (2026-10-05)
---

# 11. Прод-деплой на Proxmox (Ansible)

## 0. Мета

Промисловий деплой того самого стеку, що в `docker-compose.yml` (db, nats, app, notifier), на VM у
Proxmox замовника — з **технічно гарантованими** правилами репозиторію, а не "в коді так написано":
- назовні з доменними даними — **тільки** знеособлене WhatsApp-повідомлення через notifier;
- жодних ПІБ у системних логах/моніторингу;
- `source_files/` (ДСК) на проді **відсутні** (архів уже перенесено в БД; прод їх не потребує);
- секрети — тільки через Ansible Vault (dev-compose plaintext-паролі в прод не копіюються).

## 0.1. Обмеження хоста

Фізичний хост (`pidhotovka`, Ryzen 5 5600GT, 14.9 ГБ RAM). Dev-VM забрала 8 ГБ. Прод-VM
потребує значно менше (~2–4 ГБ): на проді не компілюється Rust, не ставляться dev-інструменти —
лише Docker-контейнери з готовими образами. Рантайм-стек (db+nats+app+notifier) споживає
~220 МБ RSS (виміряно на dev-VM). З урахуванням буферів/кешу ОС 2 ГБ RAM + 2 ГБ swap —
достатній мінімум. 4 ГБ — комфортно.

**Якщо обидві VM одночасно**: dev-VM (8 ГБ) + prod-VM (2 ГБ) = 10 ГБ з 14.9 ГБ фізичних —
вміщається, з резервом ~5 ГБ для хоста. Dev-VM можна тимчасово зменшити до 6 ГБ, якщо потрібно
більше запасу.

## 0.2. Зовнішній доступ — Cloudflare tunnel

> **Рішення змінено 2026-10-05** (`.claude/decisions/prod-cloudflare-tunnel.md`):
> замовник вирішив публікувати через Cloudflare tunnel для зручного доступу з польових пристроїв.
> Попереднє обмеження "НІКОЛИ через Cloudflare tunnel" скасовано.

На хості є `cf-connector` (LXC 150) з активним Cloudflare tunnel. Для прод-VM використовується
**окремий `cloudflared` daemon** безпосередньо на прод-VM:
- проксює `localhost:3000` (застосунок) на зовнішній домен через Cloudflare;
- не залежить від `cf-connector` LXC (ізоляція, окремий токен);
- Cloudflare Access policy для обмеження доступу (email/OTP або інший IdP);
- tunnel token зберігається в Ansible Vault.

## 0.3. Мережа — VyOS сегментація

Прод-VM **не** на тій самій LAN (vmbr0), що й dev-VM. Маршрутизацією прод-VM займається VyOS:
- прод-VM підключена до VyOS-керованого мережевого сегменту (окремий bridge або VLAN);
- VyOS забезпечує маршрутизацію, firewall між сегментами, NAT для вихідного трафіку;
- параметри (bridge, VLAN, IP-діапазон) — в `inventories/prod/group_vars/all.yml`.

## 1. Ролі (перевикористання з dev)

| Роль | Dev | Prod | Різниця на проді |
|---|---|---|---|
| `provision` | ✓ | ✓ | Інший VMID, менше ресурсів, VyOS-мережа |
| `base` | ✓ | ✓ | `base_user_passwordless_sudo: false` |
| `docker` | ✓ | ✓ | `docker_add_primary_user_to_group: false` |
| `firewall` | ✓ | ✓ | SSH + порти для cloudflared (якщо потрібно) |
| `app_stack` | — | ✓ | **Нова**: compose-стек, env-файли з Vault, volumes |
| `cloudflare_tunnel` | — | ✓ | **Нова**: cloudflared daemon, tunnel config |
| `rust_toolchain` | ✓ | — | Не потрібна: образи збираються на dev |
| `node` | ✓ | — | Не потрібна: notifier всередині Docker |
| `devtools` | ✓ | — | |
| `claude_code` | ✓ | — | |
| `project` | ✓ | — | Замінена на `app_stack` |
| `autonomy` | ✓ | — | |

## 2. Секрети

Ansible Vault (`inventories/prod/group_vars/vault.yml`):
- `vault_postgres_app_password` — пароль app→Postgres
- `vault_postgres_notifier_password` — пароль notifier→Postgres
- `vault_nats_app_password` — пароль app→NATS
- `vault_nats_notifier_password` — пароль notifier→NATS
- `vault_cloudflare_tunnel_token` — токен tunnel
- `vault_prod_vm_user_ssh_public_key` — SSH-ключ для доступу

## 3. Мережа

### 3.1. Вхідні (nftables на гостьовій VM)
SSH (22) — тільки з адмін-підмережі. Усе інше — drop.
NATS 4222 — не публікується; 8222 — тільки 127.0.0.1; Postgres — не публікується.
Порт 3000 — тільки 127.0.0.1 (cloudflared проксює локально).

### 3.2. Docker і хостовий файрвол
`DOCKER-USER` ланцюжок для обмеження контейнерних з'єднань. Compose-файл: усі порти на
`127.0.0.1`, не на `0.0.0.0`.

### 3.3. Egress allowlist (WhatsApp)
Notifier на проді — через egress-мережу з обмеженням доменів (squid/tinyproxy з allowlist).
Список доменів: `web.whatsapp.com`, `*.whatsapp.net`, `*.whatsapp.com`, CDN-домени WhatsApp
(фактичний список зібрати на dev-VM через proxy-логування перед прод-деплоєм).

### 3.4. Docker-мережі: топологія ізоляції

Три мережі (як у dev `docker-compose.yml`, так і в прод-шаблоні):

| Мережа | Тип | Сервіси | Доступ до інтернету |
|---|---|---|---|
| `internal` | `internal: true` | db, nats, app, notifier | **НІ** (нема gateway) |
| `publish` | bridge, `enable_ip_masquerade: false` | db, nats, app | **НІ** (нема NAT); тільки для публікації портів на 127.0.0.1 |
| `egress` | bridge (звичайний) | notifier | **ТАК** — єдина мережа з виходом назовні |

Ключові наслідки:
- **db** і **nats** не мають жодного маршруту в інтернет — навіть якщо зловмисник потрапить у
  контейнер, DNS/TCP-з'єднання назовні не пройдуть;
- **app** (сервер) теж не має інтернету — XSS/SSRF з контейнера не вийде за периметр;
- **notifier** — єдиний з виходом, обмежений egress allowlist (§3.3) до доменів WhatsApp.

### 3.5. Захист від прямого зовнішнього доступу (DB, NATS, WhatsApp)

Завдання: жоден зовнішній актор не може дістатися до бази, брокера або WhatsApp-сесії напряму.

**Рівень 1 — nftables на VM (§3.1):**
- INPUT policy DROP; дозволено: SSH (22) тільки з адмін-підмережі, loopback, established/related.
- Усі порти Docker-сервісів (5432, 4222, 8222, 3000) прив'язані до `127.0.0.1` → ззовні
  недоступні навіть без файрволу.

**Рівень 2 — `DOCKER-USER` ланцюжок (§3.2):**
- nftables/iptables правила в `DOCKER-USER`: DROP вхідних з'єднань до контейнерних портів,
  які прийшли НЕ з `lo`. Страховка від випадкової зміни bind-адреси в compose.

**Рівень 3 — автентифікація сервісів:**
- **Postgres**: `pg_hba.conf` — `md5`/`scram-sha-256` для всіх підключень; `listen_addresses = '*'`
  тільки всередині Docker-мережі (порт не видно ззовні). Два окремих користувача з мінімальними
  правами (§9).
- **NATS**: анонімний доступ заборонено; авторизація per-user з мінімальними правами (§8).
  Monitoring (8222) — тільки `127.0.0.1`, read-only.
- **WhatsApp-сесія**: том `taktoblik_wa_session` доступний лише контейнеру notifier;
  Chromium sandbox увімкнений (SYS_ADMIN cap, не `--privileged`).

**Рівень 4 — Cloudflare tunnel:**
- Tunnel проксює **тільки** `localhost:3000` (застосунок); жоден інший порт/сервіс не
  маршрутизується через tunnel.
- Cloudflare Access policy: email/OTP автентифікація перед доступом.

**Рівень 5 — VyOS (§0.3):**
- Прод-VM в окремому мережевому сегменті; VyOS firewall між сегментами.

Ansible-роль `firewall` має перевіряти всі 5 рівнів при кожному прогоні (ідемпотентно).

## 4. Дані, томи, бекапи
| Том | Що це | Бекап |
|---|---|---|
| Postgres data | основні дані (рівень ДСК) | `pg_dump`, **шифровано**, у периметрі |
| `wa-session` | жива прив'язка WhatsApp | рішення замовника |
| NATS data | черги (outbox у БД — джерело правди) | не обов'язково |
| `data/` застосунку | збережені файли подань | разом із БД |
| `source_files/` | ДСК-архів | **на проді нема** |

## 5. Процес деплою

1. На dev-VM: `docker compose build` → `docker save` образів → transfer на prod-VM
2. На prod-VM: `docker load` → Ansible `app_stack` роль оновлює compose
3. Бекап БД перед міграціями
4. `docker compose up -d` — міграції застосовуються на старті сервера
5. Healthchecks → готово

## 6. Логи й моніторинг
- Docker `log-driver: local`, ротація.
- `WA_PRINT_QR_TO_LOGS` — вимкнено.
- Перевірити, що жоден сервіс не пише ПІБ/номери телефонів.

## 7. Файли (Ansible)

```
infra/ansible/
├── inventories/prod/
│   ├── hosts.yml
│   └── group_vars/
│       ├── all.yml           # параметри прод-VM
│       └── vault.yml.example  # шаблон секретів
├── playbooks/
│   ├── provision-prod-vm.yml  # створює VM
│   └── prod-vm.yml            # конфігурує VM + стек
└── roles/
    ├── app_stack/             # compose-стек, env, volumes, nats-server.conf
    └── cloudflare_tunnel/     # cloudflared daemon
```

## 8. NATS — авторизація, JetStream, стріми

### 8.1. Конфіг-файл (`nats-server.conf`)

Шаблонізується Ansible (`roles/app_stack/templates/nats-server.conf.j2`), паролі з Vault.
Dev-значення (`app-dev-password`, `notifier-dev-password`) **не копіюються** на прод.

```
jetstream { store_dir: /data }
authorization {
  users: [
    { user: app,      password: {{ vault_nats_app_password }},      permissions: { ... } }
    { user: notifier,  password: {{ vault_nats_notifier_password }}, permissions: { ... } }
  ]
}
```

Анонімний доступ — заборонено (немає блоку `no_auth_user`).

### 8.2. Матриця прав (subject-и)

| Користувач | publish | subscribe |
|---|---|---|
| `app` | `vyshkil.discrepancy.>`, `vyshkil.notify.send.>`, `vyshkil.notifier.whatsapp.>`, `$JS.API.>` | `vyshkil.notify.result.>`, `_INBOX.>`, `$JS.API.>`, `$KV.>` |
| `notifier` | `vyshkil.notify.result.>`, `vyshkil.dlq.notify.>`, `$JS.API.>`, `$KV.>`, `_INBOX.>` | `vyshkil.notify.send.>`, `vyshkil.notifier.whatsapp.>`, `_INBOX.>`, `$JS.API.>`, `$KV.>` |

`$JS.API.>` — JetStream management (стріми/консюмери). `$KV.>` — KV-bucket `notifier_status`
(стан прив'язки WhatsApp). `_INBOX.>` — request-reply.

### 8.3. JetStream: стріми та консюмери

Створюються програмно на старті (ідемпотентно, get-or-create):

| Стрім | Subject | Хто створює | Опис |
|---|---|---|---|
| `NOTIFY_CMD` | `vyshkil.notify.send.v1` | notifier (`nats.ts`) | Команди сповіщень: relay → notifier |
| `NOTIFY_RESULT` | `vyshkil.notify.result.v1` | notifier | Результати доставки: notifier → app |
| `DLQ` | `vyshkil.dlq.>` | notifier | Dead-letter queue: невалідні / вичерпані повідомлення |

**Durable consumer:** `notifier-whatsapp` на `NOTIFY_CMD`:
- `ack_policy: explicit` — повідомлення підтверджується (ack) тільки при успішній доставці
- `ack_wait: 30s` — timeout на ack
- `max_deliver: 20` — після 20 невдалих спроб → DLQ + term
- `filter_subject: vyshkil.notify.send.v1`

**KV bucket:** `notifier_status` (TTL 60s, history 1) — стан прив'язки WhatsApp для SSE в UI.

### 8.4. Гарантія доставки: at-least-once

Повідомлення зникає з черги (`ack`) **тільки** коли доставку підтверджено:

| Результат доставки | Дія | Повідомлення |
|---|---|---|
| `delivered` (усі контакти) | `markProcessed` + `ack` | Видалено з черги, записано в inbox |
| `suppressed` (нема контактів) | `markProcessed` + `ack` | Не помилка каналу — нікому слати |
| `failed` (група не знайдена, мережа) | `nak(30s)` | Повертається в чергу, повторна спроба |
| `failed` після `max_deliver` спроб | `DLQ` + `term` | В dead-letter queue, не губиться мовчки |
| `invalid` (невалідний zod) | `DLQ` + `term` | Не ретрається (структурна помилка) |
| `ChannelNotReadyError` (не прив'язаний) | `nak(30s)` | Чекає прив'язки WhatsApp |
| Інший exception | `nak(5s)` / `DLQ` на останній | Transient помилка |

Inbox-дедуплікація (`inbox` таблиця): запис додається тільки при успішній доставці, тому
повторні спроби (після nak) не блокуються як "дублікат".

### 8.5. Перенесення NATS-стану з dev на прод

NATS-стріми та консюмери **не потрібно переносити** — вони створюються програмно при старті
(`ensureStream` в `nats.ts`, `bindNotifyCmdConsumer`). Достатньо:
1. Скопіювати `nats-server.conf` (Ansible шаблонізує з prod-паролями).
2. Запустити стек — стріми/консюмери/KV-bucket створяться автоматично.
3. Перевірити: `nats stream ls`, `nats consumer ls NOTIFY_CMD`, `nats kv ls` (через
   SSH-тунель на 4222, CLI `nats` з адмін-підмережі).

## 9. Postgres — ізоляція доступу

### 9.1. Бази та користувачі

| База | Користувач | Пароль (Vault) | Призначення |
|---|---|---|---|
| `taktoblik` | app-user | `vault_postgres_app_password` | Застосунок (SeaORM) |
| `notifier` | notifier | `vault_postgres_notifier_password` | Сервіс сповіщень |

Створюються `docker/init-notifier-db.sql` при першій ініціалізації тому.
Прод-паролі з Vault → `db.env` (Ansible `app_stack` роль).

### 9.2. Мережева ізоляція

- `listen_addresses = '*'` всередині контейнера — але контейнер підключений лише до `internal`
  та `publish` мережі (§3.4), жодна з яких не має маршруту в інтернет.
- Порт `5432` прив'язаний до `127.0.0.1` — ззовні VM недоступний.
- `DOCKER-USER` chain (§3.5, рівень 2) — страховка від зміни bind-адреси.
- `pg_hba.conf`: стандартний Alpine-образ дозволяє підключення `md5` тільки з Docker-підмереж.

### 9.3. Мінімальні права

Користувач `notifier` НЕ має доступу до бази `taktoblik` (і навпаки). Якщо зловмисник
скомпрометує notifier-контейнер — він побачить лише `org_contact`, `delivery_log`, `inbox`,
`_migrations` (3 таблиці нотифікатора), але не основні дані застосунку.

## 10. Чеклист для іншого агента (розгортання прод-VM)

Послідовність дій:

1. **Ansible inventory** — `inventories/prod/group_vars/vault.yml` з усіма секретами (§2).
2. **Provision** — `playbooks/provision-prod-vm.yml` (VM, VyOS-мережа, диски).
3. **Base + Docker + Firewall** — ролі з dev, параметри prod (`group_vars/all.yml`).
4. **nats-server.conf** — Ansible шаблонізує з prod-паролями; анонімний доступ заборонено.
5. **Postgres init** — `init-notifier-db.sql`, prod-паролі в `db.env`.
6. **Docker images** — `docker save` з dev → `docker load` на prod.
7. **`docker compose up -d`** — стріми/консюмери/KV створяться автоматично.
8. **nftables** — перевірити всі рівні §3.5.
9. **Cloudflare tunnel** — `cloudflared` daemon, тільки порт 3000.
10. **Smoke test** — ззовні перевірити, що 5432/4222/8222 **не** відповідають; з адмін-підмережі
    по SSH: `nats stream ls`, `psql`, `docker compose logs`.
11. **WhatsApp pairing** — через admin UI, QR/pairing code → перша тестова нотифікація.
