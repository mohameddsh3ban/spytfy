import { Routes } from '@angular/router';
import { spotifyAuthGuard } from './guards/auth.guard';

export const routes: Routes = [
  { path: '', redirectTo: 'input', pathMatch: 'full' },
  {
    path: 'onboarding',
    loadComponent: () =>
      import('./pages/onboarding/onboarding.page').then(
        (m) => m.OnboardingPage
      ),
  },
  {
    path: 'input',
    canActivate: [spotifyAuthGuard],
    loadComponent: () =>
      import('./pages/input/input.page').then((m) => m.InputPage),
  },
  {
    path: 'downloads',
    canActivate: [spotifyAuthGuard],
    loadComponent: () =>
      import('./pages/downloads/downloads.page').then(
        (m) => m.DownloadsPage
      ),
  },
  {
    path: 'library',
    canActivate: [spotifyAuthGuard],
    loadComponent: () =>
      import('./pages/library/library.page').then((m) => m.LibraryPage),
  },
  {
    path: 'settings',
    loadComponent: () =>
      import('./pages/settings/settings.page').then((m) => m.SettingsPage),
  },
];
