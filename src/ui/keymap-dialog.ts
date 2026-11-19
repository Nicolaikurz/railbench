import {
  KEY_ACTION_DEFS,
  loadKeymap,
  saveKeymap,
  resetKeymap,
  eventToShortcutString,
  type KeyActionId,
  type KeyActionDef,
} from "@/settings/keymap-manager";
import { getLanguage } from "@/i18n";
import { showToast } from "./toast";
import { ICONS } from "./icons";

export function openKeymapDialog(): void {
  const isDe = getLanguage() === "de";

  const existing = document.getElementById("keymap-modal");
  if (existing) existing.remove();

  let keymap = loadKeymap();
  let activeRecordingActionId: KeyActionId | null = null;
  let activeCategory: string = "all";
  let searchQuery = "";

  const modal = document.createElement("div");
  modal.id = "keymap-modal";
  modal.className = "rb-modal-overlay";

  modal.innerHTML = `
    <div class="rb-modal-card" style="max-width: 820px; width: 95vw; max-height: 85vh; display: flex; flex-direction: column;">
      <div class="rb-modal-header">
        <div style="display: flex; align-items: center; gap: 10px;">
          <span style="font-size: 20px;">⌨️</span>
          <div>
            <div class="rb-modal-title" style="font-size: 16px; font-weight: 600;">
              ${isDe ? "Tastaturbelegung anpassen" : "Customize Keyboard Shortcuts"}
            </div>
            <div style="font-size: 12px; color: var(--text-muted, #94a3b8); margin-top: 2px;">
              ${
                isDe
                  ? "Klicke auf ein Kürzel, um eine eigene Taste zuzuweisen. +/- für Zoom und Pfeiltasten für Bildlauf."
                  : "Click a shortcut to record a new key combination. +/- for zoom and arrow keys for pan."
              }
            </div>
          </div>
        </div>
        <button type="button" class="rb-modal-close-btn" id="keymap-close-btn" title="${isDe ? "Schließen" : "Close"}">✕</button>
      </div>

      <div style="padding: 12px 20px; border-bottom: 1px solid var(--border-subtle, #334155); display: flex; flex-wrap: wrap; gap: 10px; align-items: center; justify-content: space-between; background: var(--bg-card-header, #1e293b);">
        <div style="display: flex; gap: 6px; flex-wrap: wrap;" id="keymap-category-filters">
          <button type="button" class="rb-btn rb-btn-sm ${activeCategory === "all" ? "rb-btn-primary" : "rb-btn-secondary"}" data-cat="all">
            ${isDe ? "Alle" : "All"}
          </button>
          <button type="button" class="rb-btn rb-btn-sm ${activeCategory === "view" ? "rb-btn-primary" : "rb-btn-secondary"}" data-cat="view">
            ${isDe ? "Navigation & Ansicht" : "Navigation & View"}
          </button>
          <button type="button" class="rb-btn rb-btn-sm ${activeCategory === "edit" ? "rb-btn-primary" : "rb-btn-secondary"}" data-cat="edit">
            ${isDe ? "Bearbeiten" : "Editing"}
          </button>
          <button type="button" class="rb-btn rb-btn-sm ${activeCategory === "tools" ? "rb-btn-primary" : "rb-btn-secondary"}" data-cat="tools">
            ${isDe ? "Werkzeuge & Elektrik" : "Tools & Power"}
          </button>
          <button type="button" class="rb-btn rb-btn-sm ${activeCategory === "file" ? "rb-btn-primary" : "rb-btn-secondary"}" data-cat="file">
            ${isDe ? "Datei" : "File"}
          </button>
        </div>

        <div style="position: relative; min-width: 220px; flex: 1; max-width: 320px;">
          <input
            type="text"
            id="keymap-search"
            placeholder="${isDe ? "Aktion oder Taste suchen..." : "Search action or key..."}"
            style="width: 100%; padding: 6px 12px; font-size: 12.5px; border-radius: 6px; border: 1px solid var(--border-color, #475569); background: var(--input-bg, #0f172a); color: var(--text-color, #f8fafc);"
          />
        </div>
      </div>

      <div id="keymap-list-container" style="flex: 1; overflow-y: auto; padding: 12px 20px; display: flex; flex-direction: column; gap: 8px;">
        <!-- Rows rendered dynamically -->
      </div>

      <div class="rb-modal-footer" style="padding: 12px 20px; display: flex; justify-content: space-between; align-items: center; border-top: 1px solid var(--border-subtle, #334155); background: var(--bg-card-header, #1e293b);">
        <button type="button" class="rb-btn rb-btn-secondary rb-btn-sm" id="keymap-reset-all" style="color: #f87171;">
          ↺ ${isDe ? "Alle auf Standard zurücksetzen" : "Reset All to Defaults"}
        </button>
        <button type="button" class="rb-btn rb-btn-primary" id="keymap-done-btn" style="min-width: 90px;">
          ${isDe ? "Schließen" : "Done"}
        </button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const listContainer = modal.querySelector("#keymap-list-container") as HTMLElement;
  const searchInput = modal.querySelector("#keymap-search") as HTMLInputElement;
  const filterBtns = modal.querySelectorAll("#keymap-category-filters button");

  function renderList(): void {
    const q = searchQuery.toLowerCase().trim();
    const filtered = KEY_ACTION_DEFS.filter((def) => {
      if (activeCategory !== "all" && def.category !== activeCategory) return false;
      if (!q) return true;
      const label = (isDe ? def.labelDe : def.labelEn).toLowerCase();
      const desc = (isDe ? def.descDe : def.descEn).toLowerCase();
      const keys = (keymap[def.id] || []).join(" ").toLowerCase();
      return label.includes(q) || desc.includes(q) || keys.includes(q);
    });

    if (filtered.length === 0) {
      listContainer.innerHTML = `
        <div style="text-align: center; padding: 40px 20px; color: var(--text-muted, #94a3b8); font-size: 13px;">
          ${isDe ? "Keine Aktionen gefunden." : "No matching actions found."}
        </div>
      `;
      return;
    }

    listContainer.innerHTML = filtered
      .map((def) => {
        const isRecording = activeRecordingActionId === def.id;
        const currentKeys = keymap[def.id] || [];
        const isModified = JSON.stringify(currentKeys) !== JSON.stringify(def.defaultKeys);

        return `
          <div class="keymap-row" style="display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; background: var(--bg-card, #1e293b); border: 1px solid ${
            isRecording ? "var(--primary-color, #38bdf8)" : "var(--border-subtle, #334155)"
          }; border-radius: 8px; transition: border-color 0.15s ease;">
            <div style="flex: 1; padding-right: 16px;">
              <div style="font-size: 13.5px; font-weight: 500; color: var(--text-color, #f8fafc);">
                ${isDe ? def.labelDe : def.labelEn}
              </div>
              <div style="font-size: 11.5px; color: var(--text-muted, #94a3b8); margin-top: 2px;">
                ${isDe ? def.descDe : def.descEn}
              </div>
            </div>

            <div style="display: flex; align-items: center; gap: 8px;">
              <div style="display: flex; gap: 5px; flex-wrap: wrap; align-items: center;">
                ${
                  isRecording
                    ? `
                    <span class="key-recording-badge" style="display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; background: rgba(56, 189, 248, 0.15); border: 1px dashed #38bdf8; border-radius: 6px; color: #38bdf8; font-size: 12px; font-weight: 600; animation: pulse 1.2s infinite;">
                      ⌨️ ${isDe ? "Drücke Taste... (Esc = Abbruch)" : "Press key... (Esc to cancel)"}
                    </span>
                  `
                    : currentKeys.length > 0
                      ? currentKeys
                          .map(
                            (k) => `
                        <kbd class="key-badge" style="display: inline-flex; align-items: center; gap: 4px; padding: 3px 8px; font-family: ui-monospace, monospace; font-size: 11.5px; background: var(--bg-app, #0f172a); border: 1px solid var(--border-color, #475569); border-radius: 5px; color: #f1f5f9; box-shadow: 0 1px 2px rgba(0,0,0,0.3);">
                          ${k}
                        </kbd>
                      `,
                          )
                          .join("")
                      : `<span style="font-size: 11.5px; color: var(--text-muted, #94a3b8); font-style: italic;">${isDe ? "Keine Taste" : "No key"}</span>`
                }
              </div>

              <div style="display: flex; align-items: center; gap: 4px; margin-left: 8px;">
                <button
                  type="button"
                  class="rb-btn rb-btn-sm ${isRecording ? "rb-btn-danger" : "rb-btn-secondary"}"
                  data-action="record"
                  data-id="${def.id}"
                  style="font-size: 12px; padding: 4px 10px;"
                >
                  ${isRecording ? (isDe ? "Abbrechen" : "Cancel") : isDe ? "Ändern" : "Edit"}
                </button>

                ${
                  isModified
                    ? `
                  <button
                    type="button"
                    class="rb-btn rb-btn-sm rb-btn-secondary"
                    data-action="reset-single"
                    data-id="${def.id}"
                    title="${isDe ? "Auf Standard zurücksetzen" : "Reset to default"}"
                    style="padding: 4px 8px; font-size: 12px;"
                  >
                    ↺
                  </button>
                `
                    : ""
                }
              </div>
            </div>
          </div>
        `;
      })
      .join("");
  }

  renderList();

  // Search input listener
  searchInput.addEventListener("input", () => {
    searchQuery = searchInput.value;
    renderList();
  });

  // Category filter listeners
  filterBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      filterBtns.forEach((b) => {
        b.classList.remove("rb-btn-primary");
        b.classList.add("rb-btn-secondary");
      });
      btn.classList.remove("rb-btn-secondary");
      btn.classList.add("rb-btn-primary");
      activeCategory = (btn.getAttribute("data-cat") || "all") as any;
      renderList();
    });
  });

  // Action clicks (Record / Reset)
  listContainer.addEventListener("click", (e) => {
    const target = (e.target as HTMLElement).closest("button") as HTMLElement | null;
    if (!target) return;

    const action = target.getAttribute("data-action");
    const id = target.getAttribute("data-id") as KeyActionId | null;
    if (!id) return;

    if (action === "record") {
      if (activeRecordingActionId === id) {
        activeRecordingActionId = null;
      } else {
        activeRecordingActionId = id;
      }
      renderList();
    } else if (action === "reset-single") {
      const def = KEY_ACTION_DEFS.find((d) => d.id === id);
      if (def) {
        keymap[id] = [...def.defaultKeys];
        saveKeymap(keymap);
        renderList();
        showToast(
          isDe ? `Kürzel für "${def.labelDe}" zurückgesetzt` : `Reset shortcut for "${def.labelEn}"`,
          "↺",
        );
      }
    }
  });

  // Global Keydown Handler for recording new shortcuts
  const keydownRecorder = (e: KeyboardEvent) => {
    if (!activeRecordingActionId) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      activeRecordingActionId = null;
      renderList();
      return;
    }

    const shortcut = eventToShortcutString(e);
    if (!shortcut) return; // Nur Modifikatortaste gedrückt

    const currentActionId = activeRecordingActionId;
    const def = KEY_ACTION_DEFS.find((d) => d.id === currentActionId);

    // Auf Konflikte prüfen
    let conflictAction: KeyActionDef | null = null;
    for (const otherDef of KEY_ACTION_DEFS) {
      if (otherDef.id === currentActionId) continue;
      const otherKeys = keymap[otherDef.id] || [];
      if (otherKeys.some((k) => k.toLowerCase() === shortcut.toLowerCase())) {
        conflictAction = otherDef;
        break;
      }
    }

    if (conflictAction) {
      // Vorherigen Konflikt entfernen
      keymap[conflictAction.id] = (keymap[conflictAction.id] || []).filter(
        (k) => k.toLowerCase() !== shortcut.toLowerCase(),
      );
      showToast(
        isDe
          ? `Kürzel "${shortcut}" von "${conflictAction.labelDe}" übernommen.`
          : `Shortcut "${shortcut}" moved from "${conflictAction.labelEn}".`,
        ICONS.alertTriangle,
      );
    }

    keymap[currentActionId] = [shortcut];
    saveKeymap(keymap);
    activeRecordingActionId = null;
    renderList();

    showToast(
      isDe
        ? `"${def?.labelDe || currentActionId}" auf [${shortcut}] gesetzt`
        : `"${def?.labelEn || currentActionId}" assigned to [${shortcut}]`,
      "✓",
    );
  };

  window.addEventListener("keydown", keydownRecorder, true);

  // Close & Reset listeners
  const close = () => {
    window.removeEventListener("keydown", keydownRecorder, true);
    modal.remove();
  };

  modal.querySelector("#keymap-close-btn")?.addEventListener("click", close);
  modal.querySelector("#keymap-done-btn")?.addEventListener("click", close);

  modal.querySelector("#keymap-reset-all")?.addEventListener("click", () => {
    const ok = window.confirm(
      isDe
        ? "Möchtest du wirklich alle Tastaturkürzel auf die Werkseinstellungen zurücksetzen?"
        : "Do you really want to reset all keyboard shortcuts to factory defaults?",
    );
    if (!ok) return;

    keymap = resetKeymap();
    activeRecordingActionId = null;
    renderList();
    showToast(isDe ? "Alle Tastaturkürzel zurückgesetzt" : "All shortcuts reset to defaults", "↺");
  });

  // Click outside to close (if not recording)
  modal.addEventListener("click", (e) => {
    if (e.target === modal && !activeRecordingActionId) {
      close();
    }
  });
}
