import { catalogRegistry } from "./registry";
import type { Catalog } from "./types";
import { openFileDialog } from "@/io/export";
import { getLanguage } from "@/i18n";

export interface CatalogImportOptions {
  onSuccess: (catalog: Catalog) => void;
  showToast: (msg: string, icon?: string) => void;
}

declare global {
  interface Window {
    showOpenFilePicker?: (options?: any) => Promise<any[]>;
  }
}

/**
 * Öffnet einen Dateidialog zum Importieren benutzerdefinierter Gleiskataloge (.json).
 * Unterstützt Electron-native Dialoge, showOpenFilePicker und HTML-FileInput.
 */
export async function openCatalogImportDialog(options: {
  onSuccess: (catalog: Catalog) => void;
  showToast: (msg: string, icon?: string) => void;
}): Promise<void> {
  const isDe = getLanguage() === "de";

  const handleContent = (content: string, filename: string) => {
    try {
      const imported = catalogRegistry.loadFromJSON(content);
      options.onSuccess(imported);
      options.showToast(
        isDe
          ? `Gleiskatalog „${imported.name}“ (${imported.entries.length} Gleise) erfolgreich geladen.`
          : `Track catalog "${imported.name}" (${imported.entries.length} tracks) loaded successfully.`,
        "✅",
      );
    } catch (err) {
      console.error(`Fehler beim Import von ${filename}:`, err);
      options.showToast(
        isDe
          ? `Fehler beim Import von ${filename}: ${(err as Error).message}`
          : `Error importing ${filename}: ${(err as Error).message}`,
        "⚠️",
      );
    }
  };

  // 1. Electron-nativer Dialog (vollständige Unterstützung ohne Sandbox/Browser-Einschränkung)
  if (window.electronAPI?.openCatalog) {
    try {
      const res = await window.electronAPI.openCatalog();
      if (res && res.content) {
        handleContent(res.content, res.fileName);
      }
      return;
    } catch (err) {
      console.warn("Electron openCatalog failed, falling back:", err);
    }
  }

  // 2. Modern Browser File System Access API
  if (typeof window !== "undefined" && typeof window.showOpenFilePicker === "function") {
    try {
      const [handle] = await window.showOpenFilePicker({
        types: [
          {
            description: isDe ? "Gleiskatalog JSON (*.json)" : "Track Catalog JSON (*.json)",
            accept: {
              "application/json": [".json"],
              "text/plain": [".json"],
            },
          },
        ],
        multiple: false,
      });

      if (handle) {
        const file = await handle.getFile();
        const content = await file.text();
        handleContent(content, file.name);
        return;
      }
    } catch (err: any) {
      if (err.name === "AbortError") return;
      console.warn("showOpenFilePicker failed, falling back to file dialog:", err);
    }
  }

  // 3. Fallback: Datei-Input
  openFileDialog(".json,application/json,text/plain,*/*", (content, filename) => {
    handleContent(content, filename);
  });
}
