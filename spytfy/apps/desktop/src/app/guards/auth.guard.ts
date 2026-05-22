import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { hasSpotifyCredentials } from '@spytfy/tauri-ipc';

export const spotifyAuthGuard: CanActivateFn = async () => {
  const router = inject(Router);
  try {
    const hasCreds = await hasSpotifyCredentials();
    if (!hasCreds) {
      router.navigate(['/onboarding']);
      return false;
    }
    return true;
  } catch {
    return true;
  }
};
