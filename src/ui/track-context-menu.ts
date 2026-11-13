import trackPropertiesModalHtml from "./templates/track-properties-modal.html?raw";
import trackReplaceModalHtml from "./templates/track-replace-modal.html?raw";
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
  const groupSection = piece.groupId ? `
    <span class="track-prop-label">${isDe ? "Gruppe:" : "Group:"}</span>
    <span class="track-prop-value">${piece.groupId}</span>
  ` : "";

  modal.innerHTML = trackPropertiesModalHtml
    .replace("{{TITLE}}", isDe ? "Gleiseigenschaften & Information" : "Track Properties & Info")
    .replace("{{ARTICLE_NUMBER_LABEL}}", isDe ? "Artikelnummer:" : "Article Number:")
    .replace("{{CATALOG_ID}}", piece.catalogId)
    .replace("{{NAME_LABEL}}", isDe ? "Bezeichnung:" : "Name / Type:")
    .replace("{{NAME_VALUE}}", catalogEntry?.name || "-")
    .replace("{{CATEGORY_LABEL}}", isDe ? "Kategorie:" : "Category:")
    .replace("{{CATEGORY_VALUE}}", catalogEntry?.kind || "-")
    .replace("{{LENGTH_LABEL}}", isDe ? "Länge:" : "Length:")
    .replace("{{LENGTH_VALUE}}", curLength ? `${curLength} mm` : "-")
    .replace("{{RADIUS_LABEL}}", isDe ? "Radius:" : "Radius:")
    .replace("{{RADIUS_VALUE}}", curRadius ? `R = ${curRadius} mm` : "-")
    .replace("{{ANGLE_LABEL}}", isDe ? "Winkel:" : "Angle:")
    .replace("{{ANGLE_VALUE}}", curAngle !== undefined ? `${curAngle}°` : "-")
    .replace("{{GAUGE_LABEL}}", isDe ? "Spurweite / Gleissystem:" : "Gauge / System:")
    .replace("{{GAUGE_VALUE}}", `${catalogEntry?.gauge ?? 16.5} mm (H0)`)
    .replace("{{LAYER_LABEL}}", isDe ? "Ebene:" : "Layer:")
    .replace("{{LAYER_VALUE}}", layerName)
    .replace("{{HEIGHT_LABEL}}", isDe ? "Höhe (Start / Ende):" : "Height (Start / End):")
    .replace("{{HEIGHT_VALUE}}", `${Math.round(h0)} mm ➔ ${Math.round(h1)} mm ${slopePercent > 0 ? `(${slopePercent.toFixed(1)}% Steigung)` : ""}`)
    .replace("{{OPEN_ENDS_LABEL}}", isDe ? "Offene Enden:" : "Open Ends:")
    .replace("{{OPEN_ENDS_VALUE}}", `${openConns.length} / ${piece.connectors.length} ${isDe ? "frei" : "free"}`)
    .replace("{{FEEDER_LABEL}}", isDe ? "Stromeinspeisung:" : "Power Feeder:")
    .replace("{{FEEDER_VALUE}}", piece.powerFeed ? (isDe ? "⚡ Aktiv" : "⚡ Active") : (isDe ? "Nein" : "No"))
    .replace("{{ISOLATOR_LABEL}}", isDe ? "Trennstelle:" : "Isolator Gap:")
    .replace("{{ISOLATOR_VALUE}}", piece.isIsolator ? `⊘ Pin ${(piece.isolatedConnectorIndex ?? 0) + 1}` : (isDe ? "Nein" : "No"))
    .replace("{{CATENARY_LABEL}}", isDe ? "Oberleitung:" : "Catenary:")
    .replace("{{CATENARY_VALUE}}", piece.hasCatenary ? (isDe ? "⌁ Aktiv" : "⌁ Active") : (isDe ? "Nein" : "No"))
    .replace("{{GROUP_SECTION}}", groupSection)
    .replace("{{CLOSE_BTN}}", isDe ? "Schließen" : "Close");

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
  const compatibleItemsHtml = compatiblePieces.length > 0
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
    : `<div style="grid-column: span 2; font-size:12px; color:#94a3b8;">${isDe ? "Keine weiteren kompatiblen Typen gefunden" : "No other compatible types found"}</div>`;

  const splitCardDesc = isFlex
    ? (isDe ? "Das ausgewählte Flexgleis wird mittig in zwei eigenständige Teilstücke getrennt." : "The selected flex track will be cut in half into two separate pieces.")
    : (isDe ? "Ersetzt das gerade Standardgleis durch zwei passende Halb-Gleise aus dem Katalog." : "Replaces this straight track with two matching half-tracks from the catalog.");

  modal.innerHTML = trackReplaceModalHtml
    .replace("{{TITLE}}", isDe ? "Gleis teilen oder Typ ersetzen" : "Split Track or Replace Type")
    .replace("{{SPLIT_CARD_TITLE}}", isDe ? "Gleis in zwei Hälften teilen" : "Split Track into Halves")
    .replace("{{SPLIT_CARD_DESC}}", splitCardDesc)
    .replace("{{SPLIT_BTN_LABEL}}", isDe ? "Gleis jetzt teilen" : "Split Track Now")
    .replace("{{REPLACE_CARD_TITLE}}", isDe ? "Durch einen anderen Typ ersetzen" : "Replace with Another Type")
    .replace("{{REPLACE_CARD_DESC}}", isDe ? "Klicken Sie auf ein Gleisstück, um das aktuelle Gleis an derselben Position zu ersetzen:" : "Click on a track piece to replace the current piece at the same position:")
    .replace("{{COMPATIBLE_ITEMS}}", compatibleItemsHtml)
    .replace("{{CANCEL_BTN}}", isDe ? "Abbrechen" : "Cancel");

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
