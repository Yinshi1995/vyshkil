# Frontend: Leptos → React + shadcn/ui migration

**Date**: 2026-10-02
**Status**: In progress

## Decision

Migrate the frontend from Leptos 0.7 (SSR + WASM) to React 19 + TypeScript 6 + Vite 8 + Tailwind CSS 4 + shadcn/ui (base-nova style). The Axum backend remains; Leptos `#[server]` functions are replaced by JSON API handlers in `server/src/api.rs`.

## Motivation

- 103 MB WASM bundle → 583 KB JS (185 KB gzip): massive load-time improvement
- React ecosystem: richer component library, faster iteration, easier hiring
- shadcn/ui base-nova style with military theme (Oswald headings, gold gradients, chamfered clip-path) — user-confirmed visual direction
- Leptos WASM had LNK2019/LNK1140 linking issues on Windows (why dev moved to Linux VM)

## Architecture

- `web/` — React app (Vite), runs on `:5174` in dev
- `server/src/api.rs` — JSON API at `/api/*` (same repo/policy as #[server] functions)
- `server/src/spa.rs` — serves `web/dist/` from Axum when built (production mode)
- Vite proxy (`/api → :3000`) in dev
- Auth: cookie-based sessions, same as before
- Fonts: self-hosted Oswald + Roboto in `web/public/fonts/` (no CDN per CLAUDE.md)

## Coexistence

Leptos SSR routes remain functional for now. If `web/dist/index.html` exists, Axum serves React; otherwise falls back to Leptos SSR. Both share the same `api.rs` endpoints.

## Migration status (2026-10-02)

- [x] React project scaffolding (web/)
- [x] Military visual theme
- [x] Auth flow (login, change-password, session cookie)
- [x] Dashboard (home page with stats)
- [x] Org detail page
- [x] Discrepancies page
- [x] Settings page (account + admin user management)
- [x] JSON API layer in api.rs (18 endpoints → 30+ endpoints)
- [x] Documents page (D1–D6 generation with real API)
- [x] Import page (file type selection, drop zone, recent submissions)
- [x] Training form page (groups table, file import UI)
- [x] Production SPA serving from Axum (spa.rs)
- [ ] File upload/parse endpoints for import (multipart)
- [ ] Training form: grid editor with autosave, undo/redo
- [ ] Remove Leptos dependency entirely
