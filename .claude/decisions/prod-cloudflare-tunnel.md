---
date: 2026-10-05
status: accepted
supersedes: "docs/spec/11-prod-deploy.md §0.2 (old: НІКОЛИ через Cloudflare tunnel)"
---

# Прод-доступ через Cloudflare tunnel

## Контекст

`docs/spec/11-prod-deploy.md` §0.2 забороняв публікацію через Cloudflare tunnel ("Цей застосунок
через цей (чи будь-який інший) Cloudflare tunnel не публікується ніколи").

## Рішення

Замовник змінив вимогу (2026-10-05): зовнішній доступ до прод-VM через Cloudflare tunnel
дозволено. Мотивація — зручний доступ з польових пристроїв без необхідності VPN.

## Реалізація

- `cloudflared` daemon безпосередньо на прод-VM (не через існуючий `cf-connector` LXC 150)
- Окремий tunnel token (ізольований від інших сервісів хоста)
- Cloudflare Access policy для обмеження доступу
- Проксює лише `localhost:3000` (web-застосунок)
- Token зберігається в Ansible Vault

## Що НЕ змінилось

- Жодних даних назовні, крім знеособлених WhatsApp-повідомлень через notifier
- Жодних ПІБ у логах/моніторингу
- `source_files/` на проді відсутні
- Секрети — лише через Ansible Vault
