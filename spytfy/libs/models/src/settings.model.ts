export interface Settings {
  outputRoot: string;
  concurrency: number;
  bitrateKbps: number;
  overwriteExisting: boolean;
  writeCoverJpg: boolean;
  namingTemplate: string;
}

export type SettingsPatch = Partial<Settings>;
