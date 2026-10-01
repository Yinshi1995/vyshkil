# infra/

- Усе тут — Ansible, не разові скрипти (`[[iac-ansible]]`). `ansible-lint` мусить лишатись чистим,
  `ansible-playbook --syntax-check` — для обох плейбуків.
- Ролі `base`, `docker`, `firewall` параметризовані навмисно (`*_passwordless_sudo`,
  `*_add_primary_user_to_group` тощо) — `docs/spec/11-prod-deploy.md` перевикористає їх без
  переписування, лише інший `group_vars/prod`.
- Секрети — лише `inventories/*/group_vars/vault.yml` (не в git, шаблон — `vault.yml.example`),
  шифрується `ansible-vault`. Пароль vault-у — поза репо.
- Параметри dev-VM (RAM/мережа/git-remote/source_files) — не вгадані, зафіксовані відповідями
  замовника: `.claude/decisions/dev-vm-parameters.md`.
- Порядок ролей у `playbooks/dev-vm.yml` важливий: `rust_toolchain`/`node`/`devtools` ставлять
  інструменти ДО того, як `project` (останній) клонує репозиторій — версія Rust там синхронна
  з `/rust-toolchain.toml` вручну (репо ще не існує на VM, коли роль rust_toolchain виконується).
- `roles/provision/` ходить у Proxmox API (токен), не по SSH на гіпервізор — замовник тримає
  гіпервізор чистим. Ще НЕ перевірено проти живого PVE (перший реальний прогін — Фаза 2,
  `docs/spec/10-dev-vm.md` §5).

| Елемент | Що це | Хто використовує |
|---|---|---|
| `ansible.cfg` | Інвентар за замовчуванням, `become` для всіх задач, pipelining | усі прогони |
| `requirements.yml` | Точні версії колекцій (`community.proxmox` 2.0.0 тощо) | `ansible-galaxy collection install -r` |
| `inventories/dev/hosts.yml` | `proxmox_api` (localhost, API-токен) + `dev` (сама VM, IP — вписати після Фази 2) | обидва плейбуки |
| `inventories/dev/group_vars/all.yml` | Несекретні параметри (розмір VM, мережа, версії) | усі ролі |
| `inventories/dev/group_vars/vault.yml.example` | Шаблон секретів (API-токен, SSH-ключ) — реальний `vault.yml` не в git | перший прогін |
| `playbooks/provision-dev-vm.yml` | Створює шаблон 9000 і клонує dev-VM 300 (роль `provision`) | Фаза 2 |
| `playbooks/dev-vm.yml` | Конфігурує VM (усі ролі з `roles/`) | Фаза 3 |
| `roles/provision` | Proxmox API: download-url імпорт cloud-образу, create/clone/start VM | `provision-dev-vm.yml` |
| `roles/base` | Користувач `dev`, SSH, swap, локаль/час, sysctl, qemu-guest-agent | `dev-vm.yml` |
| `roles/docker` | Docker Engine + compose plugin з офіційного репо | `dev-vm.yml` |
| `roles/rust_toolchain` | rustup/toolchain, wasm32-таргет, mold, sccache, cargo-leptos, wasm-opt, cargo-modules | `dev-vm.yml` |
| `roles/node` | Node 22 LTS (NodeSource) для `services/notifier` | `dev-vm.yml` |
| `roles/devtools` | git, build-essential, ripgrep/fd/jq/tmux, Playwright system-залежності, `nats` CLI | `dev-vm.yml` |
| `roles/claude_code` | Claude Code CLI з офіційного apt-репо (канал stable), вхід — замовник сам | `dev-vm.yml` |
| `roles/project` | Bare-репо, робоча копія, `.env`, `data/` (source_files/ вимкнено за замовчуванням) | `dev-vm.yml` |
| `roles/firewall` | nftables: вхідні лише SSH; перевірка, що docker-compose.yml нічого не публікує на 0.0.0.0 | `dev-vm.yml` |
