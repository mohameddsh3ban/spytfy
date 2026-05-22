import { Component, ChangeDetectionStrategy, signal } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';

interface NavItem {
  path: string;
  label: string;
  icon: string;
  activeIcon: string;
}

@Component({
  selector: 'spytfy-sidebar',
  standalone: true,
  imports: [RouterLink, RouterLinkActive],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <nav class="sidebar" [style.width.px]="collapsed() ? 72 : 240">
      <div class="logo-area">
        <div class="logo-mark">
          <div class="soundbars">
            <span class="bar"></span>
            <span class="bar"></span>
            <span class="bar"></span>
          </div>
        </div>
        @if (!collapsed()) {
          <span class="logo-text">Spytfy</span>
        }
      </div>

      <div class="nav-items">
        @for (item of navItems; track item.path) {
          <a
            [routerLink]="item.path"
            routerLinkActive="active"
            #rla="routerLinkActive"
            class="nav-item"
            [class.collapsed]="collapsed()"
            [title]="collapsed() ? item.label : ''"
          >
            <span class="nav-icon" [innerHTML]="rla.isActive ? getIcon(item.activeIcon) : getIcon(item.icon)"></span>
            @if (!collapsed()) {
              <span class="nav-label">{{ item.label }}</span>
            }
          </a>
        }
      </div>

      <button class="toggle" (click)="collapsed.set(!collapsed())" [title]="collapsed() ? 'Expand' : 'Collapse'">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          @if (collapsed()) {
            <path d="M6 3l5 5-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          } @else {
            <path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          }
        </svg>
      </button>
    </nav>
  `,
  styles: `
    .sidebar {
      display: flex;
      flex-direction: column;
      height: 100%;
      background: var(--surface-900);
      transition: width 200ms var(--ease-in-out);
      overflow: hidden;
      flex-shrink: 0;
      padding: 8px;
    }
    .logo-area {
      display: flex;
      align-items: center;
      gap: 10px;
      height: 56px;
      padding: 0 12px;
      margin-bottom: 8px;
    }
    .logo-mark {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      flex-shrink: 0;
    }
    .soundbars {
      display: flex;
      align-items: flex-end;
      gap: 3px;
      height: 20px;
    }
    .bar {
      display: block;
      width: 4px;
      border-radius: 2px;
      background: var(--accent);
    }
    .bar:nth-child(1) {
      height: 8px;
      animation: soundbar 1.2s ease-in-out infinite;
      animation-delay: 0s;
    }
    .bar:nth-child(2) {
      height: 16px;
      animation: soundbar 1.2s ease-in-out infinite;
      animation-delay: 0.2s;
    }
    .bar:nth-child(3) {
      height: 12px;
      animation: soundbar 1.2s ease-in-out infinite;
      animation-delay: 0.4s;
    }
    @keyframes soundbar {
      0%, 100% { height: 4px; }
      50% { height: 18px; }
    }
    .logo-text {
      font-family: 'Space Grotesk', sans-serif;
      color: var(--text-primary);
      font-weight: 800;
      font-size: 22px;
      letter-spacing: -0.04em;
      white-space: nowrap;
    }
    .nav-items {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    .nav-item {
      position: relative;
      display: flex;
      align-items: center;
      gap: 16px;
      padding: 10px 12px;
      border-radius: var(--radius-md);
      color: var(--text-secondary);
      text-decoration: none;
      transition: color var(--duration-fast) var(--ease-out),
                  background var(--duration-fast) var(--ease-out);
      cursor: pointer;
      white-space: nowrap;
    }
    .nav-item:hover {
      color: var(--text-primary);
    }
    .nav-item.active {
      color: var(--text-primary);
      font-weight: 600;
    }
    .nav-item.collapsed {
      justify-content: center;
      padding: 10px;
    }
    .nav-icon {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 24px;
      height: 24px;
      flex-shrink: 0;
    }
    .nav-label {
      font-size: 14px;
      font-weight: 500;
    }
    .nav-item.active .nav-label {
      font-weight: 700;
    }
    .toggle {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      margin: 8px auto;
      border: none;
      border-radius: 50%;
      background: var(--surface-600);
      color: var(--text-secondary);
      cursor: pointer;
      transition: color var(--duration-fast) var(--ease-out),
                  background var(--duration-fast) var(--ease-out),
                  transform var(--duration-fast) var(--ease-out);
    }
    .toggle:hover {
      color: var(--text-primary);
      background: var(--surface-500);
      transform: scale(1.1);
    }
    @media (prefers-reduced-motion: reduce) {
      .bar { animation: none; }
      .bar:nth-child(1) { height: 8px; }
      .bar:nth-child(2) { height: 16px; }
      .bar:nth-child(3) { height: 12px; }
    }
  `,
})
export class SidebarComponent {
  collapsed = signal(false);

  private icons: Record<string, SafeHtml> = {};

  private svgMap: Record<string, string> = {
    'home': '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>',
    'home-filled': '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M12.707 2.293a1 1 0 0 0-1.414 0l-9 9A1 1 0 0 0 3 13h1v7a2 2 0 0 0 2 2h4v-6h4v6h4a2 2 0 0 0 2-2v-7h1a1 1 0 0 0 .707-1.707l-9-9Z"/></svg>',
    'arrow-down': '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>',
    'arrow-down-filled': '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M19 21H5a3 3 0 0 1-3-3v-4a1 1 0 0 1 2 0v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4a1 1 0 0 1 2 0v4a3 3 0 0 1-3 3ZM12 16a1 1 0 0 1-.707-.293l-5-5a1 1 0 0 1 1.414-1.414L11 12.586V4a1 1 0 0 1 2 0v8.586l3.293-3.293a1 1 0 0 1 1.414 1.414l-5 5A1 1 0 0 1 12 16Z"/></svg>',
    'library': '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>',
    'library-filled': '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M21 3a1 1 0 0 0-1.168-.986l-12 2A1 1 0 0 0 7 5v9.268A4 4 0 1 0 9 18V7.82l10-1.667v7.115A4 4 0 1 0 21 17V3Z"/></svg>',
    'settings': '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>',
    'settings-filled': '<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" stroke="none"><path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"/></svg>',
  };

  constructor(private sanitizer: DomSanitizer) {
    for (const [key, svg] of Object.entries(this.svgMap)) {
      this.icons[key] = this.sanitizer.bypassSecurityTrustHtml(svg);
    }
  }

  navItems: NavItem[] = [
    { path: '/input', label: 'Home', icon: 'home', activeIcon: 'home-filled' },
    { path: '/downloads', label: 'Downloads', icon: 'arrow-down', activeIcon: 'arrow-down-filled' },
    { path: '/library', label: 'Library', icon: 'library', activeIcon: 'library-filled' },
    { path: '/settings', label: 'Settings', icon: 'settings', activeIcon: 'settings-filled' },
  ];

  getIcon(key: string): SafeHtml {
    return this.icons[key] ?? '';
  }
}
