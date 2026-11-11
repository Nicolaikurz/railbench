import type { Layout, TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { isTurnoutEntry } from "@/core/simulation";
import { isFlexTrack } from "@/core/flextrack";
import { getLanguage } from "@/i18n";
import { getOpenConnectors } from "@/core/ramp-generator";

export interface TrackContextMenuOptions {
  piece: TrackPiece;
  screenX: number;
  screenY: number;
  catalogEntry?: CatalogEntry;
  layout: Layout;
  // Ramp / height
  onOpenRampGenerator: () => void;
  onAutoRamp: () => void;
  onOpenHeightDialog: () => void;
  // Edit
  onRotateCw?: () => void;
  onRotateCcw?: () => void;
  onMirrorFlip?: () => void;
  onCopy: () => void;
  onPaste?: () => void;
  onDuplicate?: () => void;
  onDelete: () => void;
  // Selection
  onSelectConnected?: () => void;
  onSelectLayer?: () => void;
  onToggleGroup?: () => void;
  // Track-specific
  onSplitTrack?: () => void;
  onReplaceTrack?: (newCatalogId: string) => void;
  onToggleTurnoutDecoder?: () => void;
  onToggleTurnoutLantern?: () => void;
  // Backward compatibility
  onRotate?: () => void;
  onToggleFeeder?: () => void;
  onToggleIsolator?: () => void;
  onToggleCatenary?: () => void;
}

const MENU_ID = "track-context-menu";

export function closeTrackContextMenu(): void {
  const existing = document.getElementById(MENU_ID);
  if (existing) existing.remove();
}

export function showTrackContextMenu(options: TrackContextMenuOptions): void {
  closeTrackContextMenu();

  const {
    piece,
    screenX,
    screenY,
    catalogEntry,
    layout,
    onOpenRampGenerator,
    onAutoRamp,
    onOpenHeightDialog,
    onRotateCw,
    onRotateCcw,
    onMirrorFlip,
    onCopy,
    onPaste,
    onDuplicate,
    onDelete,
    onSelectConnected,
    onSelectLayer,
    onToggleGroup,
    onSplitTrack,
    onReplaceTrack,
    onToggleTurnoutDecoder,
    onToggleTurnoutLantern,
    onRotate,
  } = options;

  const isDe = getLanguage() === "de";
  const layers = layout.layers || [];
  const pieceLayer = layers.find((l) => l.id === piece.layerId) || layers[0];
  const layerName = pieceLayer ? pieceLayer.name.replace(/\s*\([0-9]+\s*mm\)$/, "") : (isDe ? "Ebene" : "Layer");
  const layerHeight = pieceLayer?.heightMm ?? 0;
  const h0 = layerHeight + (piece.elevation?.start ?? 0);
  const h1 = layerHeight + (piece.elevation?.end ?? (piece.elevation?.start ?? 0));
  const heightStr = Math.abs(h0 - h1) > 0.1 ? `${Math.round(h0)}..${Math.round(h1)} mm` : `${Math.round(h0)} mm`;

  const isTurnout = isTurnoutEntry(catalogEntry);
  const openConns = getOpenConnectors(piece, layout.pieces);
  const openEndsCount = openConns.length;
  const openEndsLabel = isDe
    ? `${openEndsCount} ${openEndsCount === 1 ? "offenes Ende" : "offene Enden"}`
    : `${openEndsCount} ${openEndsCount === 1 ? "open end" : "open ends"}`;

  const trackTypeName = catalogEntry?.name || (isDe ? "Gleisstück" : "Track Piece");

  const menu = document.createElement("div");
  menu.id = MENU_ID;
  menu.className = "track-context-menu";

  menu.innerHTML = `
    <style>
      .track-context-menu {
        position: fixed;
        z-index: 99999;
        background: #1a2234;
        color: #e2e8f0;
        border: 1px solid #334155;
        border-radius: 10px;
        box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
        min-width: 250px;
        max-width: 320px;
        max-height: calc(100vh - 24px);
        overflow-y: auto;
        scrollbar-width: thin;
        scrollbar-color: #334155 transparent;
        padding: 5px;
        font-family: inherit;
        font-size: 12px;
        user-select: none;
        backdrop-filter: blur(8px);
        animation: ctxMenuFadeIn 0.1s ease-out;
      }
      @keyframes ctxMenuFadeIn {
        from { opacity: 0; transform: scale(0.96); }
        to { opacity: 1; transform: scale(1); }
      }
      .ctx-header {
        padding: 8px 10px;
        font-size: 11px;
        color: #94a3b8;
        border-bottom: 1px solid #2d3748;
        margin-bottom: 4px;
        display: flex;
        flex-direction: column;
        gap: 3px;
        background: rgba(15, 23, 42, 0.4);
        border-radius: 6px;
      }
      .ctx-header-title {
        font-weight: 700;
        color: #f8fafc;
        font-size: 12.5px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .ctx-header-meta {
        display: flex;
        align-items: center;
        gap: 6px;
        font-size: 10.5px;
        color: #94a3b8;
      }
      .ctx-header-meta .meta-badge {
        background: #25334d;
        color: #38bdf8;
        padding: 1px 5px;
        border-radius: 4px;
        font-weight: 600;
      }
      .ctx-section-title {
        font-size: 9.5px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.6px;
        color: #64748b;
        padding: 6px 10px 2px 10px;
      }
      .ctx-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 5px 10px;
        border-radius: 6px;
        cursor: pointer;
        color: #cbd5e1;
        transition: all 0.12s ease;
      }
      .ctx-item:hover {
        background: #27354f;
        color: #ffffff;
      }
      .ctx-item.primary-accent {
        background: rgba(2, 132, 199, 0.15);
        color: #38bdf8;
        font-weight: 600;
        border: 1px solid rgba(2, 132, 199, 0.3);
        margin: 2px 0 3px 0;
      }
      .ctx-item.primary-accent:hover {
        background: #0284c7;
        color: #ffffff;
      }
      .ctx-item.danger:hover {
        background: #ef4444;
        color: #ffffff;
      }
      .ctx-item-left {
        display: flex;
        align-items: center;
        gap: 8px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .ctx-icon {
        font-size: 13px;
        width: 16px;
        text-align: center;
        display: inline-block;
        flex-shrink: 0;
      }
      .ctx-shortcut {
        font-size: 10.5px;
        color: #64748b;
        font-family: monospace;
        margin-left: 12px;
        flex-shrink: 0;
      }
      .ctx-item:hover .ctx-shortcut {
        color: #cbd5e1;
      }
      .ctx-divider {
        height: 1px;
        background: #2d3748;
        margin: 4px 2px;
      }
    </style>

    <!-- Header: Track ID, Type, Number of open ends -->
    <div class="ctx-header">
      <div class="ctx-header-title">#${piece.catalogId} · ${trackTypeName}</div>
      <div class="ctx-header-meta">
        <span class="meta-badge">${openEndsLabel}</span>
        <span>•</span>
        <span>${layerName} (${heightStr})</span>
      </div>
    </div>

    <!-- 1. Ramp / height -->
    <div class="ctx-section-title">${isDe ? "Rampe / Höhe" : "Ramp / Height"}</div>

    <div class="ctx-item primary-accent" data-action="ramp-generator">
      <div class="ctx-item-left">
        <span class="ctx-icon">📐</span>
        <span>${isDe ? "Rampe zu Ebene erstellen..." : "Auto-Create Ramp to Layer..."}</span>
      </div>
    </div>

    <div class="ctx-item" data-action="auto-ramp">
      <div class="ctx-item-left">
        <span class="ctx-icon">📐</span>
        <span>${isDe ? "Auto-Rampe berechnen" : "Auto-Calculate Ramp"}</span>
      </div>
      <span class="ctx-shortcut">⇧M</span>
    </div>

    <div class="ctx-item" data-action="height-dialog">
      <div class="ctx-item-left">
        <span class="ctx-icon">⤉</span>
        <span>${isDe ? "Gleishöhe & Ebene anpassen..." : "Track Height & Layer..."}</span>
      </div>
      <span class="ctx-shortcut">M</span>
    </div>

    <div class="ctx-divider"></div>

    <!-- 2. Selection -->
    <div class="ctx-section-title">${isDe ? "Auswahl" : "Selection"}</div>

    <div class="ctx-item" data-action="select-connected">
      <div class="ctx-item-left">
        <span class="ctx-icon">🔗</span>
        <span>${isDe ? "Verbundene Gleise auswählen" : "Select connected tracks"}</span>
      </div>
    </div>

    <div class="ctx-item" data-action="select-layer">
      <div class="ctx-item-left">
        <span class="ctx-icon">📑</span>
        <span>${isDe ? "Ganze Ebene auswählen" : "Select whole layer"}</span>
      </div>
    </div>

    <div class="ctx-item" data-action="toggle-group">
      <div class="ctx-item-left">
        <span class="ctx-icon">🗂</span>
        <span>${piece.groupId ? (isDe ? "Gruppierung aufheben" : "Ungroup") : (isDe ? "Gruppieren / Gruppe" : "Group / Ungroup")}</span>
      </div>
    </div>

    <div class="ctx-divider"></div>

    <!-- 4. Track-specific -->
    <div class="ctx-section-title">${isDe ? "Gleisspezifisch" : "Track-specific"}</div>

    <div class="ctx-item" data-action="split-or-replace">
      <div class="ctx-item-left">
        <span class="ctx-icon">✂</span>
        <span>${isDe ? "Gleis teilen / Typ ersetzen..." : "Split track / Replace with another type..."}</span>
      </div>
    </div>

    <div class="ctx-item" data-action="properties-info">
      <div class="ctx-item-left">
        <span class="ctx-icon">ℹ</span>
        <span>${isDe ? "Eigenschaften / Info..." : "Properties / Info..."}</span>
      </div>
    </div>

    ${
      isTurnout
        ? `
      <div class="ctx-item" data-action="toggle-decoder">
        <div class="ctx-item-left">
          <span class="ctx-icon">📟</span>
          <span>${piece.hasDecoder ? (isDe ? "Decoder entfernen" : "Remove Decoder") : (isDe ? "Digitaldecoder einbauen" : "Install Decoder")}</span>
        </div>
        <span class="ctx-shortcut">⇧D</span>
      </div>
      <div class="ctx-item" data-action="toggle-lantern">
        <div class="ctx-item-left">
          <span class="ctx-icon">💡</span>
          <span>${piece.hasLantern ? (isDe ? "Laterne entfernen" : "Remove Lantern") : (isDe ? "Weichenlaterne montieren" : "Mount Lantern")}</span>
        </div>
        <span class="ctx-shortcut">⇧L</span>
      </div>
    `
        : ""
    }
  `;

  document.body.appendChild(menu);

  // Position clamping to keep on screen
  const menuWidth = 270;
  const menuHeight = isTurnout ? 330 : 270;
  let posX = screenX + 6;
  let posY = screenY + 6;

  if (posX + menuWidth > window.innerWidth - 10) {
    posX = screenX - menuWidth - 6;
  }
  if (posY + menuHeight > window.innerHeight - 10) {
    posY = window.innerHeight - menuHeight - 10;
  }
  if (posX < 10) posX = 10;
  if (posY < 10) posY = 10;

  menu.style.left = `${posX}px`;
  menu.style.top = `${posY}px`;

  // Handle actions
  menu.addEventListener("click", (e) => {
    const item = (e.target as HTMLElement).closest<HTMLElement>(".ctx-item");
    if (!item) return;
    const action = item.dataset.action;
    closeTrackContextMenu();

    switch (action) {
      // Ramp / height
      case "ramp-generator":
        onOpenRampGenerator();
        break;
      case "auto-ramp":
        onAutoRamp();
        break;
      case "height-dialog":
        onOpenHeightDialog();
        break;

      // Edit
      case "rotate-cw":
        if (onRotateCw) onRotateCw();
        else if (onRotate) onRotate();
        break;
      case "rotate-ccw":
        if (onRotateCcw) onRotateCcw();
        break;
      case "mirror-flip":
        onMirrorFlip?.();
        break;
      case "copy":
        onCopy();
        break;
      case "paste":
        onPaste?.();
        break;
      case "duplicate":
        onDuplicate?.();
        break;
      case "delete":
        onDelete();
        break;

      // Selection
      case "select-connected":
        onSelectConnected?.();
        break;
      case "select-layer":
        onSelectLayer?.();
        break;
      case "toggle-group":
        onToggleGroup?.();
        break;

      // Track-specific
      case "split-or-replace":
        openReplaceOrSplitDialog(piece, catalogEntry, layout, onSplitTrack, onReplaceTrack);
        break;
      case "properties-info":
        openPropertiesDialog(piece, catalogEntry, layout);
        break;
      case "toggle-decoder":
        onToggleTurnoutDecoder?.();
        break;
      case "toggle-lantern":
        onToggleTurnoutLantern?.();
        break;
    }
  });

  // Close on outside click or escape
  const handleOutsideClick = (e: MouseEvent) => {
    if (!menu.contains(e.target as Node)) {
      closeTrackContextMenu();
      document.removeEventListener("pointerdown", handleOutsideClick);
      window.removeEventListener("keydown", handleKeyEsc);
    }
  };

  const handleKeyEsc = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeTrackContextMenu();
      document.removeEventListener("pointerdown", handleOutsideClick);
      window.removeEventListener("keydown", handleKeyEsc);
    }
  };

  // Delay listener to avoid triggering on the click that opened the menu
  setTimeout(() => {
    document.addEventListener("pointerdown", handleOutsideClick);
    window.addEventListener("keydown", handleKeyEsc);
  }, 30);
}

/**
 * Dialog showing detailed Track Properties & Info (length, radius, article number, electrical, layer, elevation)
 */
export function openPropertiesDialog(piece: TrackPiece, catalogEntry?: CatalogEntry, layout?: Layout): void {
  const existing = document.getElementById("track-properties-modal");
  if (existing) existing.remove();

  const isDe = getLanguage() === "de";
  const layers = layout?.layers || [];
  const pieceLayer = layers.find((l) => l.id === piece.layerId) || layers[0];
  const layerName = pieceLayer ? pieceLayer.name : (isDe ? "Standard-Ebene" : "Default Layer");
  const layerBaseH = pieceLayer?.heightMm ?? 0;
  const h0 = layerBaseH + (piece.elevation?.start ?? 0);
  const h1 = layerBaseH + (piece.elevation?.end ?? (piece.elevation?.start ?? 0));

  const curLength = piece.customLength ?? catalogEntry?.length;
  const curRadius = catalogEntry?.radius;
  const curAngle = catalogEntry?.angle;
  const openConns = layout ? getOpenConnectors(piece, layout.pieces) : [];

  let slopePercent = 0;
  if (curLength && curLength > 0 && Math.abs(h1 - h0) > 0.1) {
    slopePercent = (Math.abs(h1 - h0) / curLength) * 100;
  }

  const modal = document.createElement("div");
  modal.id = "track-properties-modal";
  modal.className = "track-modal-backdrop";
  modal.innerHTML = `
    <style>
      .track-modal-backdrop {
        position: fixed;
        inset: 0;
        z-index: 100000;
        background: rgba(15, 23, 42, 0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: inherit;
        animation: fadeIn 0.15s ease-out;
      }
      .track-modal-box {
        background: #1e293b;
        color: #e2e8f0;
        border: 1px solid #334155;
        border-radius: 12px;
        box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
        width: 440px;
        max-width: calc(100vw - 32px);
        overflow: hidden;
      }
      .track-modal-header {
        padding: 14px 18px;
        border-bottom: 1px solid #334155;
        display: flex;
        align-items: center;
        justify-content: space-between;
        background: #0f172a;
      }
      .track-modal-header h3 {
        margin: 0;
        font-size: 15px;
        font-weight: 700;
        color: #f8fafc;
        display: flex;
        align-items: center;
        gap: 8px;
      }
      .track-modal-close-btn {
        background: transparent;
        border: none;
        color: #94a3b8;
        font-size: 18px;
        cursor: pointer;
        padding: 2px 6px;
        border-radius: 4px;
      }
      .track-modal-close-btn:hover {
        background: #334155;
        color: #ffffff;
      }
      .track-modal-content {
        padding: 16px 18px;
        display: flex;
        flex-direction: column;
        gap: 12px;
        max-height: 70vh;
        overflow-y: auto;
      }
      .track-props-grid {
        display: grid;
        grid-template-columns: 140px 1fr;
        row-gap: 8px;
        column-gap: 12px;
        font-size: 12.5px;
        background: #0f172a;
        padding: 12px;
        border-radius: 8px;
        border: 1px solid #334155;
      }
      .track-prop-label {
        color: #94a3b8;
        font-weight: 500;
      }
      .track-prop-value {
        color: #f1f5f9;
        font-weight: 600;
        word-break: break-all;
      }
      .track-prop-value.accent {
        color: #38bdf8;
      }
      .track-modal-footer {
        padding: 12px 18px;
        border-top: 1px solid #334155;
        display: flex;
        justify-content: flex-end;
        background: #0f172a;
      }
      .track-modal-btn {
        background: #0284c7;
        color: #ffffff;
        border: none;
        padding: 7px 16px;
        border-radius: 6px;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        transition: background 0.15s ease;
      }
      .track-modal-btn:hover {
        background: #0369a1;
      }
    </style>

    <div class="track-modal-box">
      <div class="track-modal-header">
        <h3><span>ℹ</span> ${isDe ? "Gleiseigenschaften & Information" : "Track Properties & Info"}</h3>
        <button class="track-modal-close-btn" id="track-modal-close">✕</button>
      </div>
      <div class="track-modal-content">
        <div class="track-props-grid">
          <span class="track-prop-label">${isDe ? "Artikelnummer:" : "Article Number:"}</span>
          <span class="track-prop-value accent">#${piece.catalogId}</span>

          <span class="track-prop-label">${isDe ? "Bezeichnung:" : "Name / Type:"}</span>
          <span class="track-prop-value">${catalogEntry?.name || "-"}</span>

          <span class="track-prop-label">${isDe ? "Kategorie:" : "Category:"}</span>
          <span class="track-prop-value">${catalogEntry?.kind || "-"}</span>

          <span class="track-prop-label">${isDe ? "Länge:" : "Length:"}</span>
          <span class="track-prop-value">${curLength ? `${curLength} mm` : "-"}</span>

          <span class="track-prop-label">${isDe ? "Radius:" : "Radius:"}</span>
          <span class="track-prop-value">${curRadius ? `R = ${curRadius} mm` : "-"}</span>

          <span class="track-prop-label">${isDe ? "Winkel:" : "Angle:"}</span>
          <span class="track-prop-value">${curAngle !== undefined ? `${curAngle}°` : "-"}</span>

          <span class="track-prop-label">${isDe ? "Spurweite / Gleissystem:" : "Gauge / System:"}</span>
          <span class="track-prop-value">${catalogEntry?.gauge ?? 16.5} mm (H0)</span>

          <span class="track-prop-label">${isDe ? "Ebene:" : "Layer:"}</span>
          <span class="track-prop-value">${layerName}</span>

          <span class="track-prop-label">${isDe ? "Höhe (Start / Ende):" : "Height (Start / End):"}</span>
          <span class="track-prop-value">${Math.round(h0)} mm ➔ ${Math.round(h1)} mm ${slopePercent > 0 ? `(${slopePercent.toFixed(1)}% Steigung)` : ""}</span>

          <span class="track-prop-label">${isDe ? "Offene Enden:" : "Open Ends:"}</span>
          <span class="track-prop-value">${openConns.length} / ${piece.connectors.length} ${isDe ? "frei" : "free"}</span>

          <span class="track-prop-label">${isDe ? "Stromeinspeisung:" : "Power Feeder:"}</span>
          <span class="track-prop-value">${piece.powerFeed ? (isDe ? "⚡ Aktiv" : "⚡ Active") : (isDe ? "Nein" : "No")}</span>

          <span class="track-prop-label">${isDe ? "Trennstelle:" : "Isolator Gap:"}</span>
          <span class="track-prop-value">${piece.isIsolator ? `⊘ Pin ${(piece.isolatedConnectorIndex ?? 0) + 1}` : (isDe ? "Nein" : "No")}</span>

          <span class="track-prop-label">${isDe ? "Oberleitung:" : "Catenary:"}</span>
          <span class="track-prop-value">${piece.hasCatenary ? (isDe ? "⌁ Aktiv" : "⌁ Active") : (isDe ? "Nein" : "No")}</span>

          ${piece.groupId ? `
            <span class="track-prop-label">${isDe ? "Gruppe:" : "Group:"}</span>
            <span class="track-prop-value">${piece.groupId}</span>
          ` : ""}
        </div>
      </div>
      <div class="track-modal-footer">
        <button class="track-modal-btn" id="track-modal-ok">${isDe ? "Schließen" : "Close"}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeModal = () => modal.remove();
  modal.querySelector("#track-modal-close")?.addEventListener("click", closeModal);
  modal.querySelector("#track-modal-ok")?.addEventListener("click", closeModal);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
  });
  const handleKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
      window.removeEventListener("keydown", handleKey);
    }
  };
  window.addEventListener("keydown", handleKey);
}

/**
 * Dialog allowing splitting a track or replacing it with a compatible piece from the catalog
 */
export function openReplaceOrSplitDialog(
  piece: TrackPiece,
  catalogEntry?: CatalogEntry,
  layout?: Layout,
  onSplit?: () => void,
  onReplace?: (newCatalogId: string) => void,
): void {
  const existing = document.getElementById("track-replace-modal");
  if (existing) existing.remove();

  const isDe = getLanguage() === "de";
  const catalog = (window as any).railbenchApp?.catalog || (layout as any)?.catalog;
  const entries: CatalogEntry[] = catalog?.entries || [];

  // Filter compatible pieces of similar kind
  const pieceKind = catalogEntry?.kind;
  const compatiblePieces = entries.filter((e) => {
    if (e.id === piece.catalogId) return false;
    if (pieceKind === "straight" && e.kind === "straight") return true;
    if (pieceKind?.includes("curve") && e.kind?.includes("curve")) return true;
    if (pieceKind?.includes("turnout") && e.kind?.includes("turnout")) return true;
    return false;
  }).slice(0, 16);

  const isFlex = catalogEntry ? isFlexTrack(catalogEntry, piece) : false;

  const modal = document.createElement("div");
  modal.id = "track-replace-modal";
  modal.className = "track-modal-backdrop";
  modal.innerHTML = `
    <style>
      .track-modal-box.wide {
        width: 520px;
      }
      .track-section-card {
        background: #0f172a;
        border: 1px solid #334155;
        border-radius: 8px;
        padding: 12px 14px;
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .track-section-card-title {
        font-size: 13px;
        font-weight: 700;
        color: #f1f5f9;
        display: flex;
        align-items: center;
        gap: 6px;
      }
      .track-section-card-desc {
        font-size: 11.5px;
        color: #94a3b8;
        line-height: 1.4;
      }
      .track-replace-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 6px;
        max-height: 200px;
        overflow-y: auto;
        padding-right: 4px;
      }
      .track-replace-item {
        background: #1e293b;
        border: 1px solid #334155;
        border-radius: 6px;
        padding: 6px 10px;
        cursor: pointer;
        display: flex;
        flex-direction: column;
        transition: all 0.12s ease;
      }
      .track-replace-item:hover {
        background: #0284c7;
        border-color: #38bdf8;
      }
      .track-replace-item:hover .replace-item-sub {
        color: #e0f2fe;
      }
      .replace-item-title {
        font-size: 12px;
        font-weight: 700;
        color: #f8fafc;
      }
      .replace-item-sub {
        font-size: 10.5px;
        color: #94a3b8;
      }
      .track-action-btn {
        background: #2563eb;
        color: #ffffff;
        border: none;
        padding: 7px 14px;
        border-radius: 6px;
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
        align-self: flex-start;
        display: inline-flex;
        align-items: center;
        gap: 6px;
      }
      .track-action-btn:hover {
        background: #1d4ed8;
      }
    </style>

    <div class="track-modal-box wide">
      <div class="track-modal-header">
        <h3><span>✂</span> ${isDe ? "Gleis teilen oder Typ ersetzen" : "Split Track or Replace Type"}</h3>
        <button class="track-modal-close-btn" id="track-modal-close">✕</button>
      </div>
      <div class="track-modal-content">
        <!-- Split Track Option -->
        <div class="track-section-card">
          <div class="track-section-card-title">
            <span>✂</span>
            <span>${isDe ? "Gleis in zwei Hälften teilen" : "Split Track into Halves"}</span>
          </div>
          <div class="track-section-card-desc">
            ${
              isFlex
                ? (isDe ? "Das ausgewählte Flexgleis wird mittig in zwei eigenständige Teilstücke getrennt." : "The selected flex track will be cut in half into two separate pieces.")
                : (isDe ? "Ersetzt das gerade Standardgleis durch zwei passende Halb-Gleise aus dem Katalog." : "Replaces this straight track with two matching half-tracks from the catalog.")
            }
          </div>
          <button class="track-action-btn" id="btn-do-split">
            <span>✂</span>
            <span>${isDe ? "Gleis jetzt teilen" : "Split Track Now"}</span>
          </button>
        </div>

        <!-- Replace Track Option -->
        <div class="track-section-card">
          <div class="track-section-card-title">
            <span>🔄</span>
            <span>${isDe ? "Durch einen anderen Typ ersetzen" : "Replace with Another Type"}</span>
          </div>
          <div class="track-section-card-desc">
            ${isDe ? "Klicken Sie auf ein Gleisstück, um das aktuelle Gleis an derselben Position zu ersetzen:" : "Click on a track piece to replace the current piece at the same position:"}
          </div>

          <div class="track-replace-grid">
            ${
              compatiblePieces.length > 0
                ? compatiblePieces
                    .map(
                      (e) => `
                <div class="track-replace-item" data-catalog-id="${e.id}">
                  <span class="replace-item-title">#${e.id}</span>
                  <span class="replace-item-sub">${e.name} ${e.length ? `(${e.length} mm)` : e.radius ? `(R${e.radius})` : ""}</span>
                </div>
              `,
                    )
                    .join("")
                : `<div style="grid-column: span 2; font-size:12px; color:#94a3b8;">${isDe ? "Keine weiteren kompatiblen Typen gefunden" : "No other compatible types found"}</div>`
            }
          </div>
        </div>
      </div>
      <div class="track-modal-footer">
        <button class="track-modal-btn" id="track-modal-cancel" style="background:#475569">${isDe ? "Abbrechen" : "Cancel"}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeModal = () => modal.remove();
  modal.querySelector("#track-modal-close")?.addEventListener("click", closeModal);
  modal.querySelector("#track-modal-cancel")?.addEventListener("click", closeModal);

  modal.querySelector("#btn-do-split")?.addEventListener("click", () => {
    closeModal();
    onSplit?.();
  });

  modal.querySelectorAll(".track-replace-item").forEach((item) => {
    item.addEventListener("click", () => {
      const catId = (item as HTMLElement).dataset.catalogId;
      if (catId) {
        closeModal();
        onReplace?.(catId);
      }
    });
  });

  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
  });
  const handleKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
      window.removeEventListener("keydown", handleKey);
    }
  };
  window.addEventListener("keydown", handleKey);
}
