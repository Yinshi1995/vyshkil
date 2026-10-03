# infra/ — Dev-VM на Proxmox

Карта каталогу (що є що) — `infra/CLAUDE.md`. Параметри нижче зафіксовані відповідями замовника
й реальною інвентаризацією хоста — `.claude/decisions/dev-vm-parameters.md` (джерело, не вгадано).

## Параметри (коротко)

| Що | Значення |
|---|---|
| Proxmox | PVE 9.2.2, вузол `pidhotovka`, без кластера |
| Шаблон | VMID 9000, Ubuntu 24.04 cloud-image (створюється з нуля) |
| Dev-VM | VMID 300, 6 vCPU (`host`), 8192 МБ RAM, 8 ГБ swap, 150 ГБ диск на `local-lvm` |
| Мережа | `vmbr0` (домашня LAN), статична IP `192.168.1.200/24` (DHCP не видає оренду для нового MAC — знайдено емпірично, `[[dev-vm-parameters]]`), без VLAN |
| Доступ | VPN на VyOS → SSH; застосунок (3000) і NATS-монітор (8222) — тільки через SSH-тунель |
| Git | Bare-репо на самій VM (`/srv/git/vyshkil.git`) — Windows стає `remote` |
| `source_files/` | НЕ копіюється (явного "так" не було) — тести з ними skip, як і зараз |
| Control node | WSL2 / Arch Linux на Windows-машині розробника |

**Свідомий компроміс**: 8 ГБ RAM — усе вільне на хості, без апгрейду. Виміряно у Фазі 4
(розділ "Заміри" нижче) — вміщається одночасно, послідовний build→stop→up не знадобився.

## Як це створити з нуля

1. Control node — WSL2/Arch вже готовий (`pacman -Syu`, НЕ `-Sy` — лишає систему в частковому
   апдейті, грабля вже зловлена тут): `ansible-core 2.21.4`, `ansible-lint 26.9.0`.
2. Встановити колекції: `cd infra/ansible && ansible-galaxy collection install -r requirements.yml`.
3. Створити `inventories/dev/group_vars/vault.yml` (не в git) за шаблоном `vault.yml.example`:
   `ansible-vault create inventories/dev/group_vars/vault.yml`, вставити `vault_pve_api_token_id`,
   `vault_pve_api_token_secret`, `vault_dev_vm_user_ssh_public_key`.
4. `ansible-playbook playbooks/provision-dev-vm.yml --ask-vault-pass` — реально прогнано
   (2026-10-01), VM 300 піднята й відповідає. Грабля, зловлена саме тут: `download-url` вимагає
   `.qcow2`-розширення файлу й `images`-storage (`local-lvm`, не `local`/iso-only);
   `proxmox_kvm update:true` мовчки дропає `net0` без `update_unsafe: true`; disk resize вимагає
   явний суфікс (`"150G"`). DHCP не видає оренду новому MAC (~100-110с підвисання на бутсті,
   підтверджено двома порівняльними тестами розміру диска) — звідси статична IP нижче.
5. IP — **статична** (`192.168.1.200`, вписана в `inventories/dev/hosts.yml`), не з PVE UI —
   DHCP для цієї мережі цього MAC-а не обслуговує (п.4).
6. `ansible-playbook playbooks/dev-vm.yml --ask-vault-pass` — реально прогнано, усі ролі
   (`base`/`docker`/`rust_toolchain`/`node`/`devtools`/`claude_code`/`project`/`firewall`) зелені.
7. На VM по SSH: `claude` → інтерактивний `/login` (підписка — не автоматизовано).

**Крок "код на VM" — `git push vm main` ще НЕ виконаний, відкрито** (роль `project` створює
порожній bare-репо й саме це друкує як наступний крок, якщо гілки `main` там ще нема):
```
git remote add vm ssh://dev@192.168.1.200/srv/git/vyshkil.git   # вже є в репо
git push vm main
```
Попередня спроба замовника впала `Permission denied (publickey)` (ймовірно особистий ключ без
агента в ту мить) — ще не перевірено повторно. Поточна робоча копія `~/vyshkil` на VM (до коміту
`753c042` включно, подальші фікси Фази 4 донесені точковим `rsync`) була занесена туди НЕ через
цей механізм, а прямим копіюванням робочого дерева — тимчасовий обхід, задокументований чесно,
не прихований. Після успішного `git push vm main` повторний прогін `dev-vm.yml` (роль `project`)
має штатно перетворити це на справжній клон bare-репо.

## Підключення

Два SSH-ключі на `dev`: особистий замовника (з паролем — для ручних сесій) і окремий
`~/.ssh/vyshkil-dev` на Windows-машині (без пароля — для Ansible/Claude Code, неінтерактивний
доступ).

```
ssh dev@192.168.1.200                                                      # сама VM, особистий ключ
ssh -i ~/.ssh/vyshkil-dev dev@192.168.1.200                                # той самий, автоматизація
ssh -L 3000:localhost:3000 -L 8222:localhost:8222 dev@192.168.1.200        # + тунель для браузера
```

## Як оновити код на VM

**Фактичний робочий шлях (перевірено, 2026-10-01)** — `git bundle`, не push і не DNS-залежний:
```
# з Windows (git bundle — локальний файл, НЕ мережевий виклик, settings.json це не зачіпає):
git bundle create /tmp/vyshkil.bundle main

# передати файл на VM (з WSL — Windows OpenSSH не має встановленого rsync):
rsync -av -e 'ssh -i ~/.ssh/vyshkil-dev -o IdentitiesOnly=yes' \
  /tmp/vyshkil.bundle dev@192.168.1.200:vyshkil.bundle

# на VM — імпортувати й перевести working copy на цей стан:
cd ~/vyshkil && git fetch ~/vyshkil.bundle main:refs/heads/incoming-main \
  && git reset --hard incoming-main && git branch -d incoming-main && rm ~/vyshkil.bundle
```
Працює БЕЗ `git push` (bundle — локальний артефакт, передається звичайним rsync) і БЕЗ
залежності від DNS/github.com з самої VM. `git reset --hard` тут безпечний лише тому, що
робоча копія на VM — чистий дзеркальний чекаут, без власних незакомічених змін, вартих
збереження (перевіряти `git status --short` ПЕРЕД reset, якщо хтось коли-небудь редагував
код напряму на VM — не припускати це сліпо).

**Застаріле (було до 2026-10-01, замінено bundle-способом вище)**: ad hoc rsync одного файлу за
раз з явним шляхом призначення на обох кінцях — працювало, але не синхронізувало git-історію
VM (вона лишалась застиглою на старому коміті, доки git-історію не підтягнуто bundle-ом), і
кожен такий rsync ніс реальний ризик (кілька файлів в одну директорію без `-R` мовчки губить
підкаталоги; `--relative` з уже-конкретною директорією призначення ПОДВОЮЄ шлях).

**Штатний push-шлях (`git push vm main` у bare-репо на VM) лишається НЕ перевіреним** — див.
"Відоме відкрите" нижче; bundle-спосіб вище його не потребує, тож не є блокером.

Після оновлення коду — перезбірка:
```bash
# React SPA (потрібно Node 22+, роль `node`)
cd ~/vyshkil/web && npm ci && npx vite build   # → web/dist/

# Rust-сервер
cd ~/vyshkil && cargo build -p server

# Або Docker Compose (повний стек, включно з notifier)
cd ~/vyshkil && docker compose up --build -d
```
Сервер автоматично використовує React SPA, якщо `web/dist/` існує (`server/src/spa.rs`);
інакше fallback на Leptos SSR. `cargo leptos build` більше НЕ потрібен для React-шляху.

Для щоденної роботи НАПРЯМУ на VM (ціль "основне середовище розробки — dev-VM", `.claude/
memory/MEMORY.md`) — Claude Code-сесія на VM, VS Code Remote-SSH, або редактор по SSH — коміт
там локальний, а подальший `git push origin` з САМОЇ VM поки заблокований DNS-проблемою нижче.

## Перенесення `source_files/` (ДСК)

Зараз НЕ копіюється (`project_mount_source_files: false`, `.claude/decisions/
dev-vm-parameters.md` — явного "так" від замовника не було). Щоб увімкнути пізніше: встановити
`project_mount_source_files: true` в `inventories/dev/group_vars/all.yml`, перенести файли вручну
(`rsync`, той самий застережний патерн вище — ОДИН файл/архів за раз, не мовчки директорією) у
`project_source_files_disk_gb`-том на VM, повторний прогін ролі `project`. Жорстке правило проєкту
лишається чинним і на VM: не комітити самі файли, не цитувати сирий текст дослівно в
комітах/логах/відповідях.

## Відоме відкрите — DNS на хості VM не резолвить зовнішні домени

`resolvectl status`: DNS-сервер `192.168.1.1` (LAN-роутер), search-домен `inner.mod.ua`. Прямий
IP-маршрут в інтернет є (`ping 8.8.8.8` — ок, `curl --resolve github.com:443:<ip> https://...` —
HTTP 200), але голий `curl https://github.com` падає на резолвінгу імені. Це виглядає на
мережеву політику замовника (військова мережа), НЕ на поломку VM — свідомо НЕ обходжу мовчки
(публічний резолвер/запис у `/etc/hosts` без дозволу) — рішення за замовником: або мережева
команда додає форвардинг зовнішніх імен на `192.168.1.1`, або свідомо приймаємо "VM бачить
інтернет лише по IP" як постійне обмеження цього сегмента.

## Відновлення / перестворення VM

- Що втрачається без жалю: `target/`, кеші sccache/cargo registry, docker-образи — усе
  перебудовується повторним прогоном.
- Що зберегти ПЕРЕД перестворенням: `wa-session` (docker volume — жива прив'язка WhatsApp),
  dev-БД (`taktoblik_pgdata` volume), будь-які незакомічені зміни в робочій копії на VM
  (`git push` їх на Windows чи в bare-репо перед видаленням VM).

## Заміри (Фаза 4, 2026-10-01)

- `docker stats` під повним стеком (db+nats+app+notifier, усі healthy): разом ~220 МБ RSS
  (`app` 8 МБ, `db` 132 МБ, `notifier` 67 МБ, `nats` 11 МБ) — рантайм тривіально вміщається.
- `free -h` під час/після `cargo leptos build --release` в Docker-білдері: короткочасний
  swap-спайк (~300 МБ), система лишається відповідною, `available` повертається до ~7 ГБ після
  збірки. Відкрите питання Фази 0 ("чи вміщається release+LTO + повний docker-стек одночасно")
  закрито: вміщається, 8 ГБ RAM + 8 ГБ swap достатньо без потреби послідовного build→stop→up.

## Відомі компроміси й відкриті питання, записані відкрито (не мовчки)

- Dev-VM на тій самій LAN, що й Windows-хост — НЕ за VyOS-сегментацією, на відміну від решти
  сервісів хоста. Прод (`docs/spec/11-prod-deploy.md`) цей ярлик не успадковує.
- Групa `docker` = фактично root на хості (стандартний компроміс Docker) — прийнятно для
  одного розробника на dev, прод (11 §2) цього не успадковує.
- **Git-remote на VM ще не запрацював "штатно"** — `git push vm main` ще не виконано/не
  перевірено повторно; поточна робоча копія на VM занесена rsync-копіюванням, не клоном
  bare-репо (деталі й команди — вище, "Як оновити код на VM").
- **DNS на хості VM не резолвить зовнішні домени** (деталі вище) — блокує прямий `git push
  origin`/`git clone` з самої VM, поки не вирішено мережевою командою замовника чи свідомим
  прийняттям обмеження.
