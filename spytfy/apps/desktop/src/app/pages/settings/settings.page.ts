import { Component, ChangeDetectionStrategy, signal, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { getSettings, updateSettings, saveSpotifyCredentials, hasSpotifyCredentials } from '@spytfy/tauri-ipc';

@Component({
  selector: 'spytfy-settings-page',
  standalone: true,
  imports: [FormsModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page animate-in">
      <h1>Settings</h1>

      <section>
        <h3 class="section-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
          Spotify Connection
        </h3>
        <div class="card">
          @if (spotifyConnected()) {
            <div class="row">
              <div class="row-info">
                <span class="row-label">Status</span>
                <span class="status-connected">
                  <span class="status-dot"></span>
                  Connected
                </span>
              </div>
              <button class="secondary-btn" (click)="showCredFields.set(true)">Reconfigure</button>
            </div>
          }
          @if (!spotifyConnected() || showCredFields()) {
            <div class="form">
              <div class="field">
                <label>Client ID</label>
                <input
                  type="text"
                  [ngModel]="clientId()"
                  (ngModelChange)="clientId.set($event)"
                  placeholder="Paste your Spotify Client ID"
                  spellcheck="false"
                />
              </div>
              <div class="field">
                <label>Client Secret</label>
                <input
                  type="password"
                  [ngModel]="clientSecret()"
                  (ngModelChange)="clientSecret.set($event)"
                  placeholder="Paste your Spotify Client Secret"
                  spellcheck="false"
                />
              </div>
              @if (spotifyError()) {
                <p class="msg error">{{ spotifyError() }}</p>
              }
              @if (spotifySuccess()) {
                <p class="msg success">Connected successfully</p>
              }
              <button
                class="primary-btn"
                [disabled]="!clientId() || !clientSecret() || saving()"
                (click)="saveCredentials()"
              >
                {{ saving() ? 'Connecting...' : 'Save & Connect' }}
              </button>
            </div>
          }
        </div>
      </section>

      <section>
        <h3 class="section-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
          Performance
        </h3>
        <div class="card">
          <div class="row">
            <div class="row-info">
              <span class="row-label">Concurrent Downloads</span>
              <span class="row-value">{{ concurrency() }} workers</span>
            </div>
            <div class="slider-group">
              <input type="range" min="1" max="8" [value]="concurrency()"
                (input)="onConcurrencyChange($event)" class="slider" />
              <span class="slider-num">{{ concurrency() }}</span>
            </div>
          </div>
        </div>
      </section>

      <section>
        <h3 class="section-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
          Output
        </h3>
        <div class="card">
          <div class="row">
            <div class="row-info">
              <span class="row-label">Output Folder</span>
              <span class="row-path">{{ outputRoot() || 'Not set' }}</span>
            </div>
            <button class="secondary-btn" (click)="browseFolder()">Browse</button>
          </div>
        </div>
      </section>

      <section>
        <h3 class="section-title">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
          File Naming
        </h3>
        <div class="card">
          <div class="field">
            <label>Naming Template</label>
            <input
              type="text"
              [ngModel]="namingTemplate()"
              (ngModelChange)="onNamingTemplateChange($event)"
              placeholder="{folder}/{number} - {artist} - {title}"
              spellcheck="false"
            />
            <span class="field-hint">Tokens: {{ '{' }}folder{{ '}' }} {{ '{' }}number{{ '}' }} {{ '{' }}artist{{ '}' }} {{ '{' }}title{{ '}' }} {{ '{' }}album{{ '}' }}</span>
          </div>
          <div class="row" style="margin-top: 16px">
            <div class="row-info">
              <span class="row-label">Write cover.jpg</span>
              <span class="row-hint">Save album art in each folder</span>
            </div>
            <label class="toggle">
              <input type="checkbox" [ngModel]="writeCoverJpg()" (ngModelChange)="onWriteCoverJpgChange($event)" />
              <span class="toggle-track"></span>
            </label>
          </div>
        </div>
      </section>
    </div>
  `,
  styles: `
    .page {
      padding: 32px 40px;
      max-width: 680px;
    }
    .page.animate-in { animation: slideUp var(--duration-slow) var(--ease-out) both; }

    h1 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 28px;
      font-weight: 800;
      letter-spacing: -0.04em;
      margin-bottom: 32px;
    }

    section { margin-bottom: 24px; }
    .section-title {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 13px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--text-muted);
      margin-bottom: 10px;
    }

    .card {
      background: var(--surface-700);
      border-radius: var(--radius-lg);
      padding: 20px 24px;
      transition: background var(--duration-fast) var(--ease-out);
    }
    .card:hover { background: rgba(255, 255, 255, 0.06); }

    .row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
    }
    .row-info { min-width: 0; }
    .row-label { font-size: 14px; font-weight: 500; display: block; }
    .row-value {
      font-size: 13px;
      color: var(--text-muted);
      margin-top: 2px;
      display: block;
    }
    .row-path {
      font-size: 13px;
      color: var(--text-muted);
      font-family: 'Cascadia Code', 'Fira Code', monospace;
      margin-top: 2px;
      max-width: 360px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      display: block;
    }
    .row-hint {
      font-size: 13px;
      color: var(--text-muted);
      margin-top: 2px;
      display: block;
    }
    .status-connected {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 13px;
      color: var(--accent);
      font-weight: 600;
      margin-top: 2px;
    }
    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent);
      box-shadow: 0 0 8px var(--accent-glow);
      animation: pulse-dot 2s ease infinite;
    }

    .form {
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .field {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    label {
      font-size: 13px;
      font-weight: 600;
      color: var(--text-secondary);
    }
    input[type="text"],
    input[type="password"] {
      height: 44px;
      background: var(--surface-600);
      border: 1px solid transparent;
      border-radius: var(--radius-md);
      padding: 0 16px;
      color: var(--text-primary);
      font-family: inherit;
      font-size: 14px;
      outline: none;
      transition: border-color var(--duration-fast) var(--ease-out);
    }
    input[type="text"]:focus,
    input[type="password"]:focus {
      border-color: var(--accent);
    }
    input::placeholder { color: var(--text-muted); }
    .field-hint {
      font-size: 12px;
      color: var(--text-muted);
    }

    .msg { font-size: 13px; font-weight: 500; }
    .msg.error { color: var(--error); }
    .msg.success { color: var(--accent); }

    .primary-btn {
      height: 44px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: inherit;
      font-size: 14px;
      font-weight: 700;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .primary-btn:hover:not(:disabled) { background: var(--accent-hover); }
    .primary-btn:disabled { opacity: 0.4; cursor: default; }

    .secondary-btn {
      padding: 8px 18px;
      background: rgba(255, 255, 255, 0.07);
      border: none;
      border-radius: var(--radius-pill);
      color: var(--text-primary);
      font-family: inherit;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      flex-shrink: 0;
    }
    .secondary-btn:hover { background: rgba(255, 255, 255, 0.12); }

    .slider-group { display: flex; align-items: center; gap: 12px; }
    .slider {
      width: 140px;
      appearance: none;
      height: 4px;
      border-radius: 2px;
      background: var(--surface-500);
      outline: none;
      cursor: pointer;
    }
    .slider::-webkit-slider-thumb {
      appearance: none;
      width: 14px;
      height: 14px;
      border-radius: 50%;
      background: var(--text-primary);
      cursor: pointer;
      box-shadow: 0 0 0 2px var(--surface-700);
      transition: all var(--duration-fast) var(--ease-out);
    }
    .slider::-webkit-slider-thumb:hover {
      transform: scale(1.3);
    }
    .slider:active::-webkit-slider-thumb {
      background: var(--accent);
    }
    .slider-num {
      font-size: 14px;
      font-weight: 700;
      color: var(--text-primary);
      min-width: 20px;
      text-align: center;
    }

    .toggle {
      position: relative;
      display: inline-block;
      width: 44px;
      height: 24px;
      flex-shrink: 0;
    }
    .toggle input { opacity: 0; width: 0; height: 0; }
    .toggle-track {
      position: absolute;
      inset: 0;
      background: var(--surface-500);
      border-radius: 12px;
      cursor: pointer;
      transition: background var(--duration-normal) var(--ease-out);
    }
    .toggle-track::before {
      content: '';
      position: absolute;
      width: 18px;
      height: 18px;
      left: 3px;
      top: 3px;
      background: var(--text-primary);
      border-radius: 50%;
      transition: transform var(--duration-normal) var(--ease-out);
    }
    .toggle input:checked + .toggle-track { background: var(--accent); }
    .toggle input:checked + .toggle-track::before { transform: translateX(20px); }

    @media (prefers-reduced-motion: reduce) {
      .page.animate-in { animation: none; }
      .status-dot { animation: none; }
    }
  `,
})
export class SettingsPage implements OnInit {
  outputRoot = signal('');
  concurrency = signal(3);
  namingTemplate = signal('{folder}/{number} - {artist} - {title}');
  writeCoverJpg = signal(true);
  spotifyConnected = signal(false);
  showCredFields = signal(false);
  clientId = signal('');
  clientSecret = signal('');
  spotifyError = signal('');
  spotifySuccess = signal(false);
  saving = signal(false);

  async ngOnInit() {
    try {
      const settings = await getSettings();
      this.outputRoot.set(settings.outputRoot);
      this.namingTemplate.set(settings.namingTemplate);
      this.writeCoverJpg.set(settings.writeCoverJpg);
    } catch {}
    try {
      this.spotifyConnected.set(await hasSpotifyCredentials());
    } catch {}
  }

  async saveCredentials() {
    this.saving.set(true);
    this.spotifyError.set('');
    this.spotifySuccess.set(false);
    try {
      await saveSpotifyCredentials(this.clientId(), this.clientSecret());
      this.spotifyConnected.set(true);
      this.spotifySuccess.set(true);
      this.showCredFields.set(false);
    } catch (e: any) {
      this.spotifyError.set(typeof e === 'string' ? e : e?.message || 'Connection failed');
    } finally {
      this.saving.set(false);
    }
  }

  async onConcurrencyChange(event: Event) {
    const val = +(event.target as HTMLInputElement).value;
    this.concurrency.set(val);
    try {
      await updateSettings({ concurrency: val });
    } catch {}
  }

  private namingDebounce: any;
  async onNamingTemplateChange(val: string) {
    this.namingTemplate.set(val);
    clearTimeout(this.namingDebounce);
    this.namingDebounce = setTimeout(async () => {
      try { await updateSettings({ namingTemplate: val }); } catch {}
    }, 600);
  }

  async onWriteCoverJpgChange(val: boolean) {
    this.writeCoverJpg.set(val);
    try { await updateSettings({ writeCoverJpg: val }); } catch {}
  }

  async browseFolder() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        const settings = await updateSettings({ outputRoot: selected as string });
        this.outputRoot.set(settings.outputRoot);
      }
    } catch {}
  }
}
