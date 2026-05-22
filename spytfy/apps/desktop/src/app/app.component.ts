import { Component, ChangeDetectionStrategy, HostListener } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { SidebarComponent } from './layout/sidebar.component';
import { ToastComponent } from './layout/toast.component';

@Component({
  selector: 'spytfy-root',
  standalone: true,
  imports: [RouterOutlet, SidebarComponent, ToastComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="shell">
      <spytfy-sidebar />
      <main class="main">
        <router-outlet />
      </main>
    </div>
    <spytfy-toasts />
  `,
  styles: `
    .shell {
      display: flex;
      height: 100vh;
      background: var(--surface-900);
      gap: 0;
    }
    .main {
      flex: 1;
      overflow: auto;
      background: var(--surface-800);
      border-radius: 8px 0 0 0;
    }
  `,
})
export class AppComponent {
  private routes = ['/input', '/downloads', '/library', '/settings'];

  constructor(private router: Router) {}

  @HostListener('window:keydown', ['$event'])
  onKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key >= '1' && e.key <= '4') {
      e.preventDefault();
      this.router.navigate([this.routes[+e.key - 1]]);
    }
  }
}
