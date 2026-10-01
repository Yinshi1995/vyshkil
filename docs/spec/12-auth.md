---
tags: [spec, auth]
date: 2026-10-01
status: чернетка (потребує підтвердження замовника)
---

# 12. Автентифікація і акаунти

Замінює dev-режим "перемикач актора в шапці" (01 §6) реальним модулем автентифікації.

## 1. Модель

### 1.1 Таблиця `user_account`

| Колонка | Тип | Опис |
|---|---|---|
| `id` | `serial PK` | — |
| `login` | `text UNIQUE NOT NULL` | унікальний логін |
| `password_hash` | `text NOT NULL` | argon2id хеш |
| `display_name` | `text` | ім'я для UI (не ПІБ — CLAUDE.md: "людей поіменно не зберігати") |
| `is_active` | `bool DEFAULT true` | деактивація без видалення |
| `created_at` | `timestamptz DEFAULT now()` | — |
| `updated_at` | `timestamptz DEFAULT now()` | — |

### 1.2 Таблиця `user_role`

Зв'язок "акаунт має роль в організації" (один акаунт може мати кілька ролей для різних org).

| Колонка | Тип | Опис |
|---|---|---|
| `id` | `serial PK` | — |
| `user_id` | `int REFERENCES user_account(id)` | — |
| `org_id` | `int REFERENCES org(id)` | організація |
| `role` | `text CHECK (role IN ('admin','org_editor','viewer'))` | роль (той самий перелік, що `types::actor::Role`) |
| `UNIQUE` | `(user_id, org_id, role)` | — |

### 1.3 Таблиця `passkey_credential` (FIDO2/WebAuthn)

| Колонка | Тип | Опис |
|---|---|---|
| `id` | `serial PK` | — |
| `user_id` | `int REFERENCES user_account(id)` | — |
| `credential_id` | `bytea UNIQUE NOT NULL` | WebAuthn credential ID |
| `public_key` | `bytea NOT NULL` | DER-encoded public key |
| `sign_count` | `bigint DEFAULT 0` | replay protection |
| `name` | `text` | мітка ключа ("YubiKey робочий") |
| `created_at` | `timestamptz DEFAULT now()` | — |

### 1.4 Таблиця `user_session`

| Колонка | Тип | Опис |
|---|---|---|
| `id` | `uuid PK DEFAULT gen_random_uuid()` | session token (зберігається в cookie) |
| `user_id` | `int REFERENCES user_account(id)` | — |
| `active_org_id` | `int REFERENCES org(id)` | поточна обрана організація |
| `active_role` | `text` | поточна обрана роль |
| `created_at` | `timestamptz DEFAULT now()` | — |
| `expires_at` | `timestamptz NOT NULL` | TTL (default 30 днів) |

## 2. Потоки

### 2.1 Логін/пароль

1. `POST /api/auth/login` — `{ login, password }`.
2. Сервер: знайти `user_account` за `login`, верифікувати `password_hash` (argon2id).
3. Створити `user_session`, повернути `Set-Cookie: session=<uuid>; HttpOnly; SameSite=Strict; Secure`.
4. Перенаправити на `/`.

### 2.2 FIDO2/Passkey

1. **Реєстрація** (`/settings`, вкладка "Безпека"):
   - `POST /api/auth/passkey/register/begin` → `PublicKeyCredentialCreationOptions` (challenge).
   - Клієнт: `navigator.credentials.create()`.
   - `POST /api/auth/passkey/register/finish` → зберігає `passkey_credential`.

2. **Автентифікація**:
   - `POST /api/auth/passkey/login/begin` → `PublicKeyCredentialRequestOptions`.
   - Клієнт: `navigator.credentials.get()`.
   - `POST /api/auth/passkey/login/finish` → session cookie.

### 2.3 Перемикання актора

Після логіну в шапці замість вільного перемикача (dev-режим) — список доступних
`user_role` для цього `user_id`. `Actor { org_id, role }` залишається тим самим типом,
але тепер зберігається в `user_session.active_org_id`/`active_role`.

### 2.4 Логаут

`POST /api/auth/logout` — видаляє `user_session`, клієнт чистить cookie. Перенаправити на `/login`.

## 3. Middleware (Axum)

Extractor `AuthUser` (Axum `FromRequestParts`):
- Читає cookie `session`.
- Знаходить `user_session` (не прострочена, `user_account.is_active`).
- Повертає `AuthUser { user_id, actor: Actor { org_id, role } }`.
- Сторінки крім `/login` — `AuthUser` обов'язковий (redirect на `/login`).

## 4. Бібліотеки

| Бібліотека | Призначення |
|---|---|
| `argon2` | хешування паролів (argon2id) |
| `webauthn-rs` | FIDO2/WebAuthn сервер |
| `tower-cookies` або Axum built-in cookies | cookie management |

Усі бібліотеки — Rust, без зовнішніх мережевих залежностей (CLAUDE.md: жодних викликів назовні).

## 5. Міграція з dev-режиму

- `ActorSwitcher` залишається для `cfg(debug_assertions)` або env-змінної `DEV_MODE=1`.
- У прод-режимі `ActorSwitcher` ховається, `Actor` беремо з `AuthUser`.
- Seed-міграція створює початкового admin-акаунта (логін `admin`, пароль — env-змінна
  `INITIAL_ADMIN_PASSWORD`, без дефолтного значення — обов'язково задати при першому запуску).

## 6. Безпека

- Паролі — argon2id (не bcrypt, не SHA).
- Session UUID — `gen_random_uuid()`, не передбачуваний.
- Cookie: `HttpOnly; SameSite=Strict; Secure` (в dev без Secure).
- Брутфорс: rate limiting на `/api/auth/login` (Axum middleware, IP-based, 5 спроб / 15 хв).
- Passkey: challenge — одноразовий, TTL 5 хв, зберігається в пам'яті сервера (не в БД).
- Людей поіменно не зберігати — `display_name` може бути позивний або роль, не ПІБ.

## 7. Етапність

**Етап 10a** (логін/пароль):
- Міграції: `user_account`, `user_role`, `user_session`.
- Сторінка `/login`.
- Axum middleware `AuthUser`.
- Seed admin-акаунт.
- Перемикач актора: список з `user_role` замість вільного.

**Етап 10b** (FIDO2):
- Міграція: `passkey_credential`.
- Реєстрація ключа в `/settings` (нова вкладка "Безпека").
- Автентифікація через passkey.
