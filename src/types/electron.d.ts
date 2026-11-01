import type { ElectronAPI } from "../../electron/preload";

declare global {
  interface Window {
    electronAPI?: ElectronAPI;
    showOpenFilePicker?: (options?: any) => Promise<any[]>;
    showSaveFilePicker?: (options?: any) => Promise<any>;
  }
}

export {};

