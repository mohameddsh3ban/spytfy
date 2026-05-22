# Phase 4 — Queue & Concurrency | Discussion Notes

**Date**: 2026-05-20

## Decisions

| Decision | Choice |
|---|---|
| Scope | Full spec — SQLite jobs, Tokio pool, batch management |
| Downloads UI | Full queue view (§11.2) with per-track progress |
| Concurrency | 3 workers default, configurable 1-8 |

## Deliverables

1. SQLite job/batch tracking (persist across restarts)
2. Tokio worker pool (3 concurrent, configurable 1-8)
3. Batch management: pause/resume/cancel
4. Per-track events: job:state, job:progress, batch:progress, batch:complete
5. Downloads page: live queue UI, state badges, progress bars, batch headers
6. "Download All" from preview card enqueues all tracks
7. Concurrency slider in Settings
