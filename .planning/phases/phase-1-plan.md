# Phase 1 — Scaffold | Implementation Plan

**Date**: 2026-05-19
**Estimate**: 1 day
**Prerequisites**: Node.js ≥20, pnpm, Rust toolchain, Tauri CLI v2

---

## Step 1 — Nx Workspace Init
**Time**: ~15 min

1. `npx create-nx-workspace@latest spytfy --preset=angular-monorepo --appName=desktop --style=css --routing=true --pm=pnpm --nxCloud=skip`
2. Verify workspace builds: `npx nx build desktop`
3. Confirm Angular 20 in `package.json` — upgrade if Nx scaffolds v19
4. Remove default boilerplate from `apps/desktop/src/app/`

**Output**: Clean Nx + Angular workspace, builds green.

---

## Step 2 — Tailwind v4 Setup
**Time**: ~15 min

1. `pnpm add -D tailwindcss @tailwindcss/postcss postcss`
2. Create `apps/desktop/src/styles.css`:
   ```css
   @import "tailwindcss";
   @theme {
     --color-surface-900: #0a0a0a;
     --color-surface-800: #121212;
     --color-surface-700: #1a1a1a;
     --color-surface-600: #242424;
     --color-surface-500: #2a2a2a;
     --color-accent: #1db954;
     --color-accent-hover: #1ed760;
     --color-text-primary: #ffffff;
     --color-text-secondary: #a7a7a7;
     --color-text-muted: #6a6a6a;
     --color-error: #e74c3c;
     --color-warning: #f39c12;
     --color-success: #1db954;
     --font-sans: 'Inter', system-ui, sans-serif;
   }
   ```
3. Wire PostCSS in Angular build config
4. Verify Tailwind classes render in browser

**Output**: Dark theme tokens available, Tailwind compiling.

---

## Step 3 — Tauri 2.x Init
**Time**: ~20 min

1. `pnpm add -D @tauri-apps/cli@^2`
2. `npx tauri init` inside workspace root — point `devUrl` to Angular dev server, `frontendDist` to `dist/apps/desktop/browser`
3. Edit `src-tauri/Cargo.toml` — add deps:
   ```toml
   [dependencies]
   tauri = { version = "2", features = ["tray-icon"] }
   tauri-plugin-dialog = "2"
   tauri-plugin-shell = "2"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   tokio = { version = "1", features = ["full"] }
   sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
   uuid = { version = "1", features = ["v4", "serde"] }
   chrono = { version = "0.4", features = ["serde"] }
   ```
4. Minimal `src-tauri/src/main.rs` — Tauri builder with plugin registrations
5. `cargo build` in `src-tauri/` — verify compiles
6. `npx tauri dev` — verify Angular loads inside Tauri window

**Output**: Tauri desktop window showing Angular app.

---

## Step 4 — Nx Libraries
**Time**: ~15 min

1. `npx nx g @nx/angular:library models --directory=libs/models --standalone --skipModule`
2. `npx nx g @nx/angular:library ui --directory=libs/ui --standalone --skipModule`
3. `npx nx g @nx/angular:library tauri-ipc --directory=libs/tauri-ipc --standalone --skipModule`
4. In `libs/models/src/`:
   ```typescript
   // settings.model.ts
   export interface Settings {
     outputRoot: string;
     concurrency: number;
     bitrateKbps: number;
     overwriteExisting: boolean;
     writeCoverJpg: boolean;
   }
   ```
5. In `libs/tauri-ipc/src/`:
   ```typescript
   // ipc.service.ts — typed invoke wrapper
   import { invoke } from '@tauri-apps/api/core';
   export async function getSettings(): Promise<Settings> { ... }
   export async function updateSettings(patch: Partial<Settings>): Promise<Settings> { ... }
   ```
6. Configure `tsconfig.base.json` path aliases: `@spytfy/models`, `@spytfy/ui`, `@spytfy/tauri-ipc`

**Output**: Three libs importable from `apps/desktop`.

---

## Step 5 — SQLite + Migrations
**Time**: ~20 min

1. Create `src-tauri/migrations/` directory
2. `001_initial.sql`:
   ```sql
   CREATE TABLE IF NOT EXISTS settings (
     key TEXT PRIMARY KEY,
     value TEXT NOT NULL,
     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
   );

   CREATE TABLE IF NOT EXISTS batches (
     id TEXT PRIMARY KEY,
     source_url TEXT NOT NULL,
     source_type TEXT NOT NULL CHECK (source_type IN ('track', 'album', 'playlist')),
     name TEXT NOT NULL,
     total_tracks INTEGER NOT NULL DEFAULT 0,
     state TEXT NOT NULL DEFAULT 'active',
     created_at TEXT NOT NULL DEFAULT (datetime('now')),
     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
   );

   CREATE TABLE IF NOT EXISTS jobs (
     id TEXT PRIMARY KEY,
     batch_id TEXT NOT NULL REFERENCES batches(id),
     spotify_id TEXT NOT NULL,
     title TEXT NOT NULL,
     artist TEXT NOT NULL,
     album TEXT NOT NULL,
     duration_ms INTEGER NOT NULL,
     state TEXT NOT NULL DEFAULT 'queued',
     yt_url TEXT,
     yt_score INTEGER,
     output_path TEXT,
     error TEXT,
     progress_pct REAL,
     created_at TEXT NOT NULL DEFAULT (datetime('now')),
     updated_at TEXT NOT NULL DEFAULT (datetime('now'))
   );

   CREATE INDEX idx_jobs_batch ON jobs(batch_id);
   CREATE INDEX idx_jobs_state ON jobs(state);
   ```
3. Rust `db.rs` module: init pool, run `sqlx::migrate!()`, helper for get/set settings
4. Wire DB init into Tauri setup hook — creates `.spytfy/db.sqlite` in app data dir

**Output**: DB created on app launch, tables exist, migrations versioned.

---

## Step 6 — IPC Commands (get_settings / update_settings)
**Time**: ~15 min

1. Rust `commands/settings.rs`:
   - `get_settings` — reads all rows from settings table, returns Settings struct
   - `update_settings` — upserts changed keys, returns updated Settings
2. Register commands in Tauri builder
3. Default settings seeded on first launch: `output_root` = user's Music dir
4. Test from Angular using `libs/tauri-ipc` — round-trip works

**Output**: Frontend can read/write settings via IPC.

---

## Step 7 — Sidebar + Shell Layout
**Time**: ~30 min

1. `AppComponent` — full-height flex layout: sidebar + `<router-outlet>`
2. `SidebarComponent` (standalone, OnPush, signals):
   - Collapsed state: `signal<boolean>(false)`
   - 4 nav items: Input (🔍), Downloads (⬇), Library (📚), Settings (⚙)
   - Icon-only when collapsed, icon + label when expanded
   - Toggle button at bottom
   - Active route highlighted with accent color
   - Smooth width transition (CSS `transition: width 200ms ease`)
3. Icons: use inline SVGs or `lucide-angular` (lightweight)
4. Routes in `app.routes.ts`:
   ```typescript
   { path: '', redirectTo: 'input', pathMatch: 'full' },
   { path: 'input', loadComponent: () => import('./pages/input/input.page') },
   { path: 'downloads', loadComponent: () => import('./pages/downloads/downloads.page') },
   { path: 'library', loadComponent: () => import('./pages/library/library.page') },
   { path: 'settings', loadComponent: () => import('./pages/settings/settings.page') },
   ```
5. Placeholder pages — each shows page name centered, dark background

**Output**: Working sidebar nav, 4 routes, collapse toggle, dark theme applied.

---

## Step 8 — Settings Page (Output Folder Picker)
**Time**: ~20 min

1. `SettingsPageComponent` — loads settings on init via `getSettings()`
2. "Output Folder" row: current path displayed, "Browse" button
3. Browse triggers Tauri `dialog.open({ directory: true })` → updates via `updateSettings()`
4. Success toast/indicator on save
5. Default shows user's Music directory

**Output**: User can pick output folder, persisted to SQLite, survives app restart.

---

## Step 9 — Verify & Polish
**Time**: ~15 min

1. `npx tauri dev` — full flow: app opens, sidebar works, all routes load, settings round-trip
2. `npx nx build desktop` — production build succeeds
3. `cargo test` in src-tauri — basic DB tests pass
4. Window title set to "Spytfy"
5. Minimum window size 900×600 enforced in `tauri.conf.json`

**Output**: Phase 1 complete. App skeleton running end-to-end.

---

## Dependency Graph

```
Step 1 (Nx) ──→ Step 2 (Tailwind) ──→ Step 7 (Sidebar)
     │                                       ↓
     ├──→ Step 3 (Tauri) ──→ Step 5 (SQLite) ──→ Step 6 (IPC) ──→ Step 8 (Settings)
     │
     └──→ Step 4 (Nx Libs) ──→ Step 6 (IPC)
                                                   All ──→ Step 9 (Verify)
```

**Parallelizable**: Steps 2, 3, 4 can run concurrently after Step 1.

---

## Acceptance Criteria

- [ ] `npx tauri dev` opens Spytfy window (900×600 min)
- [ ] Sidebar collapses/expands with smooth transition
- [ ] All 4 routes load (Input, Downloads, Library, Settings)
- [ ] Settings page: output folder picker works, value persists after restart
- [ ] `npx nx build desktop` succeeds
- [ ] SQLite DB created at app data dir with jobs, batches, settings tables
- [ ] `@spytfy/models`, `@spytfy/ui`, `@spytfy/tauri-ipc` importable
