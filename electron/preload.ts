import { contextBridge, ipcRenderer } from "electron";
import * as os from "node:os";

export interface ElectronAPI {
  isElectron: boolean;
  platform: string;
  openPlan: () => Promise<{ filePath: string; content: string; fileName: string } | null>;
  openCatalog?: () => Promise<{ filePath: string; content: string; fileName: string } | null>;
  savePlanAs: (
    defaultName: string,
    content: string,
    format: "json" | "xml",
  ) => Promise<{ filePath: string; fileName: string } | null>;
  saveDirect: (
    filePath: string,
    content: string,
  ) => Promise<{ success: boolean; filePath: string; fileName: string }>;
  exportCSV: (
    defaultName: string,
    content: string,
  ) => Promise<{ success: boolean; filePath: string } | null>;
  exportPDF: (
    defaultName: string,
    landscape: boolean,
  ) => Promise<{ success: boolean; filePath: string } | null>;
  onMenuAction: (callback: (action: string) => void) => () => void;
  setWindowTitle: (title: string) => void;
  setMenuLanguage?: (lang: string) => void;
  closeWindow?: () => void;
  startCollabServer: (options?: { port?: number; password?: string; defaultRoom?: string }) => Promise<{
    running: boolean;
    port?: number;
    localUrl?: string;
    networkUrls?: string[];
    wsUrl?: string;
    networkWsUrls?: string[];
    password?: string;
    defaultRoom?: string;
    connectionsCount?: number;
    roomsCount?: number;
  }>;
  stopCollabServer: () => Promise<{
    running: boolean;
  }>;
  getCollabServerStatus: () => Promise<{
    running: boolean;
    port?: number;
    localUrl?: string;
    networkUrls?: string[];
    wsUrl?: string;
    networkWsUrls?: string[];
    password?: string;
    defaultRoom?: string;
    connectionsCount?: number;
    roomsCount?: number;
  }>;
  getServerLaunchConfig?: () => Promise<{
    shouldAutoStart: boolean;
    port: number;
    password?: string;
    defaultRoom: string;
    autoConnect: boolean;
  }>;
  setThemeSource?: (source: "system" | "dark" | "light") => void;
  getMemoryUsage?: () => Promise<{
    appRamMb: number;
    systemUsedMb: number;
    systemTotalMb: number;
  }>;
  getProcessMemoryInfo?: () => {
    rssMb: number;
    heapUsedMb: number;
    heapTotalMb: number;
    externalMb: number;
    systemUsedMb: number;
    systemTotalMb: number;
  } | null;
}

const api: ElectronAPI = {
  isElectron: true,
  platform: process.platform,
  setThemeSource: (source) => ipcRenderer.send("theme:setSource", source),
  openPlan: () => ipcRenderer.invoke("dialog:openPlan"),
  openCatalog: () => ipcRenderer.invoke("dialog:openCatalog"),
  savePlanAs: (defaultName, content, format) =>
    ipcRenderer.invoke("dialog:savePlanAs", defaultName, content, format),
  saveDirect: (filePath, content) =>
    ipcRenderer.invoke("file:saveDirect", filePath, content),
  exportCSV: (defaultName, content) =>
    ipcRenderer.invoke("file:exportCSV", defaultName, content),
  exportPDF: (defaultName, landscape) =>
    ipcRenderer.invoke("file:exportPDF", defaultName, landscape),
  onMenuAction: (callback) => {
    const handler = (_event: unknown, action: string) => callback(action);
    ipcRenderer.on("menu:action", handler);
    return () => {
      ipcRenderer.removeListener("menu:action", handler);
    };
  },
  setMenuLanguage: (lang: string) => ipcRenderer.send("menu:setLanguage", lang),
  setWindowTitle: (title: string) => {
    ipcRenderer.send("window:setTitle", title);
  },
  closeWindow: () => {
    ipcRenderer.send("window:close");
  },
  startCollabServer: (options) => ipcRenderer.invoke("collab:startServer", options),
  stopCollabServer: () => ipcRenderer.invoke("collab:stopServer"),
  getCollabServerStatus: () => ipcRenderer.invoke("collab:getServerStatus"),
  getServerLaunchConfig: () => ipcRenderer.invoke("collab:getLaunchConfig"),
  getMemoryUsage: () => ipcRenderer.invoke("system:getMemoryUsage"),
  getProcessMemoryInfo: () => {
    try {
      const mem = process.memoryUsage();
      const totalMb = os.totalmem() / (1024 * 1024);
      const freeMb = os.freemem() / (1024 * 1024);
      return {
        rssMb: mem.rss / (1024 * 1024),
        heapUsedMb: mem.heapUsed / (1024 * 1024),
        heapTotalMb: mem.heapTotal / (1024 * 1024),
        externalMb: ((mem.external || 0) + (mem.arrayBuffers || 0)) / (1024 * 1024),
        systemUsedMb: totalMb - freeMb,
        systemTotalMb: totalMb,
      };
    } catch {
      return null;
    }
  },
};

contextBridge.exposeInMainWorld("electronAPI", api);

