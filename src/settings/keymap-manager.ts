export type KeyActionCategory = "view" | "edit" | "tools" | "file";

export type KeyActionId =
  | "zoomIn"
  | "zoomOut"
  | "panUp"
  | "panDown"
  | "panLeft"
  | "panRight"
  | "resetView"
  | "toggle3D"
  | "rotateCw"
  | "rotateCcw"
  | "deleteSelected"
  | "undo"
  | "redo"
  | "copy"
  | "paste"
  | "duplicate"
  | "selectAll"
  | "deselectAll"
  | "toggleSelectionLock"
  | "toggleSnapping"
  | "toggleTrackLabels"
  | "togglePowerSim"
  | "toggleRuler"
  | "toggleMeasure"
  | "toggleBridge"
  | "toggleTunnel"
  | "toggleFeeder"
  | "toggleIsolator"
  | "toggleCatenary"
  | "toggleTurnoutLantern"
  | "toggleTurnoutDecoder"
  | "toggleSignal"
  | "toggleLayersSidebar"
  | "savePlan"
  | "savePlanAs"
  | "openPlan"
  | "newPlan"
  | "exportCsv"
  | "printPlan"
  | "openHelp"
  | "togglePerfMonitor";

export interface KeyActionDef {
  id: KeyActionId;
  category: KeyActionCategory;
  labelDe: string;
  labelEn: string;
  descDe: string;
  descEn: string;
  defaultKeys: string[];
}

export const KEY_ACTION_DEFS: KeyActionDef[] = [
  // Navigation & Ansicht
  {
    id: "zoomIn",
    category: "view",
    labelDe: "Ansicht vergrößern (Zoom In)",
    labelEn: "Zoom In",
    descDe: "Vergrößert die Ansicht auf der Arbeitsfläche",
    descEn: "Zooms into the canvas view",
    defaultKeys: ["+", "=", "NumpadAdd"],
  },
  {
    id: "zoomOut",
    category: "view",
    labelDe: "Ansicht verkleinern (Zoom Out)",
    labelEn: "Zoom Out",
    descDe: "Verkleinert die Ansicht auf der Arbeitsfläche",
    descEn: "Zooms out of the canvas view",
    defaultKeys: ["-", "_", "NumpadSubtract"],
  },
  {
    id: "panUp",
    category: "view",
    labelDe: "Ansicht nach oben verschieben",
    labelEn: "Pan View Up",
    descDe: "Verschiebt den Bildausschnitt nach oben (Pfeiltaste Oben)",
    descEn: "Pans the canvas viewport upwards",
    defaultKeys: ["ArrowUp"],
  },
  {
    id: "panDown",
    category: "view",
    labelDe: "Ansicht nach unten verschieben",
    labelEn: "Pan View Down",
    descDe: "Verschiebt den Bildausschnitt nach unten (Pfeiltaste Unten)",
    descEn: "Pans the canvas viewport downwards",
    defaultKeys: ["ArrowDown"],
  },
  {
    id: "panLeft",
    category: "view",
    labelDe: "Ansicht nach links verschieben",
    labelEn: "Pan View Left",
    descDe: "Verschiebt den Bildausschnitt nach links (Pfeiltaste Links)",
    descEn: "Pans the canvas viewport to the left",
    defaultKeys: ["ArrowLeft"],
  },
  {
    id: "panRight",
    category: "view",
    labelDe: "Ansicht nach rechts verschieben",
    labelEn: "Pan View Right",
    descDe: "Verschiebt den Bildausschnitt nach rechts (Pfeiltaste Rechts)",
    descEn: "Pans the canvas viewport to the right",
    defaultKeys: ["ArrowRight"],
  },
  {
    id: "resetView",
    category: "view",
    labelDe: "Ansicht zentrieren",
    labelEn: "Reset View",
    descDe: "Zentriert die Arbeitsfläche auf den Gleisplan",
    descEn: "Centers the viewport on the layout",
    defaultKeys: ["Ctrl+0"],
  },
  {
    id: "toggle3D",
    category: "view",
    labelDe: "3D-Ansicht umschalten",
    labelEn: "Toggle 3D View",
    descDe: "Wechselt zwischen 2D-Konstruktion und 3D-Ansicht",
    descEn: "Toggles between 2D planning and 3D elevation view",
    defaultKeys: ["3", "V"],
  },
  {
    id: "toggleRuler",
    category: "view",
    labelDe: "Lineale ein-/ausblenden",
    labelEn: "Toggle Rulers",
    descDe: "Schaltet die Millimeter-Lineale am Rand ein oder aus",
    descEn: "Toggles edge coordinate rulers",
    defaultKeys: ["Shift+L"],
  },
  {
    id: "toggleLayersSidebar",
    category: "view",
    labelDe: "Ebenen-Leiste umschalten",
    labelEn: "Toggle Layers Sidebar",
    descDe: "Öffnet oder schließt die rechte Seitenleiste",
    descEn: "Toggles the right sidebar panels",
    defaultKeys: ["Shift+B"],
  },

  // Bearbeiten
  {
    id: "rotateCw",
    category: "edit",
    labelDe: "Auswahl im Uhrzeigersinn drehen",
    labelEn: "Rotate Clockwise",
    descDe: "Dreht markierte Gleise um den eingestellten Winkel",
    descEn: "Rotates selected tracks clockwise by rotation step",
    defaultKeys: ["R"],
  },
  {
    id: "rotateCcw",
    category: "edit",
    labelDe: "Auswahl gegen Uhrzeigersinn drehen",
    labelEn: "Rotate Counter-Clockwise",
    descDe: "Dreht markierte Gleise rückwärts gegen den Uhrzeigersinn",
    descEn: "Rotates selected tracks counter-clockwise",
    defaultKeys: ["Shift+R"],
  },
  {
    id: "deleteSelected",
    category: "edit",
    labelDe: "Auswahl löschen",
    labelEn: "Delete Selected",
    descDe: "Löscht alle markierten Gleise aus dem Plan",
    descEn: "Deletes all selected tracks from layout",
    defaultKeys: ["Delete", "Backspace"],
  },
  {
    id: "undo",
    category: "edit",
    labelDe: "Rückgängig",
    labelEn: "Undo",
    descDe: "Macht den letzten Bearbeitungsschritt rückgängig",
    descEn: "Reverts the last editing action",
    defaultKeys: ["Ctrl+Z"],
  },
  {
    id: "redo",
    category: "edit",
    labelDe: "Wiederherstellen",
    labelEn: "Redo",
    descDe: "Stellt die zuletzt rückgängig gemachte Aktion wieder her",
    descEn: "Restores the last undone action",
    defaultKeys: ["Ctrl+Y", "Ctrl+Shift+Z"],
  },
  {
    id: "copy",
    category: "edit",
    labelDe: "Kopieren",
    labelEn: "Copy",
    descDe: "Kopiert markierte Gleise in die Zwischenablage",
    descEn: "Copies selected tracks to clipboard",
    defaultKeys: ["Ctrl+C"],
  },
  {
    id: "paste",
    category: "edit",
    labelDe: "Einfügen",
    labelEn: "Paste",
    descDe: "Fügt Gleise aus der Zwischenablage ein",
    descEn: "Pastes tracks from clipboard",
    defaultKeys: ["Ctrl+V"],
  },
  {
    id: "duplicate",
    category: "edit",
    labelDe: "Duplizieren",
    labelEn: "Duplicate",
    descDe: "Erstellt sofort eine Kopie der markierten Gleise",
    descEn: "Instantly duplicates selected tracks",
    defaultKeys: ["Ctrl+D"],
  },
  {
    id: "selectAll",
    category: "edit",
    labelDe: "Alles auswählen",
    labelEn: "Select All",
    descDe: "Wählt alle Gleise des aktuellen Plans aus",
    descEn: "Selects all tracks on the layout",
    defaultKeys: ["Ctrl+A"],
  },
  {
    id: "deselectAll",
    category: "edit",
    labelDe: "Auswahl aufheben",
    labelEn: "Deselect All",
    descDe: "Hebt die aktuelle Auswahl auf oder schließt Dialoge",
    descEn: "Clears selection or cancels active tool",
    defaultKeys: ["Escape"],
  },
  {
    id: "toggleSelectionLock",
    category: "edit",
    labelDe: "Auswahlsperre aufheben/aktivieren",
    labelEn: "Toggle Selection Lock",
    descDe: "Ermöglicht Auswahl gesperrter oder gelöschter Ebenen",
    descEn: "Allows selection of locked or deleted layers",
    defaultKeys: ["Alt+L"],
  },

  // Werkzeuge & Elektrik
  {
    id: "toggleSnapping",
    category: "tools",
    labelDe: "Einrasten (Snapping) umschalten",
    labelEn: "Toggle Snapping",
    descDe: "Aktiviert/Deaktiviert das automatische Verbinden an Gleisenden",
    descEn: "Toggles automatic connector snapping",
    defaultKeys: ["S"],
  },
  {
    id: "toggleTrackLabels",
    category: "tools",
    labelDe: "Gleisnummern anzeigen/verbergen",
    labelEn: "Toggle Track Labels",
    descDe: "Zeigt oder versteckt Artikelnummern an den Gleisen",
    descEn: "Shows or hides catalog part numbers",
    defaultKeys: ["N"],
  },
  {
    id: "togglePowerSim",
    category: "tools",
    labelDe: "Fahrstrom-Simulation umschalten",
    labelEn: "Toggle Power Simulation",
    descDe: "Schaltet die Spannungsabfall-Heatmap ein/aus",
    descEn: "Toggles voltage drop heatmap simulation",
    defaultKeys: ["P"],
  },
  {
    id: "toggleMeasure",
    category: "tools",
    labelDe: "Messwerkzeug / Gleishöhe",
    labelEn: "Measure Tool / Height",
    descDe: "Öffnet das Messwerkzeug oder den Höhendialog",
    descEn: "Opens distance measurement or elevation dialog",
    defaultKeys: ["M"],
  },
  {
    id: "toggleBridge",
    category: "tools",
    labelDe: "Brücke umschalten",
    labelEn: "Toggle Bridge",
    descDe: "Macht das markierte Gleis zu einer Brücke mit Pfeilern",
    descEn: "Toggles bridge status and pillars on selected track",
    defaultKeys: ["B"],
  },
  {
    id: "toggleTunnel",
    category: "tools",
    labelDe: "Tunnel umschalten",
    labelEn: "Toggle Tunnel",
    descDe: "Macht das markierte Gleis zu einem Tunnelabschnitt",
    descEn: "Toggles tunnel status on selected track",
    defaultKeys: ["T"],
  },
  {
    id: "toggleFeeder",
    category: "tools",
    labelDe: "Stromeinspeisung umschalten",
    labelEn: "Toggle Power Feeder",
    descDe: "Setzt einen Einspeisepunkt für den Fahrstrom",
    descEn: "Toggles track power feeder point",
    defaultKeys: ["F"],
  },
  {
    id: "toggleIsolator",
    category: "tools",
    labelDe: "Trennstelle umschalten",
    labelEn: "Toggle Track Isolator",
    descDe: "Setzt eine Trennstelle zur Blockstreckentrennung",
    descEn: "Toggles electrical track gap / isolator",
    defaultKeys: ["I"],
  },
  {
    id: "toggleCatenary",
    category: "tools",
    labelDe: "Oberleitung umschalten",
    labelEn: "Toggle Catenary",
    descDe: "Aktiviert Oberleitungsmasten und Fahrdraht",
    descEn: "Toggles overhead wire and catenary masts",
    defaultKeys: ["O"],
  },
  {
    id: "toggleTurnoutLantern",
    category: "tools",
    labelDe: "Weichenlaterne umschalten",
    labelEn: "Toggle Turnout Lantern",
    descDe: "Schaltet die beleuchtete Weichenlaterne um",
    descEn: "Toggles illuminated turnout lantern",
    defaultKeys: ["W"],
  },
  {
    id: "toggleTurnoutDecoder",
    category: "tools",
    labelDe: "Weichendecoder umschalten",
    labelEn: "Toggle Turnout Decoder",
    descDe: "Baut digitalen Weichendecoder ein/aus",
    descEn: "Installs or removes digital turnout decoder",
    defaultKeys: ["Shift+W", "Shift+D"],
  },
  {
    id: "toggleSignal",
    category: "tools",
    labelDe: "Lichtsignal umschalten",
    labelEn: "Toggle Signal",
    descDe: "Platziert ein Signal am Gleisende",
    descEn: "Places or toggles track signal",
    defaultKeys: ["Shift+S"],
  },
  {
    id: "togglePerfMonitor",
    category: "tools",
    labelDe: "Leistungsmonitor umschalten",
    labelEn: "Toggle Perf Monitor",
    descDe: "Zeigt FPS, Frame-Dauer und Objektanzahl",
    descEn: "Shows live FPS, frame time, and scene stats",
    defaultKeys: ["F3"],
  },

  // Datei
  {
    id: "savePlan",
    category: "file",
    labelDe: "Plan speichern",
    labelEn: "Save Plan",
    descDe: "Speichert den aktuellen Gleisplan",
    descEn: "Saves current layout plan",
    defaultKeys: ["Ctrl+S"],
  },
  {
    id: "savePlanAs",
    category: "file",
    labelDe: "Plan speichern unter...",
    labelEn: "Save Plan As...",
    descDe: "Speichert den Plan unter neuem Namen",
    descEn: "Saves plan under a new file name",
    defaultKeys: ["Ctrl+Shift+S"],
  },
  {
    id: "openPlan",
    category: "file",
    labelDe: "Plan öffnen",
    labelEn: "Open Plan",
    descDe: "Öffnet eine gespeicherte Plandatei",
    descEn: "Opens a saved layout file",
    defaultKeys: ["Ctrl+O"],
  },
  {
    id: "newPlan",
    category: "file",
    labelDe: "Neuer Plan",
    labelEn: "New Plan",
    descDe: "Erstellt einen neuen, leeren Gleisplan",
    descEn: "Creates a fresh, empty layout",
    defaultKeys: ["Ctrl+N"],
  },
  {
    id: "exportCsv",
    category: "file",
    labelDe: "Stückliste exportieren (CSV)",
    labelEn: "Export BOM (CSV)",
    descDe: "Exportiert Teileliste für Tabellenkalkulation",
    descEn: "Exports bill of materials to CSV",
    defaultKeys: ["Ctrl+E"],
  },
  {
    id: "printPlan",
    category: "file",
    labelDe: "Plan drucken / PDF",
    labelEn: "Print Plan / PDF",
    descDe: "Öffnet den ebenengetrennten Druckdialog",
    descEn: "Opens layer-separated print dialog",
    defaultKeys: ["Ctrl+P"],
  },
  {
    id: "openHelp",
    category: "file",
    labelDe: "Hilfe & Handbuch öffnen",
    labelEn: "Open Help Guide",
    descDe: "Öffnet das interaktive Handbuch",
    descEn: "Opens the interactive help guide",
    defaultKeys: ["F1", "?"],
  },
];

export const DEFAULT_KEYMAP: Record<KeyActionId, string[]> = KEY_ACTION_DEFS.reduce(
  (acc, def) => {
    acc[def.id] = [...def.defaultKeys];
    return acc;
  },
  {} as Record<KeyActionId, string[]>,
);

export const KEYMAP_STORAGE_KEY = "railbench_custom_keymap";

export type KeymapChangeListener = (keymap: Record<KeyActionId, string[]>) => void;
const listeners: KeymapChangeListener[] = [];

export function onKeymapChange(fn: KeymapChangeListener): () => void {
  listeners.push(fn);
  return () => {
    const idx = listeners.indexOf(fn);
    if (idx >= 0) listeners.splice(idx, 1);
  };
}

export function loadKeymap(): Record<KeyActionId, string[]> {
  try {
    if (typeof localStorage === "undefined") return { ...DEFAULT_KEYMAP };
    const raw = localStorage.getItem(KEYMAP_STORAGE_KEY);
    if (!raw) return { ...DEFAULT_KEYMAP };
    const parsed = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return { ...DEFAULT_KEYMAP };

    const result: Record<KeyActionId, string[]> = { ...DEFAULT_KEYMAP };
    for (const def of KEY_ACTION_DEFS) {
      if (Array.isArray(parsed[def.id])) {
        result[def.id] = parsed[def.id];
      }
    }
    return result;
  } catch (err) {
    console.warn("Could not load custom keymap:", err);
    return { ...DEFAULT_KEYMAP };
  }
}

export function saveKeymap(keymap: Record<KeyActionId, string[]>): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(KEYMAP_STORAGE_KEY, JSON.stringify(keymap));
    }
    for (const fn of listeners) {
      try {
        fn(keymap);
      } catch (e) {
        console.error("Keymap listener error:", e);
      }
    }
  } catch (err) {
    console.warn("Could not save custom keymap:", err);
  }
}

export function resetKeymap(): Record<KeyActionId, string[]> {
  const defaults = { ...DEFAULT_KEYMAP };
  saveKeymap(defaults);
  return defaults;
}

/**
 * Normalisiert ein Tastaturevent in einen sauberen Shortcut-String (z.B. "Ctrl+Z", "Shift+R", "+", "ArrowUp").
 */
export function eventToShortcutString(e: KeyboardEvent): string {
  // Ignoriere reine Modifikatortasten
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) {
    return "";
  }

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");

  const key = e.key;
  const code = e.code;

  let base = key;
  if (code === "NumpadAdd" || key === "+" || (code === "Equal" && e.shiftKey)) {
    base = "+";
  } else if (code === "NumpadSubtract" || key === "-" || key === "_") {
    base = "-";
  } else if (key === " ") {
    base = "Space";
  } else if (key.length === 1 && /[a-zA-Z]/.test(key)) {
    if (e.shiftKey) parts.push("Shift");
    base = key.toUpperCase();
  } else {
    if (e.shiftKey && (key.startsWith("Arrow") || key.startsWith("F") || key === "Delete" || key === "Backspace")) {
      parts.push("Shift");
    }
    base = key;
  }

  parts.push(base);
  return parts.join("+");
}

/**
 * Prüft, ob ein KeyboardEvent zu einem Shortcut-String passt.
 */
export function matchesKeyBinding(e: KeyboardEvent, shortcut: string): boolean {
  if (!shortcut) return false;

  // Zoom Sonderbehandlung
  if (shortcut === "+" || shortcut === "=" || shortcut === "NumpadAdd") {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    return e.key === "+" || e.key === "=" || e.code === "NumpadAdd" || (e.code === "Equal" && !e.ctrlKey);
  }
  if (shortcut === "-" || shortcut === "_" || shortcut === "NumpadSubtract") {
    if (e.ctrlKey || e.metaKey || e.altKey) return false;
    return e.key === "-" || e.key === "_" || e.code === "NumpadSubtract" || e.code === "Minus";
  }

  const parts = shortcut.split("+").map((s) => s.trim());
  const needCtrl = parts.some((p) => ["ctrl", "meta", "strg"].includes(p.toLowerCase()));
  const needAlt = parts.some((p) => p.toLowerCase() === "alt");
  const needShift = parts.some((p) => ["shift", "umschalt"].includes(p.toLowerCase()));

  let baseKey = "";
  if (shortcut.endsWith("++")) {
    baseKey = "+";
  } else {
    baseKey = parts[parts.length - 1]!;
  }

  const hasCtrl = Boolean(e.ctrlKey || e.metaKey);
  const hasAlt = Boolean(e.altKey);
  const hasShift = Boolean(e.shiftKey);

  if (hasCtrl !== needCtrl) return false;
  if (hasAlt !== needAlt) return false;

  const keyLower = e.key ? e.key.toLowerCase() : "";
  const baseLower = baseKey.toLowerCase();
  const code = e.code || "";

  // Wenn Buchstabe (z.B. "R"):
  if (baseKey.length === 1 && /[a-zA-Z]/.test(baseKey)) {
    if (hasShift !== needShift) return false;
    return keyLower === baseLower || code === `Key${baseKey.toUpperCase()}`;
  }

  if (needShift && !hasShift) return false;

  if (baseKey === "ArrowUp") return e.key === "ArrowUp" || code === "ArrowUp";
  if (baseKey === "ArrowDown") return e.key === "ArrowDown" || code === "ArrowDown";
  if (baseKey === "ArrowLeft") return e.key === "ArrowLeft" || code === "ArrowLeft";
  if (baseKey === "ArrowRight") return e.key === "ArrowRight" || code === "ArrowRight";

  if (baseKey === "Delete") return e.key === "Delete" || code === "Delete";
  if (baseKey === "Backspace") return e.key === "Backspace" || code === "Backspace";
  if (baseKey === "Escape") return e.key === "Escape" || code === "Escape";

  if (baseKey.startsWith("F") && baseKey.length <= 3) {
    return e.key.toUpperCase() === baseKey.toUpperCase() || code.toUpperCase() === baseKey.toUpperCase();
  }

  return keyLower === baseLower || code.toLowerCase() === baseLower;
}

/**
 * Prüft, ob ein KeyboardEvent zu einer bestimmten Action passt.
 */
export function matchesAction(
  e: KeyboardEvent,
  actionId: KeyActionId,
  keymap?: Record<KeyActionId, string[]>,
): boolean {
  const current = keymap ?? loadKeymap();
  const bindings = current[actionId];
  if (!Array.isArray(bindings) || bindings.length === 0) return false;
  return bindings.some((binding) => matchesKeyBinding(e, binding));
}
