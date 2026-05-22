import { Component, ChangeDetectionStrategy, signal, output } from '@angular/core';
import { processScreenshots, createPlaylistFromTracks, parseTextTracklist, parseSpotifyHtml, type ParsedTrack } from '@spytfy/tauri-ipc';
import type { ResolvedInput } from '@spytfy/models';

@Component({
  selector: 'spytfy-screenshot-modal',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="overlay" (click)="close.emit()">
      <div class="modal" (click)="$event.stopPropagation()">
        <div class="modal-header">
          <h2>Import Tracks</h2>
          <button class="close-btn" (click)="close.emit()">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
          </button>
        </div>

        <div class="tab-bar">
          <button class="tab" [class.active]="mode() === 'text'" (click)="mode.set('text')">Text</button>
          <button class="tab" [class.active]="mode() === 'html'" (click)="mode.set('html')">HTML</button>
          <button class="tab" [class.active]="mode() === 'screenshot'" (click)="mode.set('screenshot')">Screenshots</button>
        </div>

        @if (mode() === 'html' && stage() !== 'results') {
          <div class="input-section">
            <p class="hint">Inspect Element on your playlist's track list and copy the outer HTML</p>
            <textarea
              class="textarea"
              [value]="htmlInput()"
              (input)="htmlInput.set($any($event.target).value)"
              placeholder="Paste playlist HTML here..."
              rows="8"
            ></textarea>
            @if (htmlInput()) {
              <button class="primary-btn" (click)="processHtml()">Parse HTML</button>
            }
          </div>
        }

        @if (mode() === 'text' && stage() !== 'results') {
          <div class="input-section">
            <p class="hint">One track per line — <code>Artist - Title</code></p>
            <textarea
              class="textarea"
              [value]="textInput()"
              (input)="textInput.set($any($event.target).value)"
              placeholder="Abyusif - 3azra2eel 2&#10;Twenty One Pilots - Stressed Out&#10;Molotof, Marwan Pablo - Geb Felos&#10;Apashe, Vo Williams - Work"
              rows="10"
            ></textarea>
            @if (textInput()) {
              <button class="primary-btn" (click)="processText()">Parse Tracks</button>
            }
          </div>
        }

        @if (mode() === 'screenshot' && stage() === 'upload') {
          <div class="drop-zone"
            (dragover)="onDragOver($event)"
            (drop)="onDrop($event)"
            (click)="fileInput.click()"
          >
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>
            </svg>
            <p>Drop screenshots or click to browse</p>
            <span class="drop-hint">Screenshots of your Spotify track list</span>
          </div>
          <input #fileInput type="file" accept="image/*" multiple
            style="display:none" (change)="onFileSelect($event)" />

          @if (imagePaths().length > 0) {
            <div class="file-list">
              @for (path of imagePaths(); track path) {
                <div class="file-row">
                  <span class="file-name">{{ getFileName(path) }}</span>
                  <button class="remove-btn" (click)="removeImage(path)">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                  </button>
                </div>
              }
            </div>
            <button class="primary-btn" (click)="processImages()">
              Process {{ imagePaths().length }} screenshot{{ imagePaths().length > 1 ? 's' : '' }}
            </button>
          }
        }

        @if (stage() === 'processing') {
          <div class="processing">
            <div class="spinner"></div>
            <p>Extracting tracks...</p>
          </div>
        }

        @if (stage() === 'results') {
          <div class="results">
            <p class="result-count">{{ tracks().length }} tracks found</p>
            <div class="result-list">
              @for (track of tracks(); track track.trackNumber) {
                <div class="result-row">
                  <span class="row-num">{{ track.trackNumber }}</span>
                  <div class="row-info">
                    <span class="row-title">{{ track.title }}</span>
                    <span class="row-artist">{{ track.artist }}</span>
                  </div>
                  <span class="row-dur">{{ formatDuration(track.durationMs) }}</span>
                </div>
              }
            </div>
            @if (tracks().length > 0) {
              <button class="primary-btn" (click)="confirmTracks()">
                Use {{ tracks().length }} tracks
              </button>
            } @else {
              <p class="error-msg">No tracks found. Try clearer input.</p>
            }
            <button class="ghost-btn" (click)="stage.set('upload'); imagePaths.set([])">
              Try again
            </button>
          </div>
        }

        @if (error()) {
          <p class="error-msg">{{ error() }}</p>
        }
      </div>
    </div>
  `,
  styles: `
    .overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.75);
      backdrop-filter: blur(4px);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 100;
      animation: fadeIn var(--duration-normal) var(--ease-out) both;
    }
    .modal {
      width: 90%;
      max-width: 540px;
      max-height: 80vh;
      overflow-y: auto;
      background: var(--surface-700);
      border-radius: var(--radius-xl);
      padding: 28px;
      animation: scaleIn var(--duration-slow) var(--ease-out) both;
      box-shadow: var(--shadow-xl);
    }
    .modal-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 20px;
    }
    h2 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 20px;
      font-weight: 700;
      letter-spacing: -0.02em;
    }
    .close-btn {
      width: 32px;
      height: 32px;
      border: none;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.07);
      color: var(--text-secondary);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .close-btn:hover {
      background: rgba(255, 255, 255, 0.12);
      color: var(--text-primary);
    }

    .tab-bar {
      display: flex;
      gap: 4px;
      margin-bottom: 20px;
      background: rgba(255, 255, 255, 0.05);
      border-radius: var(--radius-md);
      padding: 4px;
    }
    .tab {
      flex: 1;
      padding: 10px;
      border: none;
      border-radius: 6px;
      background: transparent;
      color: var(--text-muted);
      font-family: inherit;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .tab.active {
      background: var(--surface-600);
      color: var(--text-primary);
    }
    .tab:hover:not(.active) { color: var(--text-secondary); }

    .input-section {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .hint {
      font-size: 13px;
      color: var(--text-muted);
    }
    .hint code {
      background: rgba(255, 255, 255, 0.06);
      padding: 2px 6px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      color: var(--text-secondary);
    }
    .textarea {
      width: 100%;
      padding: 14px;
      background: var(--surface-600);
      border: 1px solid transparent;
      border-radius: var(--radius-md);
      color: var(--text-primary);
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      font-size: 13px;
      resize: vertical;
      outline: none;
      line-height: 1.6;
      transition: border-color var(--duration-fast) var(--ease-out);
    }
    .textarea:focus { border-color: var(--accent); }
    .textarea::placeholder { color: var(--text-muted); }

    .drop-zone {
      border: 2px dashed var(--surface-500);
      border-radius: var(--radius-lg);
      padding: 48px 24px;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 12px;
      color: var(--text-muted);
      cursor: pointer;
      text-align: center;
      transition: all var(--duration-normal) var(--ease-out);
    }
    .drop-zone:hover {
      border-color: var(--accent);
      color: var(--text-secondary);
      background: rgba(29, 185, 84, 0.03);
    }
    .drop-zone p { font-size: 14px; font-weight: 500; }
    .drop-hint { font-size: 12px; }

    .file-list {
      margin-top: 16px;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .file-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 14px;
      background: var(--surface-600);
      border-radius: var(--radius-md);
    }
    .file-name { font-size: 13px; color: var(--text-secondary); }
    .remove-btn {
      border: none;
      background: none;
      color: var(--text-muted);
      cursor: pointer;
      display: flex;
      padding: 4px;
    }
    .remove-btn:hover { color: var(--error); }

    .primary-btn {
      width: 100%;
      height: 48px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: inherit;
      font-size: 14px;
      font-weight: 700;
      cursor: pointer;
      margin-top: 16px;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .primary-btn:hover {
      background: var(--accent-hover);
      transform: scale(1.02);
    }
    .ghost-btn {
      width: 100%;
      background: none;
      border: none;
      color: var(--text-muted);
      font-family: inherit;
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      padding: 12px;
      transition: color var(--duration-fast) var(--ease-out);
    }
    .ghost-btn:hover { color: var(--text-secondary); }

    .processing {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 16px;
      padding: 48px 0;
    }
    .processing p { font-size: 14px; color: var(--text-secondary); }
    .spinner {
      width: 32px;
      height: 32px;
      border: 3px solid var(--surface-500);
      border-top-color: var(--accent);
      border-radius: 50%;
      animation: spin 0.7s linear infinite;
    }

    .results {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .result-count {
      font-size: 14px;
      color: var(--accent);
      font-weight: 700;
    }
    .result-list {
      max-height: 300px;
      overflow-y: auto;
    }
    .result-row {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 8px 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    }
    .row-num { font-size: 13px; color: var(--text-muted); width: 24px; text-align: right; font-variant-numeric: tabular-nums; }
    .row-info { flex: 1; min-width: 0; }
    .row-title { font-size: 14px; font-weight: 500; display: block; }
    .row-artist { font-size: 13px; color: var(--text-muted); display: block; }
    .row-dur { font-size: 13px; color: var(--text-muted); font-variant-numeric: tabular-nums; }
    .error-msg { font-size: 13px; color: var(--error); text-align: center; }

    @media (prefers-reduced-motion: reduce) {
      .overlay, .modal { animation: none; }
      .spinner { animation: none; }
    }
  `,
})
export class ScreenshotModalComponent {
  close = output<void>();
  tracksReady = output<ResolvedInput>();

  mode = signal<'text' | 'html' | 'screenshot'>('text');
  htmlInput = signal('');
  stage = signal<'upload' | 'processing' | 'results'>('upload');
  textInput = signal('');
  imagePaths = signal<string[]>([]);
  tracks = signal<ParsedTrack[]>([]);
  error = signal('');

  playlistName = '';
  coverUrl: string | null = null;

  onDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
  }

  async onDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    const files = e.dataTransfer?.files;
    if (files) {
      await this.addFiles(files);
    }
  }

  async onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files) {
      await this.addFiles(input.files);
    }
  }

  private async addFiles(files: FileList) {
    const paths: string[] = [];
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const { appDataDir } = await import('@tauri-apps/api/path');
      const dir = await appDataDir();
      const tempPath = `${dir}/.spytfy/temp/screenshot-${Date.now()}-${i}.png`;

      const { mkdir, writeFile } = await import('@tauri-apps/plugin-fs');
      await mkdir(`${dir}/.spytfy/temp`, { recursive: true });
      const buffer = await file.arrayBuffer();
      await writeFile(tempPath, new Uint8Array(buffer));
      paths.push(tempPath);
    }
    this.imagePaths.update(existing => [...existing, ...paths]);
  }

  removeImage(path: string) {
    this.imagePaths.update(paths => paths.filter(p => p !== path));
  }

  getFileName(path: string): string {
    return path.split(/[/\\]/).pop() || path;
  }

  async processHtml() {
    this.stage.set('processing');
    this.error.set('');
    try {
      const result = await parseSpotifyHtml(this.htmlInput());
      this.tracks.set(result);
      this.stage.set('results');
    } catch (e: any) {
      this.error.set(typeof e === 'string' ? e : e?.message || 'HTML parse failed');
      this.stage.set('upload');
    }
  }

  async processText() {
    this.stage.set('processing');
    this.error.set('');
    try {
      const result = await parseTextTracklist(this.textInput());
      this.tracks.set(result);
      this.stage.set('results');
    } catch (e: any) {
      this.error.set(typeof e === 'string' ? e : e?.message || 'Parse failed');
      this.stage.set('upload');
    }
  }

  async processImages() {
    this.stage.set('processing');
    this.error.set('');
    try {
      const result = await processScreenshots(this.imagePaths());
      this.tracks.set(result);
      this.stage.set('results');
    } catch (e: any) {
      this.error.set(typeof e === 'string' ? e : e?.message || 'OCR failed');
      this.stage.set('upload');
    }
  }

  async confirmTracks() {
    try {
      const playlist = await createPlaylistFromTracks(
        this.playlistName || 'Imported Playlist',
        this.coverUrl,
        this.tracks()
      );
      this.tracksReady.emit(playlist);
    } catch (e: any) {
      this.error.set(typeof e === 'string' ? e : e?.message || 'Failed to create playlist');
    }
  }

  formatDuration(ms: number): string {
    if (!ms) return '';
    const sec = Math.floor(ms / 1000);
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }
}
