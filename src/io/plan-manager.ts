import type { Layout } from "@/core/types";
import { toJSON, fromJSON, toXML, fromXML } from "./serialize";
import { downloadFile, openFileDialog } from "./export";

export interface PlanFileState {
  filePath: string | null;
  fileName: string | null;
  format: "gleisplan" | "json" | "xml";
  isDirty: boolean;
}

export class PlanManager {
  private state: PlanFileState = {
    filePath: null,
    fileName: null,
    format: "gleisplan",
    isDirty: false,
  };

  private onStateChange?: (state: PlanFileState) => void;

  constructor(onStateChange?: (state: PlanFileState) => void) {
    this.onStateChange = onStateChange;
  }

  getState(): PlanFileState {
    return { ...this.state };
  }

  markDirty(isDirty = true) {
    this.state.isDirty = isDirty;
    this.updateTitle();
    this.onStateChange?.(this.getState());
  }

  setPlanInfo(name: string, filePath: string | null = null, format: "gleisplan" | "json" | "xml" = "gleisplan") {
    this.state.fileName = name;
    this.state.filePath = filePath;
    this.state.format = format;
    this.state.isDirty = false;
    this.updateTitle();
    this.onStateChange?.(this.getState());
  }

  private updateTitle() {
    const dirtyMark = this.state.isDirty ? " ●" : "";
    const name = this.state.fileName || "New Layout";
    const title = `Railbench – [${name}]${dirtyMark}`;
    document.title = title;

    if (window.electronAPI) {
      window.electronAPI.setWindowTitle(title);
    }
  }

  /**
   * Opens a plan via native Electron dialog or browser file picker.
   */
  async openPlan(): Promise<{ layout: Layout; fileName: string; filePath: string | null } | null> {
    // 1. Electron
    if (window.electronAPI) {
      const res = await window.electronAPI.openPlan();
      if (!res) return null;

      const layout = this.parseContent(res.content, res.fileName);
      this.setPlanInfo(
        res.fileName,
        res.filePath,
        res.fileName.endsWith(".xml") ? "xml" : "gleisplan",
      );
      this.saveToLocalStorage(layout);
      return { layout, fileName: res.fileName, filePath: res.filePath };
    }

    // 2. Modern Browser File System Access API
    if (typeof window.showOpenFilePicker === "function") {
      try {
        const [handle] = await window.showOpenFilePicker({
          types: [
            {
              description: "Model Railway Track Plans",
              accept: {
                "application/json": [".gleisplan", ".json"],
                "application/xml": [".xml"],
              },
            },
          ],
          multiple: false,
        });

        if (handle) {
          const file = await handle.getFile();
          const content = await file.text();
          const layout = this.parseContent(content, file.name);
          this.setPlanInfo(
            file.name,
            null,
            file.name.endsWith(".xml") ? "xml" : "gleisplan",
          );
          this.saveToLocalStorage(layout);
          return { layout, fileName: file.name, filePath: null };
        }
      } catch (err: any) {
        if (err.name === "AbortError") return null;
        console.warn("showOpenFilePicker error, falling back to standard dialog:", err);
      }
    }

    // 3. Fallback via input[type=file]
    return new Promise((resolve) => {
      openFileDialog(".gleisplan,.json,.xml", (content, filename) => {
        try {
          const layout = this.parseContent(content, filename);
          this.setPlanInfo(
            filename,
            null,
            filename.endsWith(".xml") ? "xml" : "gleisplan",
          );
          this.saveToLocalStorage(layout);
          resolve({ layout, fileName: filename, filePath: null });
        } catch (e) {
          alert(`Failed to open file: ${(e as Error).message}`);
          resolve(null);
        }
      });
    });
  }

  /**
   * Speichert den aktuellen Plan. Falls bereits ein Speicherort bekannt ist, wird direkt gespeichert,
   * sonst wird "Speichern unter..." aufgerufen.
   */
  async savePlan(layout: Layout): Promise<boolean> {
    if (this.state.filePath && window.electronAPI) {
      const content =
        this.state.format === "xml" ? toXML(layout) : toJSON(layout);
      const res = await window.electronAPI.saveDirect(this.state.filePath, content);
      if (res.success) {
        this.markDirty(false);
        this.saveToLocalStorage(layout);
        return true;
      }
    }
    return this.savePlanAs(layout, this.state.format);
  }

  /**
   * Speichert den Plan unter einem neuen Namen / Pfad.
   */
  async savePlanAs(
    layout: Layout,
    format: "gleisplan" | "json" | "xml" = "gleisplan",
  ): Promise<boolean> {
    const rawName = (layout.name || "Gleisplan").replace(/[/\\?%*:|"<>]/g, "_");
    const content = format === "xml" ? toXML(layout) : toJSON(layout);
    const mime = format === "xml" ? "application/xml" : "application/json";
    const ext = format === "xml" ? "xml" : "gleisplan";
    const defaultName = rawName.endsWith(`.${ext}`) ? rawName : `${rawName}.${ext}`;

    // 1. Electron-Pfad
    if (window.electronAPI) {
      const res = await window.electronAPI.savePlanAs(
        defaultName,
        content,
        format === "xml" ? "xml" : "json",
      );
      if (res) {
        this.setPlanInfo(res.fileName, res.filePath, format);
        this.saveToLocalStorage(layout);
        return true;
      }
      return false;
    }

    // 2. Moderner Browser File System Access API
    if (typeof window.showSaveFilePicker === "function") {
      try {
        const handle = await window.showSaveFilePicker({
          suggestedName: defaultName,
          types: [
            format === "xml"
              ? { description: "Gleisplan XML", accept: { "application/xml": [".xml"] } }
              : {
                  description: "Gleisplan Datei",
                  accept: { "application/json": [".gleisplan", ".json"] },
                },
          ],
        });
        const writable = await handle.createWritable();
        await writable.write(content);
        await writable.close();
        this.setPlanInfo(handle.name, null, format);
        this.saveToLocalStorage(layout);
        return true;
      } catch (err: any) {
        if (err.name === "AbortError") return false;
        console.warn("showSaveFilePicker Fehler, wechsle zu Download:", err);
      }
    }

    // 3. Fallback über Download
    downloadFile(defaultName, content, mime);
    this.setPlanInfo(defaultName, null, format);
    this.saveToLocalStorage(layout);
    return true;
  }

  private parseContent(content: string, filename: string): Layout {
    if (filename.toLowerCase().endsWith(".xml") || content.trim().startsWith("<?xml") || content.trim().startsWith("<layout")) {
      return fromXML(content);
    }
    return fromJSON(content);
  }

  saveToLocalStorage(layout: Layout) {
    try {
      localStorage.setItem("railbench_current_plan", toJSON(layout));
      localStorage.setItem("railbench_last_saved", new Date().toISOString());
    } catch {
      // localStorage voll oder blockiert
    }
  }

  loadFromLocalStorage(): Layout | null {
    try {
      const saved = localStorage.getItem("railbench_current_plan");
      if (saved) return fromJSON(saved);
    } catch {
      // Ignorieren
    }
    return null;
  }
}
