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

## 3. Серверне забезпечення прав

Тип `AuthUser { user_id, actor: Option<Actor>, display_name }` (types/auth.rs).

Хелпери (services/auth.rs, `#[cfg(feature = "ssr")]`):
- `require_auth()` → читає cookie `session`, знаходить `user_session` (не прострочена,
  `user_account.is_active`), повертає `AuthUser` або `ServerFnError`.
- `resolve_actor(client_actor)` → якщо є валідна auth-сесія, бере актора з неї;
  інакше — фолбек на клієнтського актора (dev-режим).
- Кожна server function, що потребує актора, викликає `resolve_actor()` замість
  прямої довіри клієнтському `actor: Option<Actor>` параметру.
- Сторінки крім `/login` — перенаправлення на `/login` (в Auth-режимі, клієнтське).

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

**Étap 10a** (логін/пароль) — **реалізовано**:
- Міграції: `user_account`, `user_role`, `user_session` (046-047).
- Сторінка `/login` (argon2id, cookie HttpOnly/SameSite=Strict).
- `require_auth()`/`resolve_actor()` — серверне забезпечення прав.
- Rate limiting: 5 спроб / 15 хв на IP (in-memory).
- Seed admin-акаунт (`admin` / `admin123`).
- Перемикач актора: AuthSwitcher (аватар + меню ролей + logout) / DevSwitcher
  (два дропдауни) + ModeToggle DEV/AUTH.

**Étap 10b** (FIDO2) — **інфраструктура готова, WebAuthn протокол — TODO**:
- Міграція 048: `passkey_credential`.
- Repo `passkeys.rs`: CRUD (store/find/update_sign_count/delete).
- Вкладка "Безпека" в `/settings`: список ключів + видалення.
- **TODO**: `webauthn-rs` інтеграція (challenge/verify), кнопка реєстрації,
  автентифікація через passkey на `/login`.
