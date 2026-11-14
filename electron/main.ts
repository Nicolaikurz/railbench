import { app, BrowserWindow, dialog, ipcMain, Menu, nativeTheme } from "electron";
import * as path from "node:path";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import { EmbeddedCollabServer } from "./collab-server";
import { MENU_EN } from "./menu-labels";

try {
  app.commandLine.appendSwitch("enable-precise-memory-info");
} catch {}

let mainWindow: BrowserWindow | null = null;
const collabServer = new EmbeddedCollabServer();

interface WindowState {
  x?: number;
  y?: number;
  width: number;
  height: number;
  isMaximized?: boolean;
}

function getWindowStatePath(): string {
  return path.join(app.getPath("userData"), "window-state.json");
}

async function loadSavedWindowState(): Promise<WindowState | null> {
  try {
    const raw = await fs.readFile(getWindowStatePath(), "utf-8");
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function saveWindowState() {
  if (!mainWindow || mainWindow.isDestroyed()) return;
  try {
    const isMaximized = mainWindow.isMaximized();
    const bounds = mainWindow.getNormalBounds ? mainWindow.getNormalBounds() : mainWindow.getBounds();
    const state: WindowState = {
      x: bounds.x,
      y: bounds.y,
      width: bounds.width,
      height: bounds.height,
      isMaximized,
    };
    fs.writeFile(getWindowStatePath(), JSON.stringify(state), "utf-8").catch(() => {});
  } catch {}
}

async function createWindow() {
  const savedState = await loadSavedWindowState();

  mainWindow = new BrowserWindow({
    width: savedState?.width || 1440,
    height: savedState?.height || 900,
    x: savedState?.x,
    y: savedState?.y,
    minWidth: 960,
    minHeight: 640,
    title: "Gleisplan – Modellbahn Planer",
    icon: path.join(__dirname, "../build/icon.png"),
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  if (savedState?.isMaximized) {
    mainWindow.maximize();
  }

  mainWindow.on("close", () => {
    saveWindowState();
  });

  mainWindow.on("resize", () => {
    saveWindowState();
  });

  mainWindow.on("move", () => {
    saveWindowState();
  });

  const isDev = process.env.NODE_ENV === "development" || !app.isPackaged;
  const devServerUrl = process.env.VITE_DEV_SERVER_URL;

  if (isDev && devServerUrl) {
    mainWindow.loadURL(devServerUrl);
    mainWindow.webContents.openDevTools();
  } else {
    // Im Produktions-Modus die gebaute index.html laden
    const indexPath = path.join(__dirname, "../dist/index.html");
    mainWindow.loadFile(indexPath);
  }

  mainWindow.on("closed", () => {
    mainWindow = null;
  });

  setupMenu();
}

let menuLanguage: "de" | "en" = app.getLocale().toLowerCase().startsWith("de") ? "de" : "en";

function setupMenu() {
  const isMac = process.platform === "darwin";
  const L = (de: string): string => (menuLanguage === "de" ? de : (MENU_EN[de] ?? de));

  if (isMac) {
    app.setAboutPanelOptions({
      applicationName: "RAILbench",
      applicationVersion: app.getVersion(),
      version: app.getVersion(),
      copyright: "Copyright © 2026 RAILbench Team",
      credits: "Modellbahn Gleisplanungs-Tool",
    });
  }

  if (!isMac) {
    // Auf Linux (inkl. Flatpak) und Windows soll keine zusätzliche native Fenster-Menüleiste
    // ("Datei Bearbeiten Ansicht Werkzeuge...") angezeigt werden, da Railbench
    // über eine eigene, moderne In-App-Toolbar und Menüs verfügt.
    Menu.setApplicationMenu(null);
    if (mainWindow) {
      mainWindow.setMenuBarVisibility(false);
      mainWindow.removeMenu();
    }
    return;
  }

  const template: Electron.MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            role: "appMenu" as const,
            submenu: [
              { role: "about" as const },
              { type: "separator" as const },
              {
                label: L("Einstellungen..."),
                accelerator: "CmdOrCtrl+,",
                click: () => mainWindow?.webContents.send("menu:action", "configPower"),
              },
              { type: "separator" as const },
              { role: "services" as const },
              { type: "separator" as const },
              { role: "hide" as const },
              { role: "hideOthers" as const },
              { role: "unhide" as const },
              { type: "separator" as const },
              { role: "quit" as const },
            ],
          },
        ]
      : []),
    {
      label: L("Gleisplan"),
      submenu: [
        {
          label: L("Neuer Plan"),
          accelerator: "CmdOrCtrl+N",
          click: () => mainWindow?.webContents.send("menu:action", "newPlan"),
        },
        {
          label: L("Plan öffnen..."),
          accelerator: "CmdOrCtrl+O",
          click: () => mainWindow?.webContents.send("menu:action", "openPlan"),
        },
        {
          label: L("Demo-Layouts..."),
          click: () => mainWindow?.webContents.send("menu:action", "demoTracks"),
        },
        { type: "separator" },
        {
          label: L("Speichern"),
          accelerator: "CmdOrCtrl+S",
          click: () => mainWindow?.webContents.send("menu:action", "savePlan"),
        },
        {
          label: L("Speichern unter..."),
          accelerator: "CmdOrCtrl+Shift+S",
          click: () => mainWindow?.webContents.send("menu:action", "savePlanAs"),
        },
        { type: "separator" },
        {
          label: L("Gleiskatalog importieren (JSON)..."),
          click: () => mainWindow?.webContents.send("menu:action", "importCatalog"),
        },
        {
          label: L("3D-Modell einfügen (.glb / .gltf)..."),
          click: () => mainWindow?.webContents.send("menu:action", "import3d"),
        },
        {
          label: L("Gleisplan als XML exportieren..."),
          click: () => mainWindow?.webContents.send("menu:action", "exportXml"),
        },
        {
          label: L("Stückliste exportieren (CSV)..."),
          accelerator: "CmdOrCtrl+E",
          click: () => mainWindow?.webContents.send("menu:action", "exportCSV"),
        },
        {
          label: L("Plan drucken (Ebenen getrennt)..."),
          accelerator: "CmdOrCtrl+P",
          click: () => mainWindow?.webContents.send("menu:action", "printPlan"),
        },
        { type: "separator" },
        {
          label: L("Live-Zusammenarbeit (Multi-User)..."),
          click: () => mainWindow?.webContents.send("menu:action", "collaborate"),
        },
        { type: "separator" },
        isMac ? { role: "close" } : { role: "quit" },
      ],
    },
    {
      label: L("Bearbeiten"),
      submenu: [
        {
          label: L("Rückgängig"),
          accelerator: "CmdOrCtrl+Z",
          click: () => mainWindow?.webContents.send("menu:action", "undo"),
        },
        {
          label: L("Wiederholen"),
          accelerator: "CmdOrCtrl+Shift+Z",
          click: () => mainWindow?.webContents.send("menu:action", "redo"),
        },
        { type: "separator" },
        {
          label: L("Kopieren"),
          accelerator: "CmdOrCtrl+C",
          click: () => mainWindow?.webContents.send("menu:action", "copy"),
        },
        {
          label: L("Einfügen"),
          accelerator: "CmdOrCtrl+V",
          click: () => mainWindow?.webContents.send("menu:action", "paste"),
        },
        {
          label: L("Duplizieren"),
          accelerator: "CmdOrCtrl+D",
          click: () => mainWindow?.webContents.send("menu:action", "duplicate"),
        },
        { type: "separator" },
        {
          label: L("Alles auswählen"),
          accelerator: "CmdOrCtrl+A",
          click: () => mainWindow?.webContents.send("menu:action", "selectAll"),
        },
        {
          label: L("Auswahl aufheben"),
          accelerator: "Esc",
          click: () => mainWindow?.webContents.send("menu:action", "deselectAll"),
        },
        { type: "separator" },
        {
          label: L("Ausgewähltes Gleis im Uhrzeigersinn drehen"),
          accelerator: "R",
          click: () => mainWindow?.webContents.send("menu:action", "rotate"),
        },
        {
          label: L("Ausgewähltes Gleis gegen den Uhrzeigersinn drehen"),
          accelerator: "Shift+R",
          click: () => mainWindow?.webContents.send("menu:action", "rotateCcw"),
        },
        {
          label: L("Ausgewähltes Gleis löschen"),
          accelerator: "Backspace",
          click: () => mainWindow?.webContents.send("menu:action", "delete"),
        },
        { type: "separator" },
        {
          label: L("Höhe anpassen / Auf Ebene verschieben..."),
          accelerator: "M",
          click: () => mainWindow?.webContents.send("menu:action", "changeHeight"),
        },
        {
          label: L("Einspeisepunkt umschalten"),
          accelerator: "F",
          click: () => mainWindow?.webContents.send("menu:action", "toggleFeeder"),
        },
      ],
    },
    {
      label: L("Ansicht"),
      submenu: [
        {
          label: L("Ansicht zentrieren"),
          accelerator: "CmdOrCtrl+0",
          click: () => mainWindow?.webContents.send("menu:action", "resetView"),
        },
        {
          label: L("3D-Ansicht umschalten"),
          accelerator: "3",
          click: () => mainWindow?.webContents.send("menu:action", "toggle3D"),
        },
        { type: "separator" },
        {
          label: L("Lineale ein/aus"),
          accelerator: "Shift+L",
          click: () => mainWindow?.webContents.send("menu:action", "toggleRulers"),
        },
        {
          label: L("Trassenbretter / Unterbau ein/aus"),
          click: () => mainWindow?.webContents.send("menu:action", "toggleWoodPlate"),
        },
        { type: "separator" },
        {
          label: L("Design: System"),
          click: () => mainWindow?.webContents.send("menu:action", "themeSystem"),
        },
        {
          label: L("Design: Dunkel"),
          click: () => mainWindow?.webContents.send("menu:action", "themeDark"),
        },
        {
          label: L("Design: Hell"),
          click: () => mainWindow?.webContents.send("menu:action", "themeLight"),
        },
        { type: "separator" },
        {
          label: L("Performance-Monitor ein/aus"),
          accelerator: "F3",
          click: () => mainWindow?.webContents.send("menu:action", "togglePerfMonitor"),
        },
        {
          label: L("Menüleisten fixieren"),
          click: () => mainWindow?.webContents.send("menu:action", "lockBars"),
        },
        {
          label: L("UI-Größen zurücksetzen"),
          click: () => mainWindow?.webContents.send("menu:action", "resetUiSizes"),
        },
        { type: "separator" },
        { role: "reload" },
        { role: "forceReload" },
        { role: "toggleDevTools" },
        { type: "separator" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
        { type: "separator" },
        { role: "togglefullscreen" },
      ],
    },
    {
      label: L("Einstellungen"),
      submenu: [
        {
          label: L("Snapping ein/aus"),
          accelerator: "S",
          click: () => mainWindow?.webContents.send("menu:action", "toggleSnapping"),
        },
        {
          label: L("Gleisnummern ein/aus"),
          accelerator: "N",
          click: () => mainWindow?.webContents.send("menu:action", "toggleTrackLabels"),
        },
        {
          label: L("Power-Drop Simulation ein/aus"),
          accelerator: "P",
          click: () => mainWindow?.webContents.send("menu:action", "togglePowerSim"),
        },
        {
          label: L("Farbcodierung umschalten"),
          click: () => mainWindow?.webContents.send("menu:action", "toggleColorCode"),
        },
        { type: "separator" },
        {
          label: L("Stromversorgung & Spannung konfigurieren..."),
          click: () => mainWindow?.webContents.send("menu:action", "configPower"),
        },
        { type: "separator" },
        {
          label: L("Sprache: Automatisch (System)"),
          click: () => mainWindow?.webContents.send("menu:action", "langAuto"),
        },
        {
          label: L("Sprache: Deutsch"),
          click: () => mainWindow?.webContents.send("menu:action", "langDe"),
        },
        {
          label: L("Sprache: English"),
          click: () => mainWindow?.webContents.send("menu:action", "langEn"),
        },
        { type: "separator" },
        {
          label: L("Persönliche Konfiguration zurücksetzen..."),
          click: () => mainWindow?.webContents.send("menu:action", "resetPersonalConfig"),
        },
      ],
    },
    ...(isMac ? [{ role: "windowMenu" as const }] : []),
    {
      label: L("Hilfe"),
      submenu: [
        {
          label: L("Anleitung & Funktionen (Help Guide)..."),
          accelerator: "F1",
          click: () => mainWindow?.webContents.send("menu:action", "help"),
        },
        {
          label: L("Tastatur-Kurzbefehle..."),
          accelerator: "?",
          click: () => mainWindow?.webContents.send("menu:action", "shortcuts"),
        },
        { type: "separator" },
        {
          label: L("Willkommens-Bildschirm..."),
          click: () => mainWindow?.webContents.send("menu:action", "welcome"),
        },
        { type: "separator" },
        {
          label: L("Ebenen-Handbuch..."),
          click: () => mainWindow?.webContents.send("menu:action", "layersGuide"),
        },
        {
          label: L("Stromversorgungs-Handbuch..."),
          click: () => mainWindow?.webContents.send("menu:action", "voltageGuide"),
        },
        {
          label: L("Druck-Handbuch..."),
          click: () => mainWindow?.webContents.send("menu:action", "printGuide"),
        },
        { type: "separator" },
        {
          label: L("Über RAILbench..."),
          click: () => mainWindow?.webContents.send("menu:action", "about"),
        },
      ],
    },
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
}

// IPC Handlers
ipcMain.on("menu:setLanguage", (_event, lang: string) => {
  const next = lang === "de" ? "de" : "en";
  if (next === menuLanguage) return;
  menuLanguage = next;
  setupMenu();
});
ipcMain.handle("dialog:openPlan", async () => {
  if (!mainWindow) return null;
  const res = await dialog.showOpenDialog(mainWindow, {
    title: "Gleisplan öffnen",
    filters: [
      {
        name: "Gleispläne (*.railbench, *.json, *.xml)",
        extensions: ["railbench", "json", "xml"],
      },
      { name: "Gleisplan JSON (*.railbench, *.json)", extensions: ["railbench", "json"] },
      { name: "Gleisplan XML (*.xml)", extensions: ["xml"] },
      { name: "Alle Dateien (*.*)", extensions: ["*"] },
    ],
    properties: ["openFile"],
  });

  if (res.canceled || res.filePaths.length === 0) return null;

  const filePath = res.filePaths[0]!;
  const content = await fs.readFile(filePath, "utf-8");
  return {
    filePath,
    content,
    fileName: path.basename(filePath),
  };
});

ipcMain.handle("dialog:openCatalog", async () => {
  if (!mainWindow) return null;
  const res = await dialog.showOpenDialog(mainWindow, {
    title: "Gleiskatalog importieren (JSON)",
    filters: [
      { name: "Gleiskatalog JSON (*.json)", extensions: ["json"] },
      { name: "Alle Dateien (*.*)", extensions: ["*"] },
    ],
    properties: ["openFile"],
  });

  if (res.canceled || res.filePaths.length === 0) return null;

  const filePath = res.filePaths[0]!;
  const content = await fs.readFile(filePath, "utf-8");
  return {
    filePath,
    content,
    fileName: path.basename(filePath),
  };
});

ipcMain.handle(
  "dialog:savePlanAs",
  async (_event, defaultName: string, content: string, format: "json" | "xml") => {
    if (!mainWindow) return null;

    const ext = format === "xml" ? "xml" : "railbench";
    const defaultPath = defaultName.endsWith(`.${ext}`)
      ? defaultName
      : `${defaultName}.${ext}`;

    const res = await dialog.showSaveDialog(mainWindow, {
      title: "Gleisplan speichern unter...",
      defaultPath,
      filters:
        format === "xml"
          ? [{ name: "XML Gleisplan (*.xml)", extensions: ["xml"] }]
          : [
              { name: "Gleisplan Datei (*.railbench)", extensions: ["railbench"] },
              { name: "JSON Datei (*.json)", extensions: ["json"] },
            ],
    });

    if (res.canceled || !res.filePath) return null;

    await fs.writeFile(res.filePath, content, "utf-8");
    return {
      filePath: res.filePath,
      fileName: path.basename(res.filePath),
    };
  },
);

ipcMain.handle("file:saveDirect", async (_event, filePath: string, content: string) => {
  await fs.writeFile(filePath, content, "utf-8");
  return {
    success: true,
    filePath,
    fileName: path.basename(filePath),
  };
});

ipcMain.handle("file:exportCSV", async (_event, defaultName: string, content: string) => {
  if (!mainWindow) return null;

  const res = await dialog.showSaveDialog(mainWindow, {
    title: "Stückliste exportieren",
    defaultPath: defaultName,
    filters: [{ name: "CSV Tabelle (*.csv)", extensions: ["csv"] }],
  });

  if (res.canceled || !res.filePath) return null;

  await fs.writeFile(res.filePath, content, "utf-8");
  return { success: true, filePath: res.filePath };
});

ipcMain.handle("file:exportPDF", async (_event, defaultName: string, landscape: boolean) => {
  if (!mainWindow) return null;

  const defaultPath = defaultName.endsWith(".pdf") ? defaultName : `${defaultName}.pdf`;

  const res = await dialog.showSaveDialog(mainWindow, {
    title: "Gleisplan als PDF exportieren",
    defaultPath,
    filters: [{ name: "PDF Dokument (*.pdf)", extensions: ["pdf"] }],
  });

  if (res.canceled || !res.filePath) return null;

  const pdfBuffer = await mainWindow.webContents.printToPDF({
    landscape,
    pageSize: "A4",
    printBackground: true,
    margins: {
      top: 0,
      bottom: 0,
      left: 0,
      right: 0,
    },
  });

  await fs.writeFile(res.filePath, pdfBuffer);
  return { success: true, filePath: res.filePath };
});

ipcMain.on("window:setTitle", (_event, title: string) => {
  if (mainWindow) {
    mainWindow.setTitle(title);
  }
});

ipcMain.on("theme:setSource", (_event, source: "system" | "dark" | "light") => {
  try {
    nativeTheme.themeSource = source;
  } catch (err) {
    console.warn("Could not set nativeTheme.themeSource", err);
  }
});

ipcMain.on("window:close", (event) => {
  const win = BrowserWindow.fromWebContents(event.sender);
  win?.close();
});

interface ServerLaunchConfig {
  shouldAutoStart: boolean;
  port: number;
  password?: string;
  defaultRoom: string;
  autoConnect: boolean;
}

function parseServerLaunchConfig(): ServerLaunchConfig {
  const argv = process.argv;
  const env = process.env;

  const hasFlag = (flags: string[]): boolean => {
    return flags.some((f) => argv.includes(f) || argv.some((a) => a.startsWith(`${f}=`)));
  };

  const getOption = (flags: string[]): string | undefined => {
    for (const f of flags) {
      const idx = argv.indexOf(f);
      if (idx !== -1 && argv[idx + 1] && !argv[idx + 1].startsWith("--")) {
        return argv[idx + 1];
      }
      const prefix = `${f}=`;
      const match = argv.find((a) => a.startsWith(prefix));
      if (match) return match.substring(prefix.length);
    }
    return undefined;
  };

  const shouldAutoStart =
    hasFlag(["--server", "--collab-server", "--start-server", "--autostart-server", "--serve"]) ||
    env.RAILBENCH_SERVER === "1" ||
    env.RAILBENCH_SERVER === "true" ||
    env.RAILBENCH_AUTOSTART_SERVER === "1" ||
    env.RAILBENCH_AUTOSTART_SERVER === "true" ||
    env.COLLAB_AUTOSTART === "1" ||
    env.COLLAB_AUTOSTART === "true";

  const rawPort =
    getOption(["--port", "--server-port", "--collab-port"]) ||
    env.RAILBENCH_SERVER_PORT ||
    env.COLLAB_PORT ||
    env.PORT;
  const port = rawPort ? parseInt(rawPort, 10) : 3000;

  const password =
    getOption(["--password", "--server-password", "--collab-password"]) ||
    env.RAILBENCH_SERVER_PASSWORD ||
    env.COLLAB_PASSWORD ||
    env.PASSWORD ||
    undefined;

  const defaultRoom =
    getOption(["--room", "--server-room", "--collab-room"]) ||
    env.RAILBENCH_SERVER_ROOM ||
    env.COLLAB_ROOM ||
    env.ROOM ||
    "default";

  const autoConnect =
    hasFlag(["--connect", "--auto-connect", "--client"]) ||
    env.RAILBENCH_CONNECT === "1" ||
    env.RAILBENCH_CONNECT === "true" ||
    env.COLLAB_AUTO_CONNECT === "1";

  return {
    shouldAutoStart,
    port: isNaN(port) ? 3000 : port,
    password: password ? password.trim() : undefined,
    defaultRoom: defaultRoom ? defaultRoom.trim() : "default",
    autoConnect,
  };
}

let serverLaunchConfig: ServerLaunchConfig | null = null;

// Collab Server IPC Handlers
ipcMain.handle("collab:startServer", async (_event, options) => {
  return await collabServer.start(options);
});

ipcMain.handle("collab:stopServer", async () => {
  return await collabServer.stop();
});

ipcMain.handle("collab:getServerStatus", async () => {
  return collabServer.getStatus();
});

ipcMain.handle("collab:getLaunchConfig", async () => {
  return serverLaunchConfig || parseServerLaunchConfig();
});

ipcMain.handle("system:getMemoryUsage", () => {
  const totalMemMb = os.totalmem() / (1024 * 1024);
  const freeMemMb = os.freemem() / (1024 * 1024);
  const usedMemMb = totalMemMb - freeMemMb;

  try {
    const metrics = app.getAppMetrics();
    let appWorkingSetKb = 0;
    let appPrivateKb = 0;
    for (const m of metrics) {
      if (m.memory) {
        appWorkingSetKb += m.memory.workingSetSize || 0;
        appPrivateKb += m.memory.privateBytes || m.memory.workingSetSize || 0;
      }
    }
    const appRamMb = (appWorkingSetKb > 0 ? appWorkingSetKb : appPrivateKb) / 1024;
    return {
      appRamMb: appRamMb > 0 ? appRamMb : process.memoryUsage().rss / (1024 * 1024),
      systemUsedMb: usedMemMb,
      systemTotalMb: totalMemMb,
    };
  } catch {
    return {
      appRamMb: process.memoryUsage().rss / (1024 * 1024),
      systemUsedMb: usedMemMb,
      systemTotalMb: totalMemMb,
    };
  }
});

app.whenReady().then(async () => {
  serverLaunchConfig = parseServerLaunchConfig();
  if (serverLaunchConfig.shouldAutoStart) {
    try {
      const status = await collabServer.start({
        port: serverLaunchConfig.port,
        password: serverLaunchConfig.password,
        defaultRoom: serverLaunchConfig.defaultRoom,
      });
      console.log(`[CollabServer] Auto-started on port ${status.port} (Room: ${status.defaultRoom})`);
    } catch (err) {
      console.error("[CollabServer] Auto-start failed:", err);
    }
  }

  createWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("before-quit", async () => {
  saveWindowState();
  await collabServer.stop();
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});

