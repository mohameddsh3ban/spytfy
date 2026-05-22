# Phase 1 — Scaffold | Discussion Notes

**Date**: 2026-05-19

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Nx | Integrated monorepo, @nx/angular | Smart builds, caching, boundary rules. Scales for v2 apps. |
| Angular | v20 | User's stack. Signals, standalone, OnPush. |
| Tailwind | v4 | @theme CSS-first, faster engine. |
| Sidebar | Collapsible icon rail | Icons ↔ labels. Spec §11 compliant. |
| SQLite migrations | sqlx::migrate!() | Embedded SQL, versioned, standard. |
| Package manager | pnpm | Fast, strict, disk-efficient. |
| Settings scope | Output folder only | Minimal for Phase 1, expand later. |
| Theme | Dark only | Spotify-adjacent aesthetic. |

## Nx Workspace Structure

```
spytfy/
├── apps/
│   └── desktop/                ← Angular 20 app (Tauri webview)
├── libs/
│   ├── ui/                     ← shared UI components
│   ├── models/                 ← TypeScript interfaces (Job, Batch, Settings)
│   └── tauri-ipc/              ← typed IPC wrappers
├── src-tauri/                  ← Rust backend (Tauri convention)
│   ├── src/
│   ├── migrations/
│   └── Cargo.toml
├── nx.json
├── package.json
└── pnpm-workspace.yaml
```

## Deliverables

1. Nx workspace with @nx/angular, pnpm, Tailwind v4
2. Tauri 2.x project with Rust backend (Cargo.toml: tokio, sqlx, serde, uuid)
3. Angular 20 app in apps/desktop — standalone components, signals
4. libs/models, libs/ui, libs/tauri-ipc scaffolded
5. Tailwind v4 dark-only theme tokens
6. Collapsible sidebar: Input, Downloads, Library, Settings
7. SQLite + sqlx migrations (jobs, batches, settings tables)
8. Settings page: output folder picker via Tauri file dialog → SQLite
9. IPC: get_settings + update_settings commands working end-to-end
