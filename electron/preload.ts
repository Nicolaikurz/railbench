import { contextBridge, ipcRenderer } from "electron";

export interface ElectronAPI {
  isElectron: boolean;
  platform: string;
  openPlan: () => Promise<{ filePath: string; content: string; fileName: string } | null>;
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
}

const api: ElectronAPI = {
  isElectron: true,
  platform: process.platform,
  setThemeSource: (source) => ipcRenderer.send("theme:setSource", source),
  openPlan: () => ipcRenderer.invoke("dialog:openPlan"),
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
};

contextBridge.exposeInMainWorld("electronAPI", api);

