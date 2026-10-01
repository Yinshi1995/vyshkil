---
tags: [spec, infra, ansible, proxmox, dev-environment]
date: 2026-10-01
status: до виконання ЗАРАЗ (після відповідей замовника на §1)
---

# 10. Dev-VM на Proxmox для автономної розробки

## 0. Мета і межі

**Мета:** Linux-віртуалка на Proxmox замовника, на якій Claude Code сам пише код, збирає (`cargo`,
`cargo leptos`), запускає `docker compose` (db, nats, app, notifier), гонить юніт/інтеграційні/e2e-тести
— **без участі Windows-хоста**.

**Головний мотив** — прибрати клас проблем MSVC-лінкера (LNK2019 між codegen-unit-ами, LNK1140 —
ліміт PDB), які повторюються з ростом графа залежностей. Критерій успіху тому не "VM піднялась", а
"повна збірка й тести проходять **зі стандартними налаштуваннями профілю**, без Windows-обхідних
змінних".

**Поза межами цього документа:** прод-деплой (див. `11-prod-deploy.md` — НЕ виконувати зараз),
CI/CD, моніторинг.

**Принцип:** спершу питання й план, потім код. Не вгадувати параметри Proxmox (§1).

---

## 1. Відкриті питання — ЗАДАТИ ЗАМОВНИКУ ДО ПОЧАТКУ (одним повідомленням, нумеровано)

Не робити жодних допущень по цих пунктах; де є рекомендація — запропонувати її як варіант за замовчуванням.

**Proxmox**
1. Версія PVE (`pveversion`)? Один вузол чи кластер?
2. Доступ: є API-токен (рекомендовано: окремий користувач `ansible@pve` з роллю тільки на потрібний
   пул/сторедж) — чи тільки веб-інтерфейс / SSH на хост?
3. Storage: імена пулів для дисків VM (`local-lvm`, ZFS, Ceph…) і для ISO/шаблонів (`local`)?
   Є Proxmox Backup Server або налаштовані `vzdump`-бекапи? (важливо для §4.6 — ДСК-дані)
4. Є готовий cloud-init шаблон? Якщо ні — який дистрибутив: рекомендація **Ubuntu 24.04 LTS** або
   **Debian 12** (обидва — ок для Rust/Docker/Chromium; Ubuntu простіше з Playwright-залежностями).
5. Скільки вільних ресурсів на хості (CPU-ядра, RAM, диск) — щоб узгодити розмір VM з §3.

**Мережа**
6. Бридж (`vmbr0`?), VLAN-тег, якщо є.
7. IP: статичний (адреса/маска/шлюз/DNS) чи DHCP (тоді — резервація за MAC)?
8. Як замовник підключається до VM: та сама LAN, VPN (який), чи тільки через PVE-хост?
9. Політика виходу VM в інтернет: повний вихід чи через проксі/allowlist? Мінімально потрібні:
   crates.io / static.crates.io / index.crates.io, registry.npmjs.org, github.com (+ objects.githubusercontent.com),
   docker.io / registry-1.docker.io / production.cloudflare.docker.com, api.anthropic.com (Claude Code),
   дзеркала apt, nodejs, rustup (static.rust-lang.org), playwright CDN.

**Код і дані**
10. Де живе git-remote? (Зараз пушить людина; віддаленого репо може не бути.) Варіанти: GitHub/GitLab
    приватний, власний Gitea на PVE, або bare-репо на самій VM (тоді Windows стає `remote`, а не навпаки).
11. **`source_files/` (ДСК) на dev-VM** — дозволено копіювати? Вони потрібні для сід-даних, тестів імпорту
    і золотих тестів. Рекомендація: так, але на **окремий віртуальний диск з `backup=0`** (не потрапляє у
    vzdump/PBS), змонтований у репо як `source_files/` (див. §4.6). Без явного "так" — не копіювати;
    тести, що їх потребують, і так skip-аються.

**Агент**
12. Claude Code на VM: вхід через підписку (інтерактивний `/login` — замовник зробить сам через SSH)
    чи API-ключ (тоді — куди класти: тільки у `~/.claude` користувача `dev`, не в репо)?
13. Звідки запускати Ansible (control node)? Ansible не працює нативно на Windows. Варіанти:
    (а) **WSL2 на Windows-машині** (рекомендовано: той самий control node потім для проду);
    (б) з самої dev-VM на `localhost` (bootstrap-скрипт → `ansible-pull`/local);
    (в) з PVE-хоста — **не рекомендовано** (гіпервізор тримаємо чистим).

**Після відповідей:** коротко підсумувати прийняті параметри в `infra/README.md` і в
`.claude/decisions/dev-vm-parameters.md`, і тільки тоді — Фаза 1.

---

## 2. Інструмент: Ansible чи скрипт

Рішення: **Ansible** (не скрипт), з обґрунтуванням у `.claude/decisions/iac-ansible.md`:
- прод (11) однаково буде на Ansible — ролі `base`, `docker`, `firewall` перевикористовуються;
- ідемпотентність: повторний прогін = безпечне "доведення до стану" після ручних змін/оновлень;
- одна VM — не аргумент проти: плейбук невеликий, а "скрипт, який вже налаштовував машину" не можна
  безпечно перезапустити.

Створення самої VM на Proxmox — окрема роль/плейбук `provision`, яка залежить від відповіді на §1.2:
- є API-токен → колекція Ansible для Proxmox. **Перевір актуальну назву колекції й модулів** (модулі
  Proxmox переїжджали з `community.general` в окрему колекцію `community.proxmox` — використовувати
  чинну); клон з cloud-init шаблону, cloud-init: користувач, SSH-ключ, мережа;
- тільки SSH на хост → невеликий ідемпотентний скрипт з `qm clone/set/resize/start`, який запускається на
  PVE-хості, решта — Ansible по SSH на VM;
- тільки веб → покрокова інструкція в `infra/README.md` для ручного клонування + Ansible конфігурує вже
  створену VM.

---

## 3. Розмір VM (не мінімальні "dev"-цифри)

Rust-граф проєкту важкий (Leptos з макро-деревом, SeaORM, async-nats, wasm32-таргет + SSR = дві
повні збірки, release з LTO). Лінковка й LTO — пікові споживачі RAM. Плюс у той самий час працюють
Postgres, NATS, app, notifier з Chromium (mem_limit 1g, shm 512m) і Playwright з браузером.

| Ресурс | Рекомендовано | Мінімум | Чому |
|---|---|---|---|
| vCPU | 8 (тип `host`) | 6 | паралельна компіляція; `host` — щоб працювали SIMD-оптимізації і не гальмував rustc |
| RAM | 24 ГБ | 16 ГБ | release+LTO+wasm-opt + docker-стек + Chromium + Playwright одночасно |
| Swap | 8 ГБ (або zram) | 4 ГБ | страховка від OOM при лінковці, не робоча пам'ять |
| Диск системний | 200 ГБ, thin, `discard=on`, `ssd=1` | 120 ГБ | `target/` (debug+release+wasm) 30–60 ГБ, кеш sccache, docker-образи, `~/.cargo/registry` |
| Диск ДСК | 10 ГБ, **`backup=0`** | — | тільки `source_files/` (§4.6) |

Агент **заміряє** реальні цифри після Фази 3 (пік RSS під час `cargo leptos build --release`, час
холодної/теплої збірки, розмір `target/`) і пише в `infra/README.md` + MEMORY.md — якщо рекомендовані
цифри завищені/занижені, скоригувати.

---

## 4. Що налаштувати на VM (ролі)

Структура (новий каталог, у git):
```
infra/
├── README.md                    як користуватись, параметри, відповіді на §1
├── CLAUDE.md                    карта каталогу (формат 07 §4)
└── ansible/
    ├── ansible.cfg
    ├── requirements.yml         колекції з точними версіями
    ├── inventories/dev/         hosts.yml, group_vars/ (без секретів у відкритому вигляді — §4.7)
    ├── playbooks/
    │   ├── provision-dev-vm.yml    створення VM (залежить від §1.2)
    │   └── dev-vm.yml              конфігурація VM
    └── roles/
        ├── base/                користувачі, ssh, оновлення, час, локаль, swap, лімітити
        ├── docker/              Docker Engine + compose plugin з офіційного репо
        ├── rust_toolchain/      rustup, toolchain з rust-toolchain.toml, таргети, компоненти
        ├── node/                Node 22 LTS (для services/notifier), без глобального мотлоху
        ├── devtools/            лінкер, кеш, cargo-leptos, wasm-opt, Playwright-залежності, утиліти
        ├── claude_code/         Claude Code CLI + налаштування користувача
        ├── project/             клон репо, ДСК-диск, .env з шаблону, перший прогін
        └── firewall/            nftables: вхідні тільки SSH (+ проброс портів не потрібен — §4.8)
```
Ролі `base`, `docker`, `firewall` пишуться **одразу придатними для проду** (параметризовані), бо
11 їх перевикористає.

### 4.1. base
- Користувач `dev` (без пароля, SSH-ключ замовника), sudo — тільки якщо замовник погодиться (питання до §1.12);
  root-логін і парольна автентифікація SSH — вимкнено.
- `unattended-upgrades` тільки для security; часовий пояс Europe/Kyiv; локаль `uk_UA.UTF-8` + `en_US.UTF-8`.
- Swap/zram за §3; `vm.swappiness` помірний; `fs.inotify.max_user_watches` збільшено (cargo/IDE).
- `qemu-guest-agent` (щоб PVE бачив IP і коректно робив snapshot/shutdown).

### 4.2. docker
- Docker Engine + `docker compose` v2 з офіційного репозиторію (не snap). Користувач `dev` у групі `docker`
  (**увага в README: група docker = фактично root на VM**; для dev-VM прийнятно, для проду — ні).
- `log-driver: local` з ротацією; `live-restore: true`.
- Перевірка: `docker compose config` репо валідний; мережі `internal` (без шлюзу) і `egress` створюються.

### 4.3. rust_toolchain + devtools
- rustup від користувача `dev`; версія — з `rust-toolchain.toml` репо (якщо файла нема — **додати його в
  репо** з поточною версією, щоб Windows і VM збирали одним toolchain-ом); таргет `wasm32-unknown-unknown`;
  компоненти `rustfmt`, `clippy`.
- Лінкер: **mold** або **lld** через `.cargo/config.toml` під `[target.x86_64-unknown-linux-gnu]` (швидше й
  менше RAM ніж ld.bfd). Обрати, заміряти, записати рішення.
- `sccache` як `RUSTC_WRAPPER` (локальний кеш на диску VM).
- `cargo-leptos` і `wasm-opt` (binaryen) — версії, сумісні з тими, що в Dockerfile; встановлення з
  прероблених релізів з перевіркою sha256, як уже зроблено для Docker-збірки.
- `cargo-modules` (граф коду, див. рішення в `.claude/decisions/`).
- Playwright: системні залежності браузерів + браузери (тест e2e/); Chromium для локального прогону
  notifier поза Docker (за потреби).
- Утиліти: git, build-essential, pkg-config, clang, jq, ripgrep, fd, htop, tmux, postgresql-client, `nats` CLI.

### 4.4. Прибрати Windows-обхідні налаштування з репо (зміни в репо, не на VM)
- Знайти, де задаються `CARGO_PROFILE_DEV_CODEGEN_UNITS=1` / `CARGO_PROFILE_DEV_DEBUG=1` (env, `.cargo/config.toml`,
  скрипти). Якщо в `.cargo/config.toml` — перенести під `[target.x86_64-pc-windows-msvc]` або в окремий
  документований env-файл тільки для Windows. На Linux — **стандартні** профілі.
- Критерій: на VM `cargo test --workspace` і `cargo leptos build` проходять без жодних додаткових env-змінних.

### 4.5. claude_code
- Встановити Claude Code офіційним способом (перевір актуальний метод встановлення в документації
  Anthropic перед написанням ролі — не з пам'яті).
- Автентифікація — **не автоматизувати** секретами в репо: за §1.12 замовник логіниться сам після
  першого прогону; роль тільки виводить інструкцію.
- Переглянути `.claude/settings.json` і `.claude/settings.local.json` репо: PowerShell-дозволи —
  Windows-специфічні; додати еквіваленти для Linux (bash) у `settings.json`, особисті — лишити в local.
  SessionStart-хук (`cat $CLAUDE_PROJECT_DIR/...`) на Linux працює — перевірити.
- MEMORY.md / кореневий CLAUDE.md: розділ "Запуск" зробити двоплатформним (Linux dev-VM — основний,
  Windows — історичний з граблями).

### 4.6. project і ДСК-дані
- Клон репо за відповіддю на §1.10 (deploy-ключ тільки на читання, якщо remote зовнішній).
- **`source_files/` (ДСК):** тільки за явного "так" у §1.11. Тоді:
  окремий віртуальний диск з `backup=0` (не потрапляє у vzdump/PBS) → файлова система → змонтовано в
  `/srv/dsk/source_files` → симлінк/bind-mount у `<repo>/source_files`; права `0700` для `dev`;
  копіювання з Windows — `rsync` по SSH (команда в README), **не через git** і не через спільні тимчасові
  каталоги. Снапшоти PVE з RAM-станом — у README попередити, що snapshot включає й цей диск локально.
- `.env` з `.env.example` (dev-паролі як зараз — допустимо для dev; прод — Vault, 11).
- `data/` (збережені файли застосунку) — на системному диску, у `.gitignore` (вже є).

### 4.7. Секрети dev-VM
Dev-паролі (DB, NATS-користувачі) — як у dev-compose, це нормально. Але: SSH-ключі, токен Proxmox API,
ключі Claude Code — **не в git**. Для токена Proxmox одразу використовувати Ansible Vault (той самий
механізм піде в прод). Файл vault-паролю — поза репо.

### 4.8. firewall (dev)
nftables: вхідні — тільки SSH (з підмережі замовника за §1.8). Веб-застосунок (3000), NATS-монітор
(8222) — **не відкривати**: доступ через SSH-тунель (`ssh -L 3000:localhost:3000 dev-vm`) — інструкція в README.
(Docker сам керує iptables для опублікованих портів — у compose портів на 0.0.0.0 бути не повинно,
перевірити; NATS 4222 не публікується — так і лишити.)

---

## 5. Фази і критерії готовності

**Фаза 0 — питання.** Надіслати замовнику §1 одним повідомленням. Чекати відповідей. Записати
параметри (`infra/README.md`, `.claude/decisions/dev-vm-parameters.md`, `iac-ansible.md`).
☐ Відповіді отримано ☐ Параметри записано ☐ Рішення записано

**Фаза 1 — каркас IaC.** `infra/` за §4, `requirements.yml` з точними версіями, `ansible-lint` чистий,
карта `infra/CLAUDE.md`, рядок у кореневому CLAUDE.md ("Інфраструктура → infra/README.md, docs/spec/10, 11").
☐ `ansible-lint` без помилок ☐ `ansible-playbook --syntax-check` для обох плейбуків

**Фаза 2 — VM існує.** `provision-dev-vm.yml` (або скрипт/інструкція за §1.2) створює VM з §3.
☐ VM завантажується ☐ SSH ключем як `dev` ☐ qemu-guest-agent показує IP у PVE ☐ повторний прогін — `changed=0`

**Фаза 3 — конфігурація.** `dev-vm.yml` з усіма ролями.
☐ Повторний прогін — `changed=0` (ідемпотентність)
☐ `rustc --version` = версія з `rust-toolchain.toml` ☐ `cargo leptos --version`, `wasm-opt --version` сумісні з Dockerfile
☐ `docker run hello-world` від `dev` ☐ `node --version` = 22.x
☐ Claude Code запускається (вхід — замовник)

**Фаза 4 — проєкт збирається і проходить усе (головний критерій).** На VM, з чистого клону,
**без Windows-обхідних env-змінних**:
☐ `cargo test --workspace` зелений (тести з `source_files/` — зелені, якщо диск ДСК підключено, інакше skip)
☐ `cargo clippy --workspace` без помилок
☐ `cargo leptos build` і `cargo leptos build --release`
☐ `docker compose up -d` → db, nats, app, notifier — healthy; застосунок відкривається через SSH-тунель
☐ e2e Playwright (`e2e/`) зелені headless
☐ notifier: контейнер збирається; з `internal`-мережі (`app`) вихід в інтернет **неможливий**
   (`docker compose exec app` → спроба з'єднання назовні падає) — перевірка тим самим скриптом,
   що піде в прод-тести (11 §6)
☐ Заміри (§3) записані в README і MEMORY.md

**Фаза 5 — передача.** `infra/README.md`: як створити VM з нуля, як оновити, як підключитись (SSH,
тунель для браузера, VS Code Remote-SSH за бажанням), як перенести `source_files/`, як відновитись
(перестворення VM: що втрачається — `target/` і кеші (не страшно), що зберегти — `wa-session`, БД dev).
MEMORY.md: "основне середовище розробки — dev-VM; Windows — тільки для перегляду". `/handoff`.
☐ Замовник пройшов README сам і підключився ☐ Агент на VM виконав одну реальну задачу з roadmap
від початку до коміту
