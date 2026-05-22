import { invoke } from '@tauri-apps/api/core';
import type { Settings, SettingsPatch } from '@spytfy/models';

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings');
}

export async function updateSettings(patch: SettingsPatch): Promise<Settings> {
  return invoke<Settings>('update_settings', { patch });
}

export async function openFolder(path: string): Promise<void> {
  return invoke('open_folder', { path });
}
