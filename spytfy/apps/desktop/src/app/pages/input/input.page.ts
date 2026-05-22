import { Component, ChangeDetectionStrategy, signal, computed } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { PreviewCardComponent } from './preview-card.component';
import { resolveUrl, debugScrape, resolveFromJson } from '@spytfy/tauri-ipc';
import type { ResolvedInput } from '@spytfy/models';

type UrlType = 'track' | 'album' | 'playlist' | 'artist' | null;

@Component({
  selector: 'spytfy-input-page',
  standalone: true,
  imports: [FormsModule, PreviewCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page">
      <div class="content animate-in">
        <div class="hero">
          <h1>What do you want to download?</h1>
          <p class="subtitle">Paste a Spotify link — track, album, playlist, or artist</p>
        </div>

        <div class="search-bar" [class.focused]="inputFocused()" [class.has-type]="!!urlType()">
          <div class="search-icon">
            @if (urlType()) {
              <span class="type-pill">{{ urlType() }}</span>
            } @else {
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
                <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
              </svg>
            }
          </div>
          <input
            type="url"
            [ngModel]="url()"
            (ngModelChange)="onUrlChange($event)"
            (focus)="inputFocused.set(true)"
            (blur)="inputFocused.set(false)"
            (paste)="onPaste($event)"
            placeholder="https://open.spotify.com/..."
            spellcheck="false"
            autocomplete="off"
          />
          @if (url()) {
            <button class="clear-btn" (click)="clear()" aria-label="Clear">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
              </svg>
            </button>
          } @else {
            <button class="paste-pill" (click)="pasteFromClipboard()">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect width="8" height="4" x="8" y="2" rx="1" ry="1"/><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
              </svg>
              Paste
            </button>
          }
        </div>

        @if (url() && !urlType()) {
          <p class="error-hint">Not a valid Spotify URL</p>
        }

        @if (urlType() && !resolved() && !loading()) {
          <button class="fetch-btn" (click)="fetchUrl()">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>
            </svg>
            Fetch {{ urlType() }}
          </button>
        }

        @if (loading()) {
          <div class="loading-state">
            <div class="loading-bars">
              <span class="lbar"></span>
              <span class="lbar"></span>
              <span class="lbar"></span>
              <span class="lbar"></span>
            </div>
            <span class="loading-text">Fetching metadata...</span>
          </div>
        }

        @if (fetchError()) {
          <p class="error-hint">{{ fetchError() }}</p>
        }

        @if (resolved()) {
          <spytfy-preview-card [resolved]="resolved()!" [sourceUrl]="url()" />
        }

        @if (urlType()) {
          <div class="debug-section">
            <button class="ghost-btn" (click)="runDebug()">Debug Scrape</button>
            <button class="ghost-btn" (click)="showJsonInput.set(!showJsonInput())">Paste API JSON</button>
          </div>
        }

        @if (debugOutput()) {
          <pre class="debug-output">{{ debugOutput() }}</pre>
        }

        @if (showJsonInput()) {
          <div class="json-input-section">
            <textarea
              class="json-textarea"
              [ngModel]="jsonInput()"
              (ngModelChange)="jsonInput.set($event)"
              placeholder="Paste Spotify API JSON response here..."
              rows="6"
            ></textarea>
            <button class="fetch-btn compact" (click)="loadFromJson()">Load from JSON</button>
          </div>
        }

        @if (!resolved()) {
          <div class="type-chips">
            @for (type of supportedTypes; track type.name) {
              <div class="chip" [class.matched]="urlType() === type.name">
                <span class="chip-icon" [innerHTML]="type.icon"></span>
                <span>{{ type.label }}</span>
              </div>
            }
          </div>

          <p class="kbd-hint">
            <kbd>Ctrl</kbd><span>+</span><kbd>V</kbd> to paste
          </p>
        }
      </div>
    </div>
  `,
  styles: `
    .page {
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100%;
      padding: 48px 32px;
    }
    .content {
      display: flex;
      flex-direction: column;
      align-items: center;
      width: 100%;
      max-width: 560px;
      gap: 24px;
    }
    .content.animate-in {
      animation: slideUp var(--duration-slow) var(--ease-out) both;
    }

    .hero {
      text-align: center;
      margin-bottom: 8px;
    }
    h1 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 32px;
      font-weight: 800;
      letter-spacing: -0.04em;
      color: var(--text-primary);
      line-height: 1.1;
      margin-bottom: 12px;
    }
    .subtitle {
      font-size: 15px;
      color: var(--text-muted);
      line-height: 1.4;
    }

    .search-bar {
      display: flex;
      align-items: center;
      gap: 12px;
      width: 100%;
      height: 52px;
      background: var(--surface-700);
      border-radius: var(--radius-pill);
      padding: 0 8px 0 16px;
      transition: background var(--duration-fast) var(--ease-out),
                  box-shadow var(--duration-fast) var(--ease-out);
    }
    .search-bar:hover {
      background: var(--surface-600);
    }
    .search-bar.focused {
      background: var(--surface-600);
      box-shadow: 0 0 0 2px var(--accent);
    }
    .search-bar.has-type {
      background: var(--surface-600);
    }
    .search-icon {
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      flex-shrink: 0;
    }
    .type-pill {
      display: inline-flex;
      align-items: center;
      padding: 3px 10px;
      background: var(--accent);
      color: #000;
      font-size: 11px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      border-radius: var(--radius-pill);
    }
    input {
      flex: 1;
      background: none;
      border: none;
      outline: none;
      color: var(--text-primary);
      font-family: inherit;
      font-size: 14px;
      height: 100%;
      min-width: 0;
    }
    input::placeholder {
      color: var(--text-muted);
    }
    .clear-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      border: none;
      border-radius: 50%;
      background: var(--surface-500);
      color: var(--text-secondary);
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      flex-shrink: 0;
    }
    .clear-btn:hover {
      color: var(--text-primary);
      background: var(--surface-500);
      transform: scale(1.1);
    }
    .paste-pill {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 0 14px;
      height: 36px;
      background: var(--surface-500);
      border: none;
      border-radius: var(--radius-pill);
      color: var(--text-secondary);
      font-family: inherit;
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      white-space: nowrap;
      flex-shrink: 0;
    }
    .paste-pill:hover {
      color: var(--text-primary);
      background: rgba(255, 255, 255, 0.15);
    }

    .error-hint {
      font-size: 13px;
      color: var(--error);
      font-weight: 500;
    }

    .fetch-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      width: 100%;
      height: 52px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: 'Space Grotesk', sans-serif;
      font-size: 16px;
      font-weight: 700;
      letter-spacing: -0.01em;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      text-transform: capitalize;
    }
    .fetch-btn:hover {
      background: var(--accent-hover);
      transform: scale(1.02);
    }
    .fetch-btn:active {
      transform: scale(0.98);
    }
    .fetch-btn.compact {
      height: 44px;
      font-size: 14px;
    }

    .loading-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 16px;
      padding: 32px 0;
    }
    .loading-bars {
      display: flex;
      align-items: flex-end;
      gap: 4px;
      height: 24px;
    }
    .lbar {
      display: block;
      width: 4px;
      border-radius: 2px;
      background: var(--accent);
    }
    .lbar:nth-child(1) { animation: soundbar 1s ease-in-out infinite 0s; }
    .lbar:nth-child(2) { animation: soundbar 1s ease-in-out infinite 0.15s; }
    .lbar:nth-child(3) { animation: soundbar 1s ease-in-out infinite 0.3s; }
    .lbar:nth-child(4) { animation: soundbar 1s ease-in-out infinite 0.45s; }
    @keyframes soundbar {
      0%, 100% { height: 4px; }
      50% { height: 22px; }
    }
    .loading-text {
      font-size: 13px;
      color: var(--text-muted);
      font-weight: 500;
    }

    .type-chips {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      justify-content: center;
    }
    .chip {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 8px 16px;
      background: rgba(255, 255, 255, 0.05);
      border: none;
      border-radius: var(--radius-pill);
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 500;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .chip.matched {
      background: var(--accent-subtle);
      color: var(--accent);
    }
    .chip-icon {
      display: flex;
      align-items: center;
    }

    .kbd-hint {
      font-size: 12px;
      color: var(--text-muted);
      display: flex;
      align-items: center;
      gap: 4px;
    }
    kbd {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 24px;
      height: 22px;
      padding: 0 6px;
      background: rgba(255, 255, 255, 0.06);
      border: 1px solid rgba(255, 255, 255, 0.1);
      border-radius: var(--radius-sm);
      font-family: inherit;
      font-size: 11px;
      font-weight: 600;
      color: var(--text-muted);
    }

    .debug-section {
      display: flex;
      gap: 8px;
      width: 100%;
    }
    .ghost-btn {
      padding: 8px 14px;
      background: transparent;
      border: 1px solid var(--surface-600);
      border-radius: var(--radius-pill);
      color: var(--text-muted);
      font-family: inherit;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .ghost-btn:hover {
      color: var(--text-primary);
      border-color: var(--text-muted);
    }
    .debug-output {
      width: 100%;
      max-height: 200px;
      overflow: auto;
      padding: 16px;
      background: var(--surface-900);
      border-radius: var(--radius-md);
      font-size: 11px;
      color: var(--text-secondary);
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      white-space: pre-wrap;
      word-break: break-all;
    }
    .json-input-section {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .json-textarea {
      width: 100%;
      padding: 16px;
      background: var(--surface-700);
      border: 1px solid var(--surface-600);
      border-radius: var(--radius-md);
      color: var(--text-primary);
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      font-size: 12px;
      resize: vertical;
      outline: none;
      transition: border-color var(--duration-fast) var(--ease-out);
    }
    .json-textarea:focus {
      border-color: var(--accent);
    }

    @media (prefers-reduced-motion: reduce) {
      .content.animate-in { animation: none; }
      .lbar { animation: none; }
      .lbar:nth-child(1) { height: 8px; }
      .lbar:nth-child(2) { height: 18px; }
      .lbar:nth-child(3) { height: 14px; }
      .lbar:nth-child(4) { height: 10px; }
    }
  `,
})
export class InputPage {
  url = signal('');
  inputFocused = signal(false);
  resolved = signal<ResolvedInput | null>(null);
  loading = signal(false);
  fetchError = signal('');
  debugOutput = signal('');
  showJsonInput = signal(false);
  jsonInput = signal('');

  urlType = computed<UrlType>(() => {
    const val = this.url();
    if (!val) return null;
    const match = val.match(/open\.spotify\.com\/(track|album|playlist|artist)\//);
    return (match?.[1] as UrlType) ?? null;
  });

  supportedTypes = [
    { name: 'track', label: 'Track', icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>' },
    { name: 'album', label: 'Album', icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>' },
    { name: 'playlist', label: 'Playlist', icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15V6"/><path d="M18.5 18a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z"/><path d="M12 12H3"/><path d="M16 6H3"/><path d="M12 18H3"/></svg>' },
    { name: 'artist', label: 'Artist', icon: '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>' },
  ];

  onUrlChange(value: string) {
    this.url.set(value.trim());
  }

  onPaste(event: ClipboardEvent) {
    const text = event.clipboardData?.getData('text')?.trim();
    if (text) {
      event.preventDefault();
      this.url.set(text);
    }
  }

  async pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) this.url.set(text.trim());
    } catch {}
  }

  clear() {
    this.url.set('');
    this.resolved.set(null);
    this.fetchError.set('');
  }

  async runDebug() {
    try {
      this.debugOutput.set('Fetching...');
      const result = await debugScrape(this.url());
      this.debugOutput.set(result);
    } catch (e: any) {
      this.debugOutput.set(`Error: ${typeof e === 'string' ? e : e?.message}`);
    }
  }

  async loadFromJson() {
    try {
      const result = await resolveFromJson(this.jsonInput());
      this.resolved.set(result);
      this.showJsonInput.set(false);
      this.fetchError.set('');
    } catch (e: any) {
      this.fetchError.set(typeof e === 'string' ? e : e?.message || 'Invalid JSON');
    }
  }

  async fetchUrl() {
    this.loading.set(true);
    this.fetchError.set('');
    this.resolved.set(null);
    try {
      const result = await resolveUrl(this.url());
      this.resolved.set(result);
    } catch (e: any) {
      this.fetchError.set(typeof e === 'string' ? e : e?.message || 'Failed to resolve URL');
    } finally {
      this.loading.set(false);
    }
  }
}
