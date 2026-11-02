# QA Runner — Reference

---

## Web Page Patterns

Classify each discovered route into one of these patterns, then instantiate the matching test template.

---

### Pattern: CRUD List Page

**Triggers:** A page with a table of records + create/edit/delete actions.
**Examples:** Users, Roles, Clients, Knowledge Documents, Access Requests

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Page loads with table (correct columns visible) |
| 2 | Search | Search input filters table to matching rows |
| 3 | Filter | Each filter dropdown correctly narrows results |
| 4 | Combined filters | Multiple filters apply simultaneously (AND logic) |
| 5 | Clear filters | Reset button clears all filters and restores full list |
| 6 | Pagination | Next/prev page shows different rows |
| 7 | Empty state | Zero-result filter shows friendly empty state, not blank |
| 8 | Open create modal | Create button opens empty form |
| 9 | Create — success | Fill valid data → submit → success toast → row appears |
| 10 | Create — duplicate | Submit duplicate unique field → clear error shown |
| 11 | Create — missing required | Submit empty form → field validation errors shown |
| 12 | Create — invalid format | Submit malformed field (e.g. bad email) → format error |
| 13 | Cancel create | Open modal → fill data → cancel → no record created |
| 14 | Edit — pre-filled | Edit button opens modal pre-filled with current data |
| 15 | Edit — save | Change a field → save → table reflects new value immediately |
| 16 | Edit — cancel | Change fields → cancel → original data unchanged |
| 17 | Delete — success | Delete unused record → confirm → removed from list |
| 18 | Delete — blocked | Delete record in use → error explaining why |
| 19 | Permission: view-only | User without edit permission sees no Edit/Delete buttons |
| 20 | Permission: create | User without create permission sees no Create button |
| 21 | Table refresh | After mutation, table updates without manual page reload |

---

### Pattern: Settings Page

**Triggers:** A page with configuration fields, toggles, and a Save button (no row-level CRUD).
**Examples:** AI Settings, Follow-up Config, System Settings

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Page loads with all settings sections visible |
| 2 | Save disabled | Save button is disabled when no changes have been made |
| 3 | Save enabled | Save button becomes active immediately after any change |
| 4 | Persist — text field | Change text field → save → reload → change persists |
| 5 | Persist — toggle | Toggle a switch → save → reload → state persists |
| 6 | Persist — dropdown | Change dropdown → save → reload → persists |
| 7 | Invalid value | Enter invalid value (negative number, empty required field) → save blocked with error |
| 8 | Restore | Change a value → save → change it back → save again → reload → restored |
| 9 | Navigate away | Make unsaved changes → navigate away → come back → values not saved |

---

### Pattern: Approval Workflow Page

**Triggers:** A page where items have a pending/approved/rejected lifecycle and admins take approve/reject actions.
**Examples:** Access Requests, Knowledge Document review

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Page loads with list/table of items |
| 2 | Status tabs | Each status tab (Pending/Approved/Rejected) filters correctly |
| 3 | Search | Search filters items by name/email/title |
| 4 | Pagination | Pagination works when >1 page of items |
| 5 | Empty state | Status tab with 0 items shows friendly empty state |
| 6 | Approve | Click Approve on pending item → status changes → toast shown |
| 7 | Reject | Click Reject on pending item → status changes → toast shown |
| 8 | Already processed | Approve/Reject buttons absent or disabled on non-pending items |
| 9 | Permission: view-only | User without approve permission sees no Approve/Reject buttons |
| 10 | Permission: approve | User with approve permission sees and can use Approve/Reject |

---

### Pattern: Read-Only Dashboard

**Triggers:** A page that displays charts, stats, or aggregated data with no CRUD actions.
**Examples:** Analytics, Audit Log

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Page loads, all chart containers/stat cards visible |
| 2 | Data renders | Charts render with data and labeled axes (no blank/broken state) |
| 3 | Filter | Date range or filter controls update displayed data |
| 4 | Empty range | Selecting a period with no data shows empty state, not crash |
| 5 | Loading state | Skeleton/spinner visible during data fetch |
| 6 | Drill-down | Clicking a row/item opens detail view with correct data |
| 7 | Combined filters | Multiple filters apply simultaneously |
| 8 | Clear filters | Reset restores unfiltered view |

---

## Mobile Screen Patterns

Classify each discovered screen into one of these patterns, then instantiate the matching test template. Use `testID` props as the primary selector strategy; fall back to text/label matching when `testID` is absent.

---

### Pattern: List Screen

**Triggers:** A screen backed by `FlatList` or `SectionList` displaying a collection of items.
**Examples:** Feed, Inbox, Order History, Search Results, Contacts

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Screen renders with at least one list item visible |
| 2 | Scroll down | Scroll to bottom — new items load (infinite scroll) or end indicator appears |
| 3 | Pull to refresh | Pull from top → refresh spinner → list reloads |
| 4 | Tap item | Tap a list item → correct detail screen opens |
| 5 | Search / filter | Search input narrows list to matching items |
| 6 | Clear search | Clearing search restores full list |
| 7 | Empty state | Filtered/empty list shows friendly empty state (not blank screen) |
| 8 | Loading state | Skeleton or spinner visible while data loads |
| 9 | Error state | Network error shows retry prompt, not crash |
| 10 | Swipe action | Swipe left/right on item reveals action buttons (if applicable) |
| 11 | Permission: write | Add/create button absent for view-only user |

---

### Pattern: Auth Screen

**Triggers:** Login, registration, OTP, password reset, or biometric prompt screens.
**Examples:** Login, Sign Up, Forgot Password, OTP Verification, Biometric Prompt

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Screen renders all fields and submit button |
| 2 | Valid login | Enter valid credentials → submit → lands on home/dashboard |
| 3 | Invalid credentials | Enter wrong password → inline error shown, no crash |
| 4 | Empty submit | Submit with empty fields → field validation errors shown |
| 5 | Invalid email format | Enter malformed email → format error before submission |
| 6 | Keyboard avoidance | Input fields remain visible when keyboard opens |
| 7 | Submit loading | Submit button shows loading indicator during request |
| 8 | Logout | Logout → returns to login screen, protected screens inaccessible |
| 9 | Session persistence | Close and reopen app → still logged in (if remember-me expected) |
| 10 | Token expiry | Expired token → redirected to login without crash |
| 11 | Biometric | Biometric prompt appears (if configured) → success logs in |
| 12 | Social login | Social login button visible and tappable (if applicable) |
| 13 | Forgot password | Forgot password link navigates to reset flow |

---

### Pattern: Form Screen

**Triggers:** A screen with multiple input fields, validation, and a submit action. No navigation tabs.
**Examples:** Create Post, Edit Profile, Checkout, Add Address, Feedback

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Screen renders with all form fields visible |
| 2 | Keyboard navigation | Tapping next/return moves focus to next field |
| 3 | Keyboard avoidance | Fields near bottom remain visible when keyboard opens |
| 4 | Valid submit | Fill all required fields → submit → success feedback |
| 5 | Missing required | Submit with empty required field → error shown on that field |
| 6 | Invalid format | Enter invalid format (bad email, too short) → format error |
| 7 | Submit loading | Submit button shows loading state during request |
| 8 | Double submit | Rapid double-tap on submit fires only one request |
| 9 | Cancel / back | Navigate back → no data persisted |
| 10 | Edit pre-fill | Editing existing record → fields pre-filled with current values |
| 11 | Edit save | Change a field → save → updated value reflected on return |
| 12 | Character limits | Fields with max length enforce or warn at limit |

---

### Pattern: Detail / Profile Screen

**Triggers:** A read-mostly screen showing the full data for one entity. May have Edit, Delete, or action buttons.
**Examples:** User Profile, Order Detail, Product Detail, Article, Event

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Screen renders all sections with correct data |
| 2 | Scroll | Long content scrolls without clipping |
| 3 | Back navigation | Back button / swipe returns to list screen |
| 4 | Edit action | Edit button opens edit form pre-filled with current data |
| 5 | Delete action | Delete → confirmation prompt → removed → returns to list |
| 6 | Share / copy | Share or copy button (if applicable) triggers system sheet |
| 7 | Permission: edit | Edit/Delete absent for view-only user |
| 8 | Deep link | Screen opens correctly from a deep link URL |
| 9 | Loading state | Skeleton visible while data loads |
| 10 | Error state | Network error shows retry, not blank screen |

---

### Pattern: Tab Navigation

**Triggers:** A screen with a persistent bottom tab bar (or top tabs) switching between sections.
**Examples:** Home / Explore / Messages / Profile tabs, Dashboard tabs

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | App opens on correct default tab |
| 2 | Tab switching | Tapping each tab shows the correct screen |
| 3 | Active indicator | Active tab is visually highlighted; inactive tabs are not |
| 4 | Badge | Notification badge on tab increments and clears correctly |
| 5 | State persistence | Navigate away from tab and return → scroll position and state preserved (if expected) |
| 6 | Permission-gated tab | Tab absent or disabled for user without permission |
| 7 | Android back on root | Pressing Android back on root tab does not crash; exits or shows confirm |
| 8 | iOS home indicator | Bottom tabs are not obscured by iOS home indicator |

---

### Pattern: Stack Navigation

**Triggers:** Screens pushed onto a navigation stack with a back button or back gesture.
**Examples:** Any drill-down flow: List → Detail → Edit

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Push | Tapping item navigates forward to expected screen |
| 2 | iOS back swipe | Swipe from left edge returns to previous screen |
| 3 | Android back button | Hardware/gesture back returns to previous screen |
| 4 | Header back button | Tapping header back arrow returns to previous screen |
| 5 | Stack depth | Navigating deep (3+ levels) then back unwinds correctly |
| 6 | Header title | Header shows correct title for each screen |
| 7 | Reset on logout | Logging out clears the navigation stack (no back to protected screen) |

---

### Pattern: Settings Screen (Mobile)

**Triggers:** A screen with toggles, pickers, account info, and no row-level CRUD.
**Examples:** App Settings, Notification Preferences, Account, Privacy

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Load | Screen renders all settings sections |
| 2 | Toggle — persist | Toggle a switch → background/foreground app → setting retained |
| 3 | Picker — persist | Change a picker value → background/foreground → value retained |
| 4 | Logout | Logout button in settings clears session and navigates to login |
| 5 | Account info | Correct user name/email displayed |
| 6 | Notification toggle | Enabling push notifications triggers system permission prompt (first time) |
| 7 | Link to external | "Privacy Policy" / "Terms" links open in browser or in-app webview |
| 8 | App version | App version displayed and matches expected build |

---

### Pattern: Onboarding Flow

**Triggers:** Multi-step screens shown to new users before reaching the main app.
**Examples:** Welcome slides, Permissions setup, Profile completion wizard

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | First launch | Fresh install opens onboarding, not main app |
| 2 | Step progression | Next button advances to next step |
| 3 | Back within flow | Back navigates to previous step (if allowed) |
| 4 | Skip | Skip button (if present) bypasses optional steps |
| 5 | Swipe between steps | Swiping advances/retreats steps (if carousel-style) |
| 6 | Permission prompt | Notification/location permission prompt appears at correct step |
| 7 | Complete | Completing all steps lands on main app screen |
| 8 | Re-open after partial | Killing app mid-onboarding and reopening resumes at correct step (or restarts) |
| 9 | Returning user | Returning user (token present) skips onboarding entirely |

---

### Pattern: Modal / Bottom Sheet

**Triggers:** An overlay screen (full-screen modal, action sheet, or bottom sheet) triggered from another screen.
**Examples:** Filter sheet, Confirmation dialog, Share sheet, Image picker, Date picker

**Test cases to generate:**

| # | Category | Test case |
|---|---|---|
| 1 | Open | Trigger opens the modal/sheet |
| 2 | Dismiss — swipe | Swipe down dismisses the sheet (bottom sheet) |
| 3 | Dismiss — backdrop | Tapping outside the sheet dismisses it (if applicable) |
| 4 | Dismiss — button | Close/Cancel button dismisses without action |
| 5 | Action — confirm | Confirming action produces correct result and closes sheet |
| 6 | Action — cancel | Canceling produces no side effects |
| 7 | Underlying screen | Underlying screen is not interactive while modal is open |
| 8 | Keyboard in sheet | Text input inside sheet stays visible when keyboard opens |

---

## Web Universal Test Cases

These go into `01-auth.md`, `02-navigation.md`, and the final `XX-cross-cutting.md` for web projects.

### Auth (01-auth.md)

- Valid login → lands on dashboard/home, not error
- Invalid/expired token → redirected to /login
- Already logged in visiting /login → redirected away
- Logout clears session → subsequent admin route access redirected to /login
- Unauthenticated direct URL to protected route → redirected to /login
- Non-admin visits admin route → redirected to dashboard
- Admin visits /admin → redirected to first accessible page (not stuck on /admin)
- Limited-permission admin visits unauthorized page → redirected away

### Navigation (02-navigation.md)

- Full-permission user sees all nav items
- Limited-permission user sees only permitted nav items
- Active route is visually highlighted
- Inactive route highlighting clears on navigation
- Sidebar collapse state persists across page reload
- "Back to Dashboard" / home link works
- Logout from nav/sidebar clears session

### Cross-Cutting (last task file)

**Permission rendering:**
- View-only user sees no mutating action buttons (Edit, Delete, Deactivate)
- Sidebar only shows permitted items

**Error handling:**
- 404 route → not-found page or redirect (no white screen)
- API 400 validation error → surfaced to user with descriptive message (no "Something went wrong")
- Expired session mid-use → redirect to /login without crash

**Forms:**
- Required fields validated on submit
- Whitespace-only input rejected
- XSS input (`<script>alert('XSS')</script>`) rendered as literal text, no alert fires
- Double submit prevented — only one API request fires
- Cancel discards all changes

**Tables:**
- Pagination works in both directions (next + prev)
- Empty filter result shows friendly state
- Table refreshes after mutation without manual reload

**Loading states:**
- Skeleton/spinner visible on initial page load
- Submit button shows loading state during submission
- Table shows loading indicator during filter change

---

## Mobile Universal Test Cases

These go into `01-auth.md`, `02-navigation.md`, and `XX-cross-cutting.md` for React Native projects.

### Auth (01-auth.md)

- Valid credentials → navigates to home screen
- Invalid credentials → inline error, stays on login screen, no crash
- Expired token (force-expired via API or AsyncStorage clear) → redirected to login
- Unauthenticated deep link → redirected to login, then to intended screen after login (if supported)
- Logout clears token → protected screens inaccessible via back gesture
- Session persistence: close and reopen → still logged in (if expected)
- Biometric login (if configured) → success navigates to home

### Navigation (02-navigation.md)

- All tabs accessible to full-permission user
- Permission-gated tabs absent or disabled for restricted user
- Active tab indicator is correct on each tab
- iOS: swipe-back gesture works on stack screens
- Android: hardware back button works on stack screens; on root screen either exits app or shows exit confirm
- Logging out resets navigation stack (back gesture cannot reach protected screens)
- Deep links route to correct screen

### Cross-Cutting (last task file)

**Permissions (OS-level):**
- First push notification trigger shows system permission prompt
- Denying permission shows graceful fallback (no crash)
- Camera/gallery permission prompt appears before access

**Offline / network errors:**
- No network → friendly offline banner or error message, no crash
- Retry after reconnect works correctly

**Platform behavior:**
- iOS safe area: content not clipped by notch or home indicator
- Android: status bar color matches app theme; no overlap with navigation bar
- Keyboard: inputs visible above keyboard on both platforms

**Performance / stability:**
- App does not crash on rapid navigation (stress-tap tabs)
- Memory: navigating through 10+ screens and back shows no blank screens

**Forms:**
- Double-tap submit fires only one request
- Required fields validated before API call
- Cancel / back discards unsaved data

**Loading states:**
- Skeleton or spinner visible on initial screen load
- Submit buttons show loading state and are disabled during request
- Pull-to-refresh spinner visible while refreshing

---

## Device Management

### iOS Simulator

```bash
# List available simulators
xcrun simctl list devices

# List only booted simulators
xcrun simctl list devices | grep Booted

# Boot a simulator by UDID
xcrun simctl boot <UDID>

# Open Simulator app (to see the booted device)
open -a Simulator

# Install an .app bundle (from build output)
xcrun simctl install booted path/to/MyApp.app

# Launch installed app
xcrun simctl launch booted com.company.appname

# Capture screenshot
xcrun simctl io booted screenshot qa/results/screenshots/screenshot.png
```

### Android Emulator

```bash
# List available AVDs
emulator -list-avds

# Start an emulator
emulator -avd <AVD_NAME> &

# List connected devices/emulators
adb devices

# Install APK
adb install -r android/app/build/outputs/apk/debug/app-debug.apk

# Launch app
adb shell am start -n com.company.appname/.MainActivity

# Capture screenshot
adb exec-out screencap -p > qa/results/screenshots/screenshot.png
```

### Maestro Setup

```bash
# Install Maestro
curl -Ls "https://get.maestro.mobile.dev" | bash

# Verify installation
maestro --version

# Run a single flow
maestro test qa/flows/01-auth.yaml

# Run a flow against a specific device (iOS)
maestro --device <iOS_UDID> test qa/flows/01-auth.yaml

# Run a flow against Android
maestro --device <ANDROID_SERIAL> test qa/flows/01-auth.yaml

# Run all flows in a directory
maestro test qa/flows/

# Export JUnit XML results
maestro test qa/flows/01-auth.yaml --format junit --output qa/results/01-auth-junit.xml
```

---

## Maestro Flow Templates

Use these as starting points when generating `qa/flows/NN-name.yaml` files.

### Template: Login Flow

```yaml
appId: com.company.appname
---
- launchApp:
    clearState: true

# Enter credentials
- tapOn:
    id: "email-input"       # testID="email-input", or use text: "Email"
- inputText: "user@test.com"
- tapOn:
    id: "password-input"
- inputText: "Password123!"

# Submit
- tapOn:
    id: "login-button"

# Assert success
- assertVisible:
    text: "Home"            # or id: "home-screen"
- takeScreenshot: login-success
```

### Template: Navigate and Assert

```yaml
appId: com.company.appname
---
- launchApp: {}

# Already logged in — navigate to Settings tab
- tapOn:
    id: "tab-settings"

# Assert screen loaded
- assertVisible:
    id: "settings-screen"

# Interact with a toggle
- tapOn:
    id: "notifications-toggle"

# Assert toggle changed (check label or state)
- assertVisible:
    text: "Notifications enabled"

- takeScreenshot: notifications-enabled
```

### Template: List → Detail

```yaml
appId: com.company.appname
---
- launchApp: {}

# Navigate to list screen
- tapOn:
    id: "tab-orders"

# Assert list loaded
- assertVisible:
    id: "order-list"

# Scroll to find an item (if needed)
- scrollUntilVisible:
    element:
      text: "Order #1234"
    direction: DOWN

# Tap item to open detail
- tapOn:
    text: "Order #1234"

# Assert detail screen
- assertVisible:
    id: "order-detail-screen"
- assertVisible:
    text: "Order #1234"

- takeScreenshot: order-detail
```

### Template: Form Fill and Submit

```yaml
appId: com.company.appname
---
- launchApp: {}

# Navigate to form
- tapOn:
    id: "create-button"

# Fill form fields
- tapOn:
    id: "name-input"
- clearText
- inputText: "Test Item"

- tapOn:
    id: "description-input"
- clearText
- inputText: "Test description for QA"

# Submit
- tapOn:
    id: "submit-button"

# Assert success
- assertVisible:
    text: "Item created"    # success toast or confirmation text

- takeScreenshot: form-submit-success
```

### Template: Handle System Permission Dialog

```yaml
appId: com.company.appname
---
- launchApp:
    clearState: true

# Navigate to feature that triggers permission
- tapOn:
    id: "enable-notifications-button"

# System permission dialog appears — Maestro handles it
- allowPermission           # iOS: taps "Allow" on system dialog
# For Android: same command works
# To deny: - denyPermission

# Assert app handled the permission
- assertVisible:
    id: "notifications-enabled-banner"

- takeScreenshot: permission-granted
```

### Template: Scroll and Verify Empty State

```yaml
appId: com.company.appname
---
- launchApp: {}

# Navigate to list
- tapOn:
    id: "tab-inbox"

# Apply a filter that returns no results
- tapOn:
    id: "filter-button"
- tapOn:
    text: "Archived"

# Scroll to bottom to confirm no items
- scroll

# Assert empty state visible
- assertVisible:
    id: "empty-state"       # or text: "No messages"

- takeScreenshot: empty-state-archived
```

### Template: Back Navigation and Stack Reset

```yaml
appId: com.company.appname
---
- launchApp: {}

# Go deep into the stack
- tapOn:
    id: "tab-home"
- tapOn:
    id: "first-item"        # pushes Detail screen
- tapOn:
    id: "edit-button"       # pushes Edit screen

# Back twice via header back button
- tapOn:
    id: "back-button"
- assertVisible:
    id: "detail-screen"

- tapOn:
    id: "back-button"
- assertVisible:
    id: "home-screen"

- takeScreenshot: back-navigation-complete
```

---

## Result Format

Each task result file and the final summary follow the UAT executive report format below.

### Per-task result file (`qa/results/NN-name.md`)

```markdown
# [Project] QA — [Area Name]

**Date:** YYYY-MM-DD
**Branch:** [branch]
**Tested by:** Automated QA (Playwright / Maestro)
**Platform:** Web / iOS [device, OS version] / Android [device, OS version]
**App version:** [version + build number — mobile only]
**Scope:** [one-line description of what this area covers] — [base URL or bundle ID]
**Test accounts:** [list accounts used]

---

## Summary

**Passed:** X | **Failed:** X | **Skipped:** X | **Total:** X

---

## Test Results

| ID  | Test Case | Result |
|-----|-----------|--------|
| X.1 | ... | ✅ PASS |
| X.2 | ... | ❌ FAIL |
| X.3 | ... | ⚠️ SKIP |
| X.4 | ... | 🐛 BUG |

---

## Failures & Gaps

### [ID] — [Test case name]

[Exact description of what happened, what was expected, what was observed. Include element ID, screen name, or Maestro error if relevant.]

---

## Screenshots

Screenshots saved to `qa/results/screenshots/` — filenames listed here if any were taken.
```

---

### Summary file (`qa/results/00-summary.md`)

```markdown
# [Project] QA — UAT Execution Report

**Date:** YYYY-MM-DD
**Branch:** [branch]
**Reported by:** [name from git config or ask]
**Tested by:** Automated QA (Playwright / Maestro)
**Platform:** Web / iOS [device, OS] / Android [device, OS]
**App version:** [version + build — mobile only]
**Scope:** [description of what was covered] — [base URL or bundle ID]
**Test accounts:** [list]

---

## Executive Summary

[2–3 sentences: overall verdict, any critical bugs found, overall health signal.]

|                      | Count |
|----------------------|-------|
| Tests passed         | X     |
| Bugs found           | X     |
| Gaps identified      | X     |
| Blockers             | X     |
| Total tests executed | X     |

---

## Issues Found

### Fix Before Release (Critical)

| # | Area | Issue |
|---|------|-------|
| BUG-001 | [Area] | [Description — root cause if known] |

### Fix Soon (High Priority)

| # | Area | Issue |
|---|------|-------|

### Informational (Low)

| # | Area | Issue | Resolution |
|---|------|-------|------------|
| GAP-001 | [Area] | [Description] | [By design / not implemented / etc.] |

*(Omit any section that has no entries.)*

---

## Area NN — [Name]

**Scope:** [What this area tests]

| ID  | Test Case | Result |
|-----|-----------|--------|
| N.1 | ...       | PASS   |

**Passed:** X | **Failed:** X | **Skipped:** X

**Failed test details:** *(omit if none)*

- **[ID] — [Name].** [Exact description. Root cause if known. PR/fix reference if resolved.]

---

## Regression Diff *(omit if first run)*

| Task | Test | Previous | This Run | Type |
|------|------|----------|----------|------|
| 04-users | U11 | PASS | FAIL | 🔴 Regression |
| 06-knowledge | K7 | FAIL | PASS | 🟢 Fixed |

---

## What's Working Well

- [Area]: [one-line description of what's solid]

---

## Testing Methodology

[Paragraph: tool used (Playwright for web / Maestro for mobile), number of test cases, areas covered, auth method, device/OS targets, any notable constraints or deviations from the test plan.]
```

---

## Export Script

Use this when `/qa export <file.md>` is invoked. Prefer `pandoc` if available; fall back to `python-docx`.

### Option A — pandoc (preferred)

```bash
# Check if pandoc is available
pandoc --version 2>/dev/null && \
  pandoc -f markdown -t docx \
    --reference-doc ~/.claude/skills/qa/reference.docx \
    -o "${INPUT%.md}.docx" "$INPUT" \
  || echo "pandoc not found, falling back to python-docx"
```

### Option B — python-docx fallback

Write and run this Python script:

```python
#!/usr/bin/env python3
"""Convert a QA results markdown file to a formatted .docx"""
import sys, re
from pathlib import Path

try:
    from docx import Document
    from docx.shared import Pt, RGBColor
    from docx.enum.text import WD_ALIGN_PARAGRAPH
except ImportError:
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "python-docx"])
    from docx import Document
    from docx.shared import Pt, RGBColor

def convert(md_path: str):
    src = Path(md_path)
    lines = src.read_text().splitlines()
    doc = Document()

    # Styles
    style = doc.styles['Normal']
    style.font.name = 'Calibri'
    style.font.size = Pt(11)

    i = 0
    while i < len(lines):
        line = lines[i]

        # Headings
        if line.startswith('# ') and not line.startswith('## '):
            p = doc.add_heading(line[2:], level=1)
        elif line.startswith('## '):
            doc.add_heading(line[3:], level=2)
        elif line.startswith('### '):
            doc.add_heading(line[4:], level=3)

        # Horizontal rule
        elif line.strip() == '---':
            doc.add_paragraph('─' * 60)

        # Table
        elif line.startswith('|'):
            rows = []
            while i < len(lines) and lines[i].startswith('|'):
                row_line = lines[i]
                if re.match(r'^\|[-| :]+\|$', row_line):
                    i += 1
                    continue
                cells = [c.strip() for c in row_line.strip('|').split('|')]
                rows.append(cells)
                i += 1
            if rows:
                table = doc.add_table(rows=len(rows), cols=len(rows[0]))
                table.style = 'Table Grid'
                for r_idx, row_data in enumerate(rows):
                    for c_idx, cell_text in enumerate(row_data):
                        cell = table.rows[r_idx].cells[c_idx]
                        cell.text = cell_text
                        if r_idx == 0:
                            for run in cell.paragraphs[0].runs:
                                run.bold = True
            continue

        # Bold metadata lines (**Key:** Value)
        elif line.startswith('**') and ':**' in line:
            p = doc.add_paragraph()
            parts = line.split(':**', 1)
            key = parts[0].lstrip('*')
            value = parts[1].rstrip('*') if len(parts) > 1 else ''
            run = p.add_run(key + ': ')
            run.bold = True
            p.add_run(value.strip())

        # Bullet points
        elif line.startswith('- '):
            doc.add_paragraph(line[2:], style='List Bullet')

        # Blank line
        elif line.strip() == '':
            doc.add_paragraph('')

        # Normal paragraph
        elif line.strip():
            doc.add_paragraph(line)

        i += 1

    out = src.with_suffix('.docx')
    doc.save(str(out))
    print(f"Saved: {out}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python export.py <file.md>")
        sys.exit(1)
    convert(sys.argv[1])
```

Save this script to a temp file and run: `python /tmp/qa_export.py <file.md>`
