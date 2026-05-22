import { Component, ChangeDetectionStrategy, signal, OnInit } from '@angular/core';
import { listBatches, listJobs, getSettings, openFolder as openFolderIpc } from '@spytfy/tauri-ipc';
import type { Batch, DownloadJob } from '@spytfy/models';

interface LibraryEntry {
  name: string;
  type: string;
  trackCount: number;
  folderPath: string;
}

@Component({
  selector: 'spytfy-library-page',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page animate-in">
      @if (loaded() && entries().length === 0) {
        <div class="empty">
          <div class="vinyl">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.75" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10" />
              <circle cx="12" cy="12" r="6" opacity="0.3" />
              <circle cx="12" cy="12" r="2" />
            </svg>
          </div>
          <h1>Your Library</h1>
          <p>Downloaded music appears here — organized automatically.</p>
        </div>
      }

      @if (entries().length > 0) {
        <div class="page-header">
          <h1>Your Library</h1>
          @if (outputRoot()) {
            <button class="folder-btn" (click)="openFolder(outputRoot())">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m22 19-8.9-8.9M2 11.5v3a2 2 0 0 0 2 2h5.5M13 2h1.5a2 2 0 0 1 2 2v1.5M19 2l3 3M2 2l20 20"/></svg>
              Open folder
            </button>
          }
        </div>
        <div class="grid">
          @for (entry of entries(); track entry.name) {
            <button class="card" (click)="openFolder(entry.folderPath)">
              <div class="card-visual">
                <div class="card-icon">
                  @if (entry.type === 'album') {
                    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
                  } @else {
                    <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.25"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>
                  }
                </div>
                <div class="play-overlay">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="#000"><polygon points="8 5 19 12 8 19 8 5"/></svg>
                </div>
              </div>
              <span class="card-name">{{ entry.name }}</span>
              <span class="card-info">{{ entry.trackCount }} track{{ entry.trackCount === 1 ? '' : 's' }} · {{ entry.type }}</span>
            </button>
          }
        </div>
      }
    </div>
  `,
  styles: `
    .page {
      padding: 32px 40px;
      max-width: 960px;
      min-height: 100%;
    }
    .page.animate-in { animation: slideUp var(--duration-slow) var(--ease-out) both; }

    .page-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 28px;
    }
    h1 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 28px;
      font-weight: 800;
      letter-spacing: -0.04em;
    }
    .folder-btn {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 16px;
      background: rgba(255, 255, 255, 0.07);
      border: none;
      border-radius: var(--radius-pill);
      color: var(--text-secondary);
      font-family: inherit;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .folder-btn:hover {
      background: rgba(255, 255, 255, 0.12);
      color: var(--text-primary);
    }

    .empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: calc(100vh - 140px);
      gap: 16px;
      text-align: center;
    }
    .vinyl {
      width: 112px;
      height: 112px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.03);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      animation: spin 20s linear infinite;
    }
    .empty h1 {
      font-size: 24px;
      margin-bottom: 0;
    }
    .empty p { font-size: 15px; color: var(--text-muted); max-width: 300px; line-height: 1.5; }

    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
      gap: 16px;
    }
    .card {
      display: flex;
      flex-direction: column;
      gap: 12px;
      padding: 16px;
      background: rgba(255, 255, 255, 0.04);
      border: none;
      border-radius: var(--radius-lg);
      cursor: pointer;
      transition: all var(--duration-normal) var(--ease-out);
      text-align: left;
      font-family: inherit;
      color: var(--text-primary);
    }
    .card:hover {
      background: rgba(255, 255, 255, 0.08);
    }
    .card-visual {
      position: relative;
      width: 100%;
      aspect-ratio: 1;
      border-radius: var(--radius-md);
      background: var(--surface-600);
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
    }
    .card-icon {
      color: var(--text-muted);
      transition: color var(--duration-fast) var(--ease-out);
    }
    .card:hover .card-icon { color: var(--text-secondary); }
    .play-overlay {
      position: absolute;
      right: 8px;
      bottom: 8px;
      width: 40px;
      height: 40px;
      border-radius: 50%;
      background: var(--accent);
      display: flex;
      align-items: center;
      justify-content: center;
      opacity: 0;
      transform: translateY(8px);
      transition: all var(--duration-normal) var(--ease-out);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    }
    .card:hover .play-overlay {
      opacity: 1;
      transform: translateY(0);
    }
    .card-name {
      font-size: 14px;
      font-weight: 600;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .card-info {
      font-size: 13px;
      color: var(--text-muted);
    }

    @keyframes spin { to { transform: rotate(360deg); } }

    @media (prefers-reduced-motion: reduce) {
      .vinyl { animation: none; }
      .page.animate-in { animation: none; }
      .play-overlay { transition: opacity var(--duration-fast); transform: none; }
    }
  `,
})
export class LibraryPage implements OnInit {
  entries = signal<LibraryEntry[]>([]);
  outputRoot = signal('');
  loaded = signal(false);

  async ngOnInit() {
    try {
      const [batches, jobs, settings] = await Promise.all([
        listBatches(100),
        listJobs(),
        getSettings(),
      ]);
      this.outputRoot.set(settings.outputRoot);

      const entries: LibraryEntry[] = batches
        .filter(b => b.state === 'complete')
        .map(batch => {
          const batchJobs = jobs.filter(j => j.batchId === batch.id);
          const doneCount = batchJobs.filter(j => j.state === 'done' || j.state === 'done_warning').length;
          return {
            name: batch.name,
            type: batch.sourceType,
            trackCount: doneCount,
            folderPath: settings.outputRoot + '\\' + batch.name,
          };
        })
        .filter(e => e.trackCount > 0);

      this.entries.set(entries);
    } catch {}
    this.loaded.set(true);
  }

  async openFolder(path: string) {
    try { await openFolderIpc(path); } catch {}
  }
}
