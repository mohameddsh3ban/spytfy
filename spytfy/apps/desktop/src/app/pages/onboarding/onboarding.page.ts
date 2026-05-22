import { Component, ChangeDetectionStrategy, signal } from '@angular/core';
import { Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { saveSpotifyCredentials } from '@spytfy/tauri-ipc';

@Component({
  selector: 'spytfy-onboarding-page',
  standalone: true,
  imports: [FormsModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page">
      <div class="card">
        <div class="progress-bar">
          <div class="progress-fill" [style.width.%]="(step() / 3) * 100"></div>
        </div>
        @switch (step()) {
          @case (0) {
            <div class="step">
              <div class="logo-mark">
                <div class="soundbars">
                  <span class="bar"></span>
                  <span class="bar"></span>
                  <span class="bar"></span>
                  <span class="bar"></span>
                </div>
              </div>
              <h1>Spytfy</h1>
              <p>Download your Spotify library as high-quality MP3s with cover art and proper tags.</p>
              <button class="primary-btn" (click)="step.set(1)">Get Started</button>
            </div>
          }
          @case (1) {
            <div class="step">
              <h2>Connect to Spotify</h2>
              <p>Spytfy reads track metadata via the Spotify API. You need a <strong>free</strong> developer account — no Premium required.</p>
              <div class="steps-list">
                <div class="guide-step">
                  <span class="guide-num">1</span>
                  <span>Go to <a href="https://developer.spotify.com/dashboard" target="_blank" rel="noopener">developer.spotify.com/dashboard</a></span>
                </div>
                <div class="guide-step">
                  <span class="guide-num">2</span>
                  <span>Log in with any Spotify account</span>
                </div>
                <div class="guide-step">
                  <span class="guide-num">3</span>
                  <span>Click <strong>Create App</strong> — set redirect URI to <code>http://localhost</code></span>
                </div>
                <div class="guide-step">
                  <span class="guide-num">4</span>
                  <span>Copy your <strong>Client ID</strong> and <strong>Client Secret</strong></span>
                </div>
              </div>
              <button class="primary-btn" (click)="step.set(2)">I have my credentials</button>
              <button class="ghost-btn" (click)="step.set(0)">Back</button>
            </div>
          }
          @case (2) {
            <div class="step">
              <h2>Enter Credentials</h2>
              <div class="field">
                <label>Client ID</label>
                <input
                  type="text"
                  [ngModel]="clientId()"
                  (ngModelChange)="clientId.set($event)"
                  placeholder="e.g. 1a2b3c4d5e6f..."
                  spellcheck="false"
                  autocomplete="off"
                />
              </div>
              <div class="field">
                <label>Client Secret</label>
                <input
                  type="password"
                  [ngModel]="clientSecret()"
                  (ngModelChange)="clientSecret.set($event)"
                  placeholder="e.g. 9z8y7x6w5v..."
                  spellcheck="false"
                  autocomplete="off"
                />
              </div>
              @if (error()) {
                <p class="msg error">{{ error() }}</p>
              }
              <button
                class="primary-btn"
                [disabled]="!clientId() || !clientSecret() || testing()"
                (click)="testAndSave()"
              >
                {{ testing() ? 'Connecting...' : 'Connect' }}
              </button>
              <button class="ghost-btn" (click)="step.set(1)">Back</button>
            </div>
          }
          @case (3) {
            <div class="step">
              <div class="success-ring">
                <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M20 6 9 17l-5-5"/>
                </svg>
              </div>
              <h2>You're all set</h2>
              <p>Spotify connected. Start pasting links to download.</p>
              <button class="primary-btn" (click)="finish()">Start Using Spytfy</button>
            </div>
          }
        }
      </div>
    </div>
  `,
  styles: `
    .page {
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
      background: linear-gradient(160deg, rgba(29, 185, 84, 0.06) 0%, var(--surface-800) 30%, var(--surface-900) 100%);
    }
    .card {
      width: 100%;
      max-width: 480px;
      background: var(--surface-700);
      border-radius: var(--radius-xl);
      padding: 0;
      overflow: hidden;
      animation: scaleIn 400ms var(--ease-out) both;
      box-shadow: var(--shadow-xl);
    }
    .progress-bar {
      height: 3px;
      background: rgba(255, 255, 255, 0.06);
    }
    .progress-fill {
      height: 100%;
      background: var(--accent);
      border-radius: 0 2px 2px 0;
      transition: width 400ms var(--ease-out);
    }

    .step {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 16px;
      padding: 48px 40px 44px;
      animation: fadeIn var(--duration-slow) var(--ease-out) both;
    }

    .logo-mark {
      margin-bottom: 8px;
    }
    .soundbars {
      display: flex;
      align-items: flex-end;
      gap: 5px;
      height: 32px;
    }
    .bar {
      display: block;
      width: 6px;
      border-radius: 3px;
      background: var(--accent);
    }
    .bar:nth-child(1) { animation: soundbar 1.4s ease-in-out infinite 0s; }
    .bar:nth-child(2) { animation: soundbar 1.4s ease-in-out infinite 0.2s; }
    .bar:nth-child(3) { animation: soundbar 1.4s ease-in-out infinite 0.4s; }
    .bar:nth-child(4) { animation: soundbar 1.4s ease-in-out infinite 0.6s; }
    @keyframes soundbar {
      0%, 100% { height: 6px; }
      50% { height: 28px; }
    }

    h1 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 32px;
      font-weight: 800;
      letter-spacing: -0.04em;
    }
    h2 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 22px;
      font-weight: 700;
      letter-spacing: -0.03em;
    }
    p {
      font-size: 15px;
      color: var(--text-secondary);
      line-height: 1.5;
      max-width: 360px;
    }
    strong { color: var(--text-primary); }
    a { color: var(--accent); text-decoration: none; }
    a:hover { text-decoration: underline; }
    code {
      background: rgba(255, 255, 255, 0.06);
      padding: 2px 8px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      color: var(--text-primary);
    }

    .steps-list {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 14px;
      text-align: left;
      margin: 4px 0;
    }
    .guide-step {
      display: flex;
      align-items: flex-start;
      gap: 14px;
      font-size: 14px;
      color: var(--text-secondary);
      line-height: 1.4;
    }
    .guide-num {
      width: 28px;
      height: 28px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.06);
      color: var(--text-primary);
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 13px;
      font-weight: 700;
      flex-shrink: 0;
    }

    .field {
      width: 100%;
      text-align: left;
    }
    label {
      display: block;
      font-size: 13px;
      font-weight: 600;
      color: var(--text-secondary);
      margin-bottom: 8px;
    }
    input {
      width: 100%;
      height: 48px;
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
    input:focus { border-color: var(--accent); }
    input::placeholder { color: var(--text-muted); }

    .msg { font-size: 13px; font-weight: 500; }
    .msg.error { color: var(--error); }

    .primary-btn {
      width: 100%;
      height: 48px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: inherit;
      font-size: 15px;
      font-weight: 700;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      margin-top: 4px;
    }
    .primary-btn:hover:not(:disabled) {
      background: var(--accent-hover);
      transform: scale(1.02);
    }
    .primary-btn:disabled { opacity: 0.4; cursor: default; }
    .ghost-btn {
      background: none;
      border: none;
      color: var(--text-muted);
      font-family: inherit;
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      padding: 8px;
      transition: color var(--duration-fast) var(--ease-out);
    }
    .ghost-btn:hover { color: var(--text-secondary); }

    .success-ring {
      width: 72px;
      height: 72px;
      border-radius: 50%;
      background: var(--success-subtle);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--accent);
      margin-bottom: 8px;
      box-shadow: 0 0 0 12px rgba(29, 185, 84, 0.05);
    }

    @media (prefers-reduced-motion: reduce) {
      .card, .step { animation: none; }
      .bar { animation: none; }
      .bar:nth-child(1) { height: 10px; }
      .bar:nth-child(2) { height: 20px; }
      .bar:nth-child(3) { height: 16px; }
      .bar:nth-child(4) { height: 12px; }
    }
  `,
})
export class OnboardingPage {
  step = signal(0);
  clientId = signal('');
  clientSecret = signal('');
  error = signal('');
  testing = signal(false);

  constructor(private router: Router) {}

  async testAndSave() {
    this.error.set('');
    this.testing.set(true);
    try {
      await saveSpotifyCredentials(this.clientId(), this.clientSecret());
      this.step.set(3);
    } catch (e: any) {
      this.error.set(typeof e === 'string' ? e : e?.message || 'Connection failed');
    } finally {
      this.testing.set(false);
    }
  }

  finish() {
    this.router.navigate(['/input']);
  }
}
