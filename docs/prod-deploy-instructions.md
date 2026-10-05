# Інструкція: розгортання прод-VM на Proxmox

> Для іншого агента / оператора. Покрокова інструкція — від створення VM до працюючого
> застосунку за Cloudflare tunnel.

## ⚠ Головне правило: питай у користувача

**Не генеруй паролі, токени та мережеві параметри самостійно.** Користувач сам надасть їх.
На кожному кроці, де потрібні credentials або параметри мережі — **запитай у користувача**
і чекай відповіді. Він скине потрібне.

Що питати (по ходу роботи, не все одразу):
1. **Мережа VyOS**: bridge, VLAN, IP прод-VM, gateway — "Який bridge/VLAN/IP для прод-сегменту?"
2. **PVE API**: пароль root@pam або токен — "Дай пароль/токен для Proxmox API"
3. **Cloudflare tunnel token** — "Дай tunnel token з Cloudflare dashboard"
4. **Паролі Postgres** (app, notifier) — "Дай паролі для Postgres (app і notifier)"
5. **Паролі NATS** (app, notifier) — "Дай паролі для NATS (app і notifier)"
6. **SSH-ключ** для прод-VM — "Дай публічний SSH-ключ для прод-VM" (або запитай, чи згенерувати)
7. **Домен Cloudflare** — "На якому домені/субдомені публікувати?"

Не рухайся далі без відповіді на критичне питання. Паролі/токени — одразу в ansible-vault,
ніколи в git, ніколи в лог, ніколи в відповідь.

## Передумови

| Що потрібно | Де взяти |
|---|---|
| SSH-доступ до control node (WSL2/Arch на Windows-машині розробника) | Фізичний доступ |
| Ansible 2.21+ на control node | `pacman -Syu ansible-core` (Arch) |
| Ansible-колекції | `cd infra/ansible && ansible-galaxy collection install -r requirements.yml` |
| Proxmox API доступ (`root@pam` або токен `ansible@pve!provision`) | PVE UI |
| Мережеві параметри VyOS-сегменту для прод (bridge, VLAN, IP-діапазон, gateway) | У замовника / VyOS-адміна |
| Cloudflare tunnel token | Cloudflare Zero Trust Dashboard |
| SSH-ключ для прод-VM (окремий від dev!) | Згенерувати: `ssh-keygen -t ed25519 -f ~/.ssh/vyshkil-prod -C prod-deploy-key` |

## Архітектура

```
┌─────────────────────────────────────────────────────────┐
│  Proxmox (pidhotovka)                                   │
│                                                         │
│  ┌───────────────┐    ┌──────────────────────────────┐  │
│  │ Dev-VM (300)  │    │ Prod-VM (400)                │  │
│  │ vmbr0 / LAN   │    │ VyOS-сегмент                 │  │
│  │ 192.168.1.200 │    │ IP: <з VyOS-конфіга>         │  │
│  │               │    │                              │  │
│  │ (збірка       │    │ ┌─────┐ ┌────┐ ┌─────────┐  │  │
│  │  образів)     │───>│ │ app │ │ db │ │notifier │  │  │
│  │               │    │ └──┬──┘ └────┘ └─────────┘  │  │
│  └───────────────┘    │    │                         │  │
│                       │ cloudflared ──> Cloudflare   │  │
│  ┌───────────────┐    │    tunnel                    │  │
│  │ VyOS          │    └──────────────────────────────┘  │
│  │ (маршрутизація│                                      │
│  │  прод-VM)     │                                      │
│  └───────────────┘                                      │
└─────────────────────────────────────────────────────────┘
```

Docker-образи збираються на dev-VM, передаються на прод-VM через `docker save/load`.
Cloudflared на прод-VM проксює `localhost:3000` назовні через Cloudflare tunnel.

## Крок 0. Підготовка мережі (VyOS)

> Цей крок виконується на VyOS-роутері. Якщо ти не маєш доступу до VyOS — запиши параметри
> в `docs/QUESTIONS.md` і попроси замовника/мережевого адміна.

Прод-VM має бути в окремому мережевому сегменті, яким керує VyOS. Потрібно:

1. **З'ясувати параметри** VyOS-сегменту для прод-VM:
   - Який bridge на Proxmox використовує VyOS для прод-сегменту (напр. `vmbr1`, `vmbr2`, або
     VLAN tag на існуючому bridge)
   - IP-діапазон прод-сегменту (напр. `10.0.1.0/24`)
   - Gateway (IP VyOS-інтерфейсу в цьому сегменті, напр. `10.0.1.1`)
   - Чи VyOS забезпечує NAT для вихідного трафіку з прод-сегменту (потрібно для Cloudflare
     tunnel і WhatsApp notifier)

2. **На VyOS** (якщо потрібно створити новий сегмент):
   ```
   configure
   # Інтерфейс до прод-сегменту (приклад — адаптувати під реальну топологію)
   set interfaces ethernet eth1 address '10.0.1.1/24'
   set interfaces ethernet eth1 description 'Prod segment'

   # NAT для вихідного трафіку
   set nat source rule 100 outbound-interface name 'eth0'
   set nat source rule 100 source address '10.0.1.0/24'
   set nat source rule 100 translation address 'masquerade'

   # Firewall: дозволити тільки необхідне
   set firewall ipv4 name PROD-OUT default-action 'drop'
   set firewall ipv4 name PROD-OUT rule 10 action 'accept'
   set firewall ipv4 name PROD-OUT rule 10 state 'established'
   set firewall ipv4 name PROD-OUT rule 10 state 'related'
   set firewall ipv4 name PROD-OUT rule 20 action 'accept'
   set firewall ipv4 name PROD-OUT rule 20 protocol 'tcp'
   set firewall ipv4 name PROD-OUT rule 20 destination port '443'
   set firewall ipv4 name PROD-OUT rule 20 description 'HTTPS out (Cloudflare + WhatsApp)'
   set firewall ipv4 name PROD-OUT rule 30 action 'accept'
   set firewall ipv4 name PROD-OUT rule 30 protocol 'tcp'
   set firewall ipv4 name PROD-OUT rule 30 destination port '53'
   set firewall ipv4 name PROD-OUT rule 31 action 'accept'
   set firewall ipv4 name PROD-OUT rule 31 protocol 'udp'
   set firewall ipv4 name PROD-OUT rule 31 destination port '53'
   set firewall ipv4 name PROD-OUT rule 31 description 'DNS'
   set firewall ipv4 name PROD-OUT rule 40 action 'accept'
   set firewall ipv4 name PROD-OUT rule 40 protocol 'tcp'
   set firewall ipv4 name PROD-OUT rule 40 destination port '7844'
   set firewall ipv4 name PROD-OUT rule 40 description 'Cloudflare tunnel QUIC/HTTP2'

   set interfaces ethernet eth1 firewall out name 'PROD-OUT'

   # Дозволити SSH з адмін-мережі в прод-сегмент
   set firewall ipv4 name ADMIN-TO-PROD default-action 'drop'
   set firewall ipv4 name ADMIN-TO-PROD rule 10 action 'accept'
   set firewall ipv4 name ADMIN-TO-PROD rule 10 protocol 'tcp'
   set firewall ipv4 name ADMIN-TO-PROD rule 10 destination port '22'

   commit
   save
   ```

   > **УВАГА**: Це приклад! Конкретні інтерфейси, IP-адреси, правила — залежать від реальної
   > топології мережі замовника. Адаптувати під дійсну конфігурацію VyOS.

3. **Записати отримані параметри** для наступного кроку:
   - `prod_vm_bridge`: ________ (bridge на Proxmox, напр. `vmbr1`)
   - `prod_vm_vlan_tag`: ________ (якщо використовується VLAN, інакше `null`)
   - `prod_vm_ip_cidr`: ________ (напр. `10.0.1.10/24`)
   - `prod_vm_gateway`: ________ (напр. `10.0.1.1`)

## Крок 1. Cloudflare tunnel

1. Зайти в [Cloudflare Zero Trust Dashboard](https://one.dash.cloudflare.com/) →
   **Networks** → **Tunnels** → **Create a tunnel**.

2. Вибрати **Cloudflared** connector type.

3. Назвати tunnel (напр. `vyshkil-prod`).

4. **Скопіювати tunnel token** — це довгий рядок, який потрібен для `vault.yml`.

5. Налаштувати Public Hostname:
   - **Subdomain**: обрати (напр. `taktoblik`)
   - **Domain**: обрати з наявних доменів у Cloudflare
   - **Service**: `http://localhost:3000`

6. (Рекомендовано) Налаштувати Cloudflare Access policy:
   - **Applications** → **Add an application** → **Self-hosted**
   - Прив'язати до того ж домену
   - Policy: дозволити конкретні email-адреси (OTP) або інший IdP

## Крок 2. Ansible vault

**Запитай у користувача по черзі** (він сам надасть значення):

1. "Дай пароль/токен для Proxmox API (root@pam або ansible@pve!provision)"
2. "Дай або згенеруй SSH-ключ для прод-VM (публічну частину)"
3. "Дай пароль Postgres для app"
4. "Дай пароль Postgres для notifier"
5. "Дай пароль NATS для app"
6. "Дай пароль NATS для notifier"
7. "Дай Cloudflare tunnel token" (з кроку 1)

З control node (WSL2):

```bash
cd /path/to/vyshkil/infra/ansible

# Створити vault-файл (пароль vault — теж запитати у користувача)
ansible-vault create inventories/prod/group_vars/vault.yml
```

Вміст vault.yml (заповнити значеннями від користувача):
```yaml
vault_pve_api_token_id: "ansible@pve!provision"   # або root@pam — запитати
vault_pve_api_token_secret: "..."                   # від користувача

vault_prod_vm_user_ssh_public_key: "ssh-ed25519 AAAA..."  # від користувача

vault_postgres_app_password: "..."         # від користувача
vault_postgres_notifier_password: "..."    # від користувача
vault_nats_app_password: "..."             # від користувача
vault_nats_notifier_password: "..."        # від користувача

vault_cloudflare_tunnel_token: "..."       # від користувача
```

> **Ніколи не вставляй паролі/токени в git, лог чи відповідь.** Тільки в зашифрований vault.

## Крок 3. Заповнити параметри мережі

**Запитай у користувача:**
- "Який Proxmox bridge для прод-сегменту VyOS?" (напр. `vmbr1`)
- "Який VLAN tag?" (або `null` якщо без VLAN)
- "Яку IP-адресу дати прод-VM?" (напр. `10.0.1.10/24`)
- "Який gateway VyOS для цього сегменту?" (напр. `10.0.1.1`)

Відредагувати `infra/ansible/inventories/prod/group_vars/all.yml` — замінити `ЗАПОВНИТИ`
отриманими значеннями:

```yaml
prod_vm_bridge: "..."              # ← від користувача
prod_vm_ip_cidr: ".../.."         # ← від користувача
prod_vm_gateway: "..."             # ← від користувача
```

Також `inventories/prod/hosts.yml` — `ansible_host` отримає IP автоматично з `prod_vm_ip`.

## Крок 4. Створити VM

```bash
cd /path/to/vyshkil/infra/ansible

# Provision: створити шаблон (якщо ще нема) + клонувати прод-VM
ansible-playbook playbooks/provision-prod-vm.yml \
  -i inventories/prod/hosts.yml \
  -e pve_api_password='ПАРОЛЬ_ROOT_PVE' \
  --ask-vault-pass
```

> **Примітка**: provision використовує ту саму роль `provision`, що й dev-VM, але з прод-параметрами
> (менше RAM/CPU, інший bridge). Шаблон 9000 створюється один раз — якщо вже є від dev, буде
> пропущений.

Після створення — перевірити в PVE UI, що VM 400 запустилась і отримала IP.

## Крок 5. Конфігурувати VM

```bash
# Перевірити SSH-доступ
ssh -i ~/.ssh/vyshkil-prod deploy@<PROD_VM_IP>

# Конфігурація: base + docker + firewall + app_stack + cloudflare_tunnel
ansible-playbook playbooks/prod-vm.yml \
  -i inventories/prod/hosts.yml \
  --ask-vault-pass
```

Що відбудеться:
1. **base**: користувач `deploy`, SSH, swap, timezone, **без passwordless sudo**
2. **docker**: Docker Engine + compose plugin, **deploy НЕ в групі docker**
3. **firewall**: nftables — тільки SSH (22)
4. **app_stack**: docker-compose.yml, env-файли з vault-секретами, NATS config
5. **cloudflare_tunnel**: cloudflared daemon з tunnel token

## Крок 6. Збірка і передача Docker-образів

На dev-VM (192.168.1.200):

```bash
# Зібрати образи
cd ~/vyshkil
docker compose build

# Зберегти образи у файл
docker save vyshkil-app:latest vyshkil-notifier:latest | gzip > /tmp/vyshkil-images.tar.gz

# Дізнатися розмір
ls -lh /tmp/vyshkil-images.tar.gz
```

Передати на прод-VM (з control node або dev-VM, залежно від мережевої доступності):

```bash
# Якщо dev-VM бачить прод-VM напряму (через VyOS-маршрутизацію):
rsync -av -e 'ssh -i ~/.ssh/vyshkil-prod' \
  /tmp/vyshkil-images.tar.gz deploy@<PROD_VM_IP>:/opt/vyshkil/images/

# Якщо не бачить — передати через control node як проміжний хоп:
# 1. З dev-VM на control node
rsync -av /tmp/vyshkil-images.tar.gz user@control-node:/tmp/
# 2. З control node на прод-VM
rsync -av -e 'ssh -i ~/.ssh/vyshkil-prod' \
  /tmp/vyshkil-images.tar.gz deploy@<PROD_VM_IP>:/opt/vyshkil/images/
```

На прод-VM:

```bash
# Завантажити образи (потребує sudo, бо deploy не в групі docker)
sudo docker load < /opt/vyshkil/images/vyshkil-images.tar.gz

# Перевірити
sudo docker images | grep vyshkil
```

## Крок 7. Запуск стеку

На прод-VM:

```bash
cd /opt/vyshkil

# Запустити (перший раз — ініціалізує БД, запускає міграції)
sudo docker compose up -d

# Перевірити healthchecks
sudo docker compose ps

# Логи (перші хвилини — дочекатись, що все стартувало)
sudo docker compose logs -f --tail=50
```

## Крок 8. Перевірка

1. **Локально на прод-VM**:
   ```bash
   curl -s http://localhost:3000 | head -20
   # Має повернути HTML React SPA
   ```

2. **Через Cloudflare tunnel** (з будь-якого пристрою):
   - Відкрити `https://taktoblik.<ваш-домен>.com` у браузері
   - Якщо налаштований Cloudflare Access — пройти автентифікацію

3. **Перевірити ізоляцію мережі** (з прод-VM):
   ```bash
   # app-контейнер НЕ має виходу в інтернет
   sudo docker exec vyshkil-app-1 wget -q --spider --timeout=5 https://google.com
   # Має зафейлитись (timeout/connection refused)

   # cloudflared працює
   sudo systemctl status cloudflared
   ```

4. **Перевірити WhatsApp** (якщо notifier потрібен):
   - Відкрити адмін-панель → WhatsApp → прив'язка QR

## Оновлення (наступні деплої)

```bash
# На dev-VM: зібрати нові образи, зберегти
cd ~/vyshkil && docker compose build
docker save vyshkil-app:latest vyshkil-notifier:latest | gzip > /tmp/vyshkil-images.tar.gz

# Передати на прод-VM (той самий rsync)

# На прод-VM:
sudo docker load < /opt/vyshkil/images/vyshkil-images.tar.gz
cd /opt/vyshkil

# Бекап БД перед міграціями!
sudo docker exec vyshkil-db-1 pg_dump -U taktoblik taktoblik | gzip > /tmp/backup-$(date +%Y%m%d).sql.gz

# Перезапустити стек
sudo docker compose up -d

# Перевірити
sudo docker compose ps
sudo docker compose logs -f --tail=50
```

## Відкат

```bash
# На прод-VM: зупинити
cd /opt/vyshkil && sudo docker compose down

# Відновити БД з бекапу (якщо міграції незворотні)
sudo docker compose up -d db
gunzip < /tmp/backup-YYYYMMDD.sql.gz | sudo docker exec -i vyshkil-db-1 psql -U taktoblik taktoblik

# Завантажити попередні образи
sudo docker load < /opt/vyshkil/images/vyshkil-images-previous.tar.gz

# Запустити зі старими образами
sudo docker compose up -d
```

## Відомі особливості

1. **DNS на хості Proxmox**: dev-VM має проблему з DNS-резолвінгом зовнішніх доменів (працює
   тільки по IP). Прод-VM через VyOS може мати інший DNS-маршрут — перевірити `resolvectl status`
   після створення. Якщо DNS не працює — `cloudflared` і `notifier` не зможуть з'єднатись із
   зовнішніми сервісами.

2. **Docker і sudo**: на проді `deploy` НЕ в групі docker — усі `docker`-команди через `sudo`.
   Це навмисно (11 §2: "адміністратори не в групі docker").

3. **SYS_ADMIN для notifier**: `cap_add: SYS_ADMIN` — свідомий компроміс для Chromium sandbox
   (whatsapp-web.js). Задокументовано в `.claude/decisions/notifier-ts-whatsapp-web-js.md`.

4. **Порти тільки на 127.0.0.1**: застосунок (3000), Postgres (5432), NATS (4222/8222) — все на
   localhost. Зовнішній доступ — тільки через cloudflared tunnel.

5. **RAM**: прод-VM має 2 ГБ RAM + 2 ГБ swap. Рантайм-стек споживає ~220 МБ. Запас достатній,
   але моніторити `free -h` перших тижнів.

## Жорсткі правила (з CLAUDE.md — не порушувати!)

- Людей поіменно не зберігати й не логувати — тільки кількості груп
- Жодних мережевих викликів з даними назовні (крім знеособлених WhatsApp-повідомлень)
- `source_files/` на проді відсутні
- Секрети — тільки через Ansible Vault, не в git, не в логах
- ДСК: номер і назва частини НІКОЛИ разом на одному екрані
