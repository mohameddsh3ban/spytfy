import { Component, ChangeDetectionStrategy, input, computed, signal, OnDestroy } from '@angular/core';
import type { ResolvedInput, SpotifyTrack } from '@spytfy/models';
import { enqueueDownload, scrapePlaylistTracks, createPlaylistFromTracks } from '@spytfy/tauri-ipc';
import { Router } from '@angular/router';
import { ScreenshotModalComponent } from './screenshot-modal.component';

@Component({
  selector: 'spytfy-preview-card',
  standalone: true,
  imports: [ScreenshotModalComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="card animate-in">
      <div class="card-hero">
        @if (coverUrl()) {
          <img [src]="coverUrl()" alt="Cover" class="cover-art" />
        } @else {
          <div class="cover-placeholder">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
          </div>
        }
        <div class="hero-meta">
          <span class="type-label">{{ resolved().type }}</span>
          <h2>{{ title() }}</h2>
          <p class="artist-line">{{ subtitle() }}</p>
          <p class="stats-line">{{ trackCount() }} track{{ trackCount() === 1 ? '' : 's' }} · {{ formattedDuration() }}</p>
        </div>
      </div>

      @if (tracks().length > 1) {
        <div class="track-list">
          @for (track of tracks(); track track.id; let i = $index) {
            <div class="track-row">
              <span class="track-num">{{ i + 1 }}</span>
              @if (track.coverUrl) {
                <img [src]="track.coverUrl" alt="" class="track-thumb" />
              } @else {
                <div class="track-thumb-empty">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
                </div>
              }
              <div class="track-meta">
                <span class="track-title">{{ track.name }}</span>
                <span class="track-artist">{{ track.artists.join(', ') }}</span>
              </div>
              <span class="track-duration">{{ formatMs(track.durationMs) }}</span>
            </div>
          }
        </div>
      }

      @if (trackCount() === 0 && (resolved().type === 'playlist' || resolved().type === 'album')) {
        <div class="import-section">
          @if (scraping()) {
            <div class="scrape-status">
              <div class="progress-track">
                <div class="progress-fill" [style.width.%]="scrapePercent()"></div>
              </div>
              <p class="scrape-msg">{{ scrapeStatus() || 'Starting...' }}</p>
              <span class="scrape-pct">{{ scrapePercent() }}%</span>
              @if (showScrapeLogs()) {
                <div class="scrape-logs">
                  @for (log of scrapeLogs(); track $index) {
                    <div class="log-entry">{{ log }}</div>
                  }
                </div>
              }
              <button class="text-btn" (click)="showScrapeLogs.set(!showScrapeLogs())">
                {{ showScrapeLogs() ? 'Hide' : 'Show' }} logs
              </button>
            </div>
          }
          @if (scrapeError()) {
            <p class="error-msg">{{ scrapeError() }}</p>
          }
          <button class="action-btn primary" (click)="autoLoadTracks()" [disabled]="scraping()">
            {{ scraping() ? 'Loading...' : 'Load Tracks' }}
          </button>
          <button class="action-btn outline" (click)="showScreenshotModal.set(true)">
            Import manually
          </button>
        </div>
      }

      @if (showScreenshotModal()) {
        <spytfy-screenshot-modal
          (close)="showScreenshotModal.set(false)"
          (tracksReady)="onTracksImported($event)"
        />
      }

      <div class="card-footer">
        <button class="text-btn" (click)="showJson.set(!showJson())">
          {{ showJson() ? 'Hide' : 'Show' }} JSON
        </button>

        @if (downloadStage() === 'idle') {
          <button class="download-btn" (click)="startDownload()">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>
            </svg>
            Download{{ trackCount() === 1 ? '' : ' all ' + trackCount() }}
          </button>
        } @else if (downloadStage() === 'failed') {
          <div class="error-row">
            <span class="error-msg">{{ downloadError() }}</span>
            <button class="action-btn primary compact" (click)="startDownload()">Retry</button>
          </div>
        }
      </div>

      @if (showJson()) {
        <pre class="json-block">{{ jsonData() }}</pre>
      }
    </div>
  `,
  styles: `
    .card {
      width: 100%;
      background: var(--surface-700);
      border-radius: var(--radius-lg);
      overflow: hidden;
    }
    .card.animate-in {
      animation: scaleIn var(--duration-slow) var(--ease-out) both;
    }

    .card-hero {
      display: flex;
      gap: 20px;
      padding: 24px;
      background: linear-gradient(135deg, rgba(29, 185, 84, 0.08) 0%, transparent 60%);
    }
    .cover-art {
      width: 128px;
      height: 128px;
      border-radius: var(--radius-md);
      object-fit: cover;
      flex-shrink: 0;
      box-shadow: var(--shadow-lg);
    }
    .cover-placeholder {
      width: 128px;
      height: 128px;
      border-radius: var(--radius-md);
      background: var(--surface-600);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      flex-shrink: 0;
    }
    .hero-meta {
      display: flex;
      flex-direction: column;
      justify-content: flex-end;
      gap: 4px;
      min-width: 0;
    }
    .type-label {
      font-size: 12px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--text-secondary);
    }
    h2 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 22px;
      font-weight: 700;
      letter-spacing: -0.03em;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      line-height: 1.2;
    }
    .artist-line {
      font-size: 14px;
      color: var(--text-secondary);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .stats-line {
      font-size: 13px;
      color: var(--text-muted);
      margin-top: 2px;
    }

    .track-list {
      max-height: 280px;
      overflow-y: auto;
      padding: 4px 0;
    }
    .track-list::-webkit-scrollbar { width: 6px; }
    .track-list::-webkit-scrollbar-track { background: transparent; }
    .track-list::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.1); border-radius: 3px; }
    .track-row {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 8px 24px;
      transition: background var(--duration-fast) var(--ease-out);
    }
    .track-row:hover {
      background: rgba(255, 255, 255, 0.05);
    }
    .track-num {
      width: 20px;
      font-size: 14px;
      color: var(--text-muted);
      text-align: right;
      flex-shrink: 0;
      font-variant-numeric: tabular-nums;
    }
    .track-thumb {
      width: 40px;
      height: 40px;
      border-radius: var(--radius-sm);
      object-fit: cover;
      flex-shrink: 0;
    }
    .track-thumb-empty {
      width: 40px;
      height: 40px;
      border-radius: var(--radius-sm);
      background: var(--surface-600);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      flex-shrink: 0;
    }
    .track-meta {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    .track-title {
      font-size: 14px;
      font-weight: 500;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      color: var(--text-primary);
    }
    .track-row:hover .track-title {
      color: #fff;
    }
    .track-artist {
      font-size: 13px;
      color: var(--text-muted);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .track-duration {
      font-size: 13px;
      color: var(--text-muted);
      flex-shrink: 0;
      font-variant-numeric: tabular-nums;
    }

    .import-section {
      padding: 16px 24px 24px;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    .action-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      width: 100%;
      height: 44px;
      border-radius: var(--radius-pill);
      font-family: inherit;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      border: none;
    }
    .action-btn.primary {
      background: var(--accent);
      color: #000;
    }
    .action-btn.primary:hover:not(:disabled) {
      background: var(--accent-hover);
      transform: scale(1.02);
    }
    .action-btn.primary:disabled { opacity: 0.5; cursor: default; }
    .action-btn.outline {
      background: transparent;
      border: 1px solid var(--surface-500);
      color: var(--text-secondary);
    }
    .action-btn.outline:hover {
      border-color: var(--text-muted);
      color: var(--text-primary);
    }
    .action-btn.compact { width: auto; height: 36px; padding: 0 20px; }

    .scrape-status {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
      width: 100%;
    }
    .scrape-msg { font-size: 13px; color: var(--text-secondary); text-align: center; }
    .scrape-pct { font-size: 13px; color: var(--accent); font-weight: 700; }
    .progress-track {
      width: 100%;
      height: 4px;
      background: var(--surface-600);
      border-radius: 2px;
      overflow: hidden;
    }
    .progress-fill {
      height: 100%;
      background: var(--accent);
      border-radius: 2px;
      transition: width 300ms ease;
      animation: progress-glow 2s ease infinite;
    }
    .scrape-logs {
      width: 100%;
      max-height: 150px;
      overflow-y: auto;
      padding: 12px;
      background: var(--surface-900);
      border-radius: var(--radius-md);
      font-family: monospace;
      font-size: 11px;
      color: var(--text-muted);
    }
    .log-entry { padding: 2px 0; }
    .error-msg { font-size: 13px; color: var(--error); text-align: center; }

    .card-footer {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 16px 24px 20px;
      gap: 12px;
    }
    .download-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      height: 48px;
      padding: 0 32px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: 'Space Grotesk', sans-serif;
      font-size: 15px;
      font-weight: 700;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      margin-left: auto;
    }
    .download-btn:hover {
      background: var(--accent-hover);
      transform: scale(1.04);
    }
    .download-btn:active {
      transform: scale(0.97);
    }
    .download-btn:focus-visible {
      outline: 2px solid var(--accent);
      outline-offset: 2px;
    }
    .text-btn {
      background: none;
      border: none;
      color: var(--text-muted);
      font-family: inherit;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      padding: 4px 0;
      transition: color var(--duration-fast) var(--ease-out);
    }
    .text-btn:hover { color: var(--text-secondary); }
    .json-block {
      margin: 0 24px 20px;
      padding: 16px;
      background: var(--surface-900);
      border-radius: var(--radius-md);
      font-size: 11px;
      color: var(--accent);
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      white-space: pre-wrap;
      word-break: break-all;
      max-height: 200px;
      overflow: auto;
    }
    .error-row {
      display: flex;
      align-items: center;
      gap: 12px;
      margin-left: auto;
    }

    @media (prefers-reduced-motion: reduce) {
      .card.animate-in { animation: none; }
      .track-row { transition: none; }
    }
  `,
})
export class PreviewCardComponent implements OnDestroy {
  resolved = input.required<ResolvedInput>();
  sourceUrl = input<string>('');
  showJson = signal(false);
  showScreenshotModal = signal(false);
  scraping = signal(false);
  scrapeError = signal('');
  scrapePercent = signal(0);
  scrapeStatus = signal('');
  scrapeLogs = signal<string[]>([]);
  showScrapeLogs = signal(false);

  constructor(private router: Router) {}

  jsonData = computed(() => JSON.stringify(this.resolved(), null, 2));

  downloadStage = signal<'idle' | 'failed'>('idle');
  downloadError = signal('');

  coverUrl = computed(() => {
    const r = this.resolved();
    return r.data.coverUrl ?? null;
  });

  title = computed(() => this.resolved().data.name);

  subtitle = computed(() => {
    const r = this.resolved();
    switch (r.type) {
      case 'track': return r.data.artists.join(', ');
      case 'album': return r.data.artists.join(', ');
      case 'playlist': return `by ${r.data.owner}`;
    }
  });

  tracks = computed<SpotifyTrack[]>(() => {
    const r = this.resolved();
    switch (r.type) {
      case 'track': return [r.data];
      case 'album': return r.data.tracks;
      case 'playlist': return r.data.tracks;
    }
  });

  trackCount = computed(() => this.tracks().length);

  formattedDuration = computed(() => {
    const totalMs = this.tracks().reduce((sum, t) => sum + t.durationMs, 0);
    const totalSec = Math.floor(totalMs / 1000);
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    return h > 0 ? `${h}h ${m}m` : `${m}m`;
  });

  formatMs(ms: number): string {
    const sec = Math.floor(ms / 1000);
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  async startDownload() {
    try {
      await enqueueDownload(this.resolved(), this.sourceUrl());
      this.router.navigate(['/downloads']);
    } catch (e: any) {
      this.downloadError.set(typeof e === 'string' ? e : e?.message || 'Failed to enqueue');
      this.downloadStage.set('failed');
    }
  }

  async autoLoadTracks() {
    this.scraping.set(true);
    this.scrapeError.set('');
    this.scrapePercent.set(0);
    this.scrapeStatus.set('Starting...');
    this.scrapeLogs.set([]);

    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<{ message: string; percent: number }>('scrape:log', (e) => {
      this.scrapeStatus.set(e.payload.message);
      this.scrapePercent.set(e.payload.percent);
      this.scrapeLogs.update(logs => [...logs, `[${e.payload.percent}%] ${e.payload.message}`]);
    });

    try {
      const tracks = await scrapePlaylistTracks(this.sourceUrl());
      if (tracks.length === 0) {
        this.scrapeError.set('No tracks found. Try the manual import.');
        return;
      }
      const r = this.resolved();
      const playlistName = r.data.name || 'Playlist';
      const coverUrl = r.data.coverUrl || null;
      const playlist = await createPlaylistFromTracks(playlistName, coverUrl, tracks);
      await enqueueDownload(playlist, this.sourceUrl());
      this.router.navigate(['/downloads']);
    } catch (e: any) {
      this.scrapeError.set(typeof e === 'string' ? e : e?.message || 'Scraping failed');
    } finally {
      unlisten();
      this.scraping.set(false);
    }
  }

  onTracksImported(playlist: ResolvedInput) {
    this.showScreenshotModal.set(false);
    enqueueDownload(playlist, this.sourceUrl()).then(() => {
      this.router.navigate(['/downloads']);
    }).catch(e => {
      this.downloadError.set(typeof e === 'string' ? e : 'Failed to enqueue');
    });
  }

  ngOnDestroy() {}
}
