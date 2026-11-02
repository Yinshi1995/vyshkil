---
name: qa-uat
description: QA test suite runner for web apps (Playwright MCP) and React Native apps (Maestro). Explores the codebase to generate project-specific test task files, executes them via browser or mobile automation, and writes structured UAT reports matching executive-ready format. Supports selective runs, resume after interruption, and .docx export. Use when user wants to generate a UAT test suite and an executive-ready UAT report for Вишкіл, or invokes /qa-uat. (Для щоденного прогону перед пушем є скіл `qa`.)
---

# QA Runner (UAT) — адаптовано для Вишкілу

Джерело: github.com/emadbaqeri/claude-qa-skill (перевірено, змінено лише назву й обмеження нижче).

**Обмеження проєкту (CLAUDE.md, без винятків):**
- Тести — лише на стенді з вимисленими даними (закриті найменування `А####`); ніколи на проді, ніколи з даними ДСК.
- У `qa/tasks`, `qa/results` і скріншотах — жодних відкритих назв частин, місць навчання, позивних, ПІБ.
- Мобільний режим (Maestro) для Вишкілу не застосовується — лише web (Playwright MCP).
- Результати (`qa/results/`) не комітити, якщо там є скріншоти реальних даних.
- Для облікових даних тестових акаунтів — лише демо-стенд (`admin` / демо-логіни), не прод.


## Invocation

```
/qa                           # auto-detect: setup if no qa/tasks/, run all if exists
/qa setup                     # (re)generate qa/tasks/ from codebase analysis
/qa <topic>                   # focused setup: generate + run task files for one topic area
/qa run                       # run full suite
/qa run 01 04 14              # run specific tasks by number
/qa run auth users            # run tasks whose filename matches
/qa run --resume              # skip tasks that already have a result file
/qa export <file.md>          # convert a results .md to .docx

# Mobile platform flags (override auto-detect)
/qa --ios                     # force iOS mobile mode
/qa --android                 # force Android mobile mode
/qa --ios --android           # run both platforms

# Combine with other commands
/qa run --ios
/qa payments --android
```

---

## Project Type Detection

Before any flow, determine the project type:

**React Native** — if ALL of the following:
- `ios/` directory exists
- `android/` directory exists
- `package.json` contains `"react-native"`

**Web** — otherwise (or if `--web` flag passed explicitly)

Platform flags (`--ios`, `--android`) always force mobile mode regardless of project structure.

When mobile mode is active, the **Maestro run flow** replaces the Playwright run flow. Everything else (task file generation, result format, export) remains the same.

---

## Auto-detect (bare `/qa`)

- `qa/tasks/` does not exist → **Setup flow**
- `qa/tasks/` exists → **Run flow** (full suite)

---

## Recognising `/qa <topic>`

If the argument is not one of the reserved keywords (`setup`, `run`, `export`) and does not look like a flag or file path, treat it as a **topic** and enter the **Focused setup flow**.

Examples that trigger focused setup:
- `/qa authentication & authorization`
- `/qa payments`
- `/qa user roles`
- `/qa onboarding flow`

---

## Mobile Prerequisites Check

Run this check before any mobile setup or run flow:

1. **Maestro installed?** — `maestro --version`
   - If not: `curl -Ls "https://get.maestro.mobile.dev" | bash` (prompt user to run this)
2. **iOS target** — `xcrun simctl list devices | grep Booted`
   - If no booted simulator: list available and ask user which to boot, or `xcrun simctl boot <UDID>`
3. **Android target** — `adb devices`
   - If no device listed: prompt user to start emulator from Android Studio
4. **App bundle ID** — check `ios/<AppName>.xcodeproj` Info.plist or `android/app/build.gradle` for `applicationId`
5. **App installed on target?** — `maestro test` will install from `--app-file` if provided; otherwise app must already be installed

Report what's ready and what's missing before proceeding.

---

## Focused Setup Flow

Triggered by `/qa <topic>`. Generates task files scoped to one area and immediately runs them.

**Step 1 — Explore for the topic.** Read only what's relevant:
- Files whose name or path relates to the topic (e.g. `auth`, `login`, `session`, `permission`, `middleware`, `guard`, `role`)
- **Web:** middleware, guards, decorators, route definitions, models/schemas, seed files
- **Mobile:** screens/components in the topic area, navigation config (React Navigation stack/tab definitions), API calls in hooks/services, any `testID` props already on elements

**Step 2 — Infer.** Build a mental model limited to the topic:
- What flows exist
- What's enforced server-side vs client-side (web) / what's gated by permissions or feature flags (mobile)
- What user types / roles interact with this area
- **Mobile extra:** which navigator (stack/tab/drawer) governs this area; any platform-specific branching

**Step 3 — Gap interview.** Ask ONLY what code cannot reveal:
- Test account credentials if not in seed/fixture files
- **Mobile:** bundle ID if not determinable from config; simulator/device to target; whether app must be built first or is already installed
- Anything genuinely ambiguous after exploration

**Step 4 — Confirm.** Present your plan:
> "Here's what I'll generate for [topic]: [list of task files + test cases + accounts + base URL / bundle ID + platform]. Does this look right?"

**Step 5 — Generate.** Write only the task files relevant to the topic into `qa/tasks/`. Use existing file numbering if `qa/tasks/` already contains files (pick the next available number); otherwise start from `01`.

**Step 6 — Run.** Immediately execute the generated task files using the Run flow. Write results to `qa/results/`.

---

## Setup Flow

**Step 1 — Explore.** Read without asking:

**Web:**
- Routes / page components (what pages exist)
- Auth middleware (how login works, is there a dev bypass?)
- User / role / permission models (how many user types, what roles exist)
- Seed files, fixtures, `.env.example`, README

**Mobile (React Native):**
- Navigation config — `RootNavigator`, `AppNavigator`, tab/stack/drawer definitions (usually in `src/navigation/`)
- Screen inventory — `src/screens/` or `src/features/` (what screens exist)
- Auth flow — token storage (`AsyncStorage`, `SecureStore`, `Keychain`), auth state management (Redux/Zustand/Context)
- User roles / permissions — any role-gating on screens or features
- `app.json` / `app.config.js` — bundle ID, scheme for deep links
- `.env.example` or `config.ts` — API base URL, feature flags
- `testID` usage — how many components have `testID` props (affects selector strategy)
- README / CONTRIBUTING for build and run instructions

**Step 2 — Infer.** Build a mental model:

**Web:**
- Base URL (from env/config)
- Auth method + dev bypass URL (if any)
- User types and their permission levels
- Page inventory → classify each page by pattern (see [REFERENCE.md](REFERENCE.md#web-page-patterns))

**Mobile:**
- Bundle ID and deep link scheme
- Auth method (email/password, social, biometric, OTP)
- User types and their permission levels
- Screen inventory → classify each screen by pattern (see [REFERENCE.md](REFERENCE.md#mobile-screen-patterns))
- Platform differences (iOS-only features, Android-only features)

**Step 3 — Gap interview.** Ask ONLY about what code cannot reveal:

**Web:**
- Seeded test account emails and their roles
- Any pages out of scope for this run
- Anything genuinely ambiguous

**Mobile:**
- Test account credentials
- Which platform(s) to target (iOS simulator, Android emulator, or both)
- Whether the app needs to be built first, or is already installed on the target
- Any screens explicitly out of scope

**Step 4 — Confirm.** Present your full understanding before writing:
> "Here's what I'll generate: [list of task files + accounts + base URL or bundle ID + platform]. Does this look right?"

**Step 5 — Generate.** Write `qa/tasks/` using patterns from [REFERENCE.md](REFERENCE.md).

Always generate (in this order):
1. `01-auth.md` — login, logout, route/screen protection, unauthenticated redirects/navigation, session expiry
2. `02-navigation.md` — tab/stack/drawer rendering, permission-gated items, active states, back behavior
3. Feature task files — one per major area, numbered from `03`, pattern from REFERENCE.md
4. `XX-cross-cutting.md` (always last) — error handling, empty states, loading states, offline behavior

---

## Run Flow

### Web (Playwright MCP)

For each task file (in order, or filtered by args):
1. Navigate → interact → `browser_snapshot` → assess pass/fail/skip
2. On **FAIL**: `browser_take_screenshot` → save to `qa/results/screenshots/` → note exact reason
3. On **SKIP**: note why (missing data, feature not built, N/A for this project)
4. Never stop on a single failure — complete all cases in the task
5. After each task: write result to `qa/results/` in UAT report format (see [REFERENCE.md](REFERENCE.md#result-format))

After all tasks complete:
- Write `qa/results/00-summary.md`
- If a previous run exists, diff and flag regressions (PASS → FAIL) and fixes (FAIL → PASS)

---

### Mobile (Maestro)

**Before starting:** run the [Prerequisites Check](#mobile-prerequisites-check).

For each task file (in order, or filtered by args):

1. **Read** the task file to understand what to test.
2. **Write** a Maestro YAML flow to `qa/flows/NN-name.yaml`. See [REFERENCE.md](REFERENCE.md#maestro-flow-templates) for templates.
3. **Run** the flow:
   ```bash
   maestro test qa/flows/NN-name.yaml
   ```
   Add `--device <udid>` if targeting a specific simulator, or run twice for iOS + Android.
4. **Assess** from exit code and stdout:
   - Exit 0 → all steps passed
   - Non-zero → parse stdout for the first failing step and exact error
5. On **FAIL**: Maestro auto-captures a screenshot on failure. Find it in `~/.maestro/tests/<timestamp>/` and copy to `qa/results/screenshots/`.
   - To capture screenshots at specific checkpoints within a flow, add `- takeScreenshot: <name>` steps.
6. On **SKIP**: note why in the result file (element not found, feature not built, simulator-only limitation)
7. Never stop on a single failure — continue all cases (split into multiple Maestro flows per task if needed).
8. After each task: write result to `qa/results/` in UAT report format.

**Multi-platform runs** (`--ios --android`):
- Generate one flow file per task (Maestro flows are platform-agnostic for most interactions)
- Run the flow against iOS simulator, then Android emulator
- Write separate result files: `qa/results/NN-name-ios.md` and `qa/results/NN-name-android.md`
- Summary notes any platform-divergent behavior

After all tasks complete:
- Write `qa/results/00-summary.md`
- Include device/platform metadata in the header (see [REFERENCE.md](REFERENCE.md#mobile-result-format))
- If a previous run exists, diff and flag regressions and fixes

---

### Resume

`/qa run --resume`: check `qa/results/` for existing result files. Skip any task whose result file already exists. Start from the first task missing a result file.

---

## Export Flow

`/qa export <file.md>`:
1. Read the markdown file
2. Run the Python export script from [REFERENCE.md](REFERENCE.md#export-script)
3. Save output as `<same-name>.docx` in the same directory

If `python-docx` is not installed: `pip install python-docx` first.
If `pandoc` is available: prefer `pandoc -f markdown -t docx -o <output>.docx <input>.md`.
