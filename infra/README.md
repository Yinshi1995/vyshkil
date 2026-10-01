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

**Свідомий компроміс**: 8 ГБ RAM — усе вільне на хості, без апгрейду. Release-збірка з LTO +
повний `docker compose` стек (Chromium/Playwright) одночасно НЕ гарантовано вміщається — Фаза 4
мусить це заміряти і, якщо не влазить, виконувати послідовно (build → зупинити → підняти стек).

## Як це створити з нуля

1. Control node — WSL2/Arch вже готовий (`pacman -Syu`, НЕ `-Sy` — лишає систему в частковому
   апдейті, грабля вже зловлена тут): `ansible-core 2.21.4`, `ansible-lint 26.9.0`.
2. Встановити колекції: `cd infra/ansible && ansible-galaxy collection install -r requirements.yml`.
3. Створити `inventories/dev/group_vars/vault.yml` (не в git) за шаблоном `vault.yml.example`:
   `ansible-vault create inventories/dev/group_vars/vault.yml`, вставити `vault_pve_api_token_id`,
   `vault_pve_api_token_secret`, `vault_dev_vm_user_ssh_public_key`.
4. Фаза 2 (ще не виконано, потребує підтвердження перед реальним прогоном проти PVE):
   `ansible-playbook playbooks/provision-dev-vm.yml --ask-vault-pass`.
5. Після старту VM — у PVE UI (Summary → IPs, потребує `qemu-guest-agent`) знайти реальну
   DHCP-адресу, вписати в `inventories/dev/hosts.yml` (`ansible_host`).
6. Фаза 3: `ansible-playbook playbooks/dev-vm.yml --ask-vault-pass`.
7. На Windows, з кореня репо: `git remote add vm ssh://dev@<IP>/srv/git/vyshkil.git && git push vm main`,
   потім повторний прогін `dev-vm.yml` (роль `project`) зробить робочу копію на VM.
8. На VM по SSH: `claude` → інтерактивний `/login` (підписка — не автоматизовано).

## Підключення

Два SSH-ключі на `dev`: особистий замовника (з паролем — для ручних сесій) і окремий
`~/.ssh/vyshkil-dev` на Windows-машині (без пароля — для Ansible/Claude Code, неінтерактивний
доступ).

```
ssh dev@192.168.1.200                                                      # сама VM, особистий ключ
ssh -i ~/.ssh/vyshkil-dev dev@192.168.1.200                                # той самий, автоматизація
ssh -L 3000:localhost:3000 -L 8222:localhost:8222 dev@192.168.1.200        # + тунель для браузера
```

## Відновлення / перестворення VM

- Що втрачається без жалю: `target/`, кеші sccache/cargo registry, docker-образи — усе
  перебудовується повторним прогоном.
- Що зберегти ПЕРЕД перестворенням: `wa-session` (docker volume — жива прив'язка WhatsApp),
  dev-БД (`taktoblik_pgdata` volume), будь-які незакомічені зміни в робочій копії на VM
  (`git push` їх на Windows чи в bare-репо перед видаленням VM).

## Заміри (Фаза 4 заповнює після реального прогону)

Пік RSS під `cargo leptos build --release`, час холодної/теплої збірки, розмір `target/` —
TODO, ще не виміряно (VM ще не створена).

## Відомі компроміси, записані відкрито (не мовчки)

- Dev-VM на тій самій LAN, що й Windows-хост — НЕ за VyOS-сегментацією, на відміну від решти
  сервісів хоста. Прод (`docs/spec/11-prod-deploy.md`) цей ярлик не успадковує.
- `roles/provision` (створення VM/шаблону через Proxmox API) ще не перевірено проти живого
  PVE — перший реальний прогін зафіксує, що саме довелось поправити.
- Групa `docker` = фактично root на хості (стандартний компроміс Docker) — прийнятно для
  одного розробника на dev, прод (11 §2) цього не успадковує.
