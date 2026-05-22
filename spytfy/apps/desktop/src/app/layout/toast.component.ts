import { Component, ChangeDetectionStrategy, Injectable, signal } from '@angular/core';

export interface Toast {
  id: number;
  message: string;
  type: 'success' | 'error' | 'info';
}

@Injectable({ providedIn: 'root' })
export class ToastService {
  toasts = signal<Toast[]>([]);
  private nextId = 0;

  show(message: string, type: Toast['type'] = 'info', duration = 3500) {
    const id = this.nextId++;
    this.toasts.update(t => [...t, { id, message, type }]);
    setTimeout(() => this.dismiss(id), duration);
  }

  success(message: string) { this.show(message, 'success'); }
  error(message: string) { this.show(message, 'error', 5000); }

  dismiss(id: number) {
    this.toasts.update(t => t.filter(x => x.id !== id));
  }
}

@Component({
  selector: 'spytfy-toasts',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="toast-container">
      @for (toast of toastService.toasts(); track toast.id) {
        <div class="toast" [class]="toast.type" (click)="toastService.dismiss(toast.id)">
          <div class="toast-indicator"></div>
          @switch (toast.type) {
            @case ('success') {
              <span class="toast-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>
              </span>
            }
            @case ('error') {
              <span class="toast-icon error">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
              </span>
            }
            @default {
              <span class="toast-icon info">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
              </span>
            }
          }
          <span class="toast-msg">{{ toast.message }}</span>
        </div>
      }
    </div>
  `,
  styles: `
    .toast-container {
      position: fixed;
      bottom: 24px;
      right: 24px;
      z-index: 9999;
      display: flex;
      flex-direction: column;
      gap: 8px;
      pointer-events: none;
    }
    .toast {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 14px 20px;
      border-radius: var(--radius-md);
      background: var(--surface-700);
      color: var(--text-primary);
      font-size: 13px;
      font-weight: 500;
      pointer-events: auto;
      cursor: pointer;
      animation: slide-in var(--duration-slow) var(--ease-out) both;
      box-shadow: var(--shadow-lg);
      max-width: 400px;
      position: relative;
      overflow: hidden;
    }
    .toast-indicator {
      position: absolute;
      left: 0;
      top: 0;
      bottom: 0;
      width: 3px;
    }
    .toast.success .toast-indicator { background: var(--accent); }
    .toast.error .toast-indicator { background: var(--error); }
    .toast.info .toast-indicator { background: var(--text-muted); }
    .toast-icon {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 28px;
      height: 28px;
      border-radius: 50%;
      flex-shrink: 0;
      background: var(--success-subtle);
      color: var(--accent);
    }
    .toast-icon.error {
      background: var(--error-subtle);
      color: var(--error);
    }
    .toast-icon.info {
      background: rgba(255, 255, 255, 0.08);
      color: var(--text-secondary);
    }
    .toast-msg { line-height: 1.4; }
    .toast:hover {
      background: var(--surface-600);
    }
    @keyframes slide-in {
      from { opacity: 0; transform: translateX(24px); }
      to { opacity: 1; transform: translateX(0); }
    }
  `,
})
export class ToastComponent {
  constructor(public toastService: ToastService) {}
}
