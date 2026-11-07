import { Container, Graphics } from "pixi.js";
import type { Layout, Layer, TrackPiece } from "@/core/types";
import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import type { Renderer } from "@/render/renderer";
import { createTrackGraphics } from "@/render/track-sprite";
import { createBaseboardGraphics } from "@/render/baseboard-sprite";
import { getPieceListSummary, getLayerPieceListSummary, type PieceSummaryItem } from "@/io/export";
import { getTranslations } from "@/i18n";
import { getBaseboards, getBaseboardBounds } from "@/core/layout";
import {
  renderFullPageToCanvas,
  renderBomPageToCanvas,
  canvasToJpegBytes,
  buildPdfFromImages,
  downloadBlob,
  escapeHtml,
} from "./print-pdf";

export { buildPdfFromImages } from "./print-pdf";

export interface PrintBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  width: number;
  height: number;
}

export interface PrintRenderOptions {
  layerId: string | "all";
  bounds: PrintBounds;
  showBaseboard?: boolean;
  showTrackLabels?: boolean;
  showColorCode?: boolean;
  ghostOtherLayers?: boolean;
  monochrome?: boolean;
}

export interface PrintPlanOptions {
  layerIds: string[];
  includeOverview: boolean;
  includeBOM: boolean;
  ghostOtherLayers: boolean;
  showTrackLabels: boolean;
  showColorCode: boolean;
  showBaseboard: boolean;
  monochrome: boolean;
  orientation: "landscape" | "portrait";
}

/** Berechnet die gemeinsamen Grenzen aller Gleise und des Tisches mit Rand. */
export function computeLayoutPrintBounds(layout: Layout): PrintBounds {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const piece of layout.pieces) {
    for (const conn of piece.connectors) {
      if (conn.x < minX) minX = conn.x;
      if (conn.x > maxX) maxX = conn.x;
      if (conn.y < minY) minY = conn.y;
      if (conn.y > maxY) maxY = conn.y;
    }
  }

  const baseboards = getBaseboards(layout);
  for (const bb of baseboards) {
    if (bb.enabled) {
      const b = getBaseboardBounds(bb);
      minX = Math.min(minX, b.x);
      minY = Math.min(minY, b.y);
      maxX = Math.max(maxX, b.x + b.width);
      maxY = Math.max(maxY, b.y + b.depth);
    }
  }

  if (!isFinite(minX) || !isFinite(maxX) || !isFinite(minY) || !isFinite(maxY)) {
    minX = -500;
    maxX = 500;
    minY = -350;
    maxY = 350;
  }

  // Großzügiger Rand (100 mm) um das Layout
  const pad = 100;
  minX -= pad;
  minY -= pad;
  maxX += pad;
  maxY += pad;

  return {
    minX,
    minY,
    maxX,
    maxY,
    width: Math.max(800, maxX - minX),
    height: Math.max(500, maxY - minY),
  };
}

/** Rendert eine Ebene (oder alle Ebenen) in eine hochauflösende PNG Data-URL. */
export async function renderLayerToDataUrl(
  layout: Layout,
  catalog: Catalog,
  renderer: Renderer,
  options: PrintRenderOptions,
): Promise<string> {
  const { bounds, layerId } = options;
  const showBaseboard = options.showBaseboard ?? true;
  const showTrackLabels = options.showTrackLabels ?? true;
  const showColorCode = options.showColorCode ?? true;
  const ghostOtherLayers = options.ghostOtherLayers ?? true;
  const monochrome = options.monochrome ?? false;

  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) {
    entryMap.set(e.id, e);
  }

  // Wrapper-Container, der genau bei (0,0) beginnt und exakt bounds.width/height groß ist
  const wrapper = new Container();

  // Weißer Hintergrund für sauberen Druck
  const bg = new Graphics();
  bg.rect(0, 0, bounds.width, bounds.height).fill({ color: 0xffffff });
  wrapper.addChild(bg);

  // Inhalt mit Verschiebung (-minX, -minY)
  const content = new Container();
  content.position.set(-bounds.minX, -bounds.minY);
  wrapper.addChild(content);

  // 1. Grundplatte / Tischumriss als Referenz
  if (showBaseboard) {
    const baseboards = getBaseboards(layout);
    for (const bb of baseboards) {
      if (bb.enabled) {
        const bounds = getBaseboardBounds(bb);
        const bbG = new Graphics();
        bbG.rect(bounds.x, bounds.y, bounds.width, bounds.depth);
        bbG.fill({ color: 0xf8fafc });
        bbG.stroke({ width: 2.5, color: 0x94a3b8 });
        content.addChild(bbG);
      }
    }
  }

  // 2. Geister-Ebenen (andere Ebenen schwach im Hintergrund als Referenz)
  if (ghostOtherLayers && layerId !== "all") {
    const ghostContainer = new Container();
    ghostContainer.alpha = 0.2;
    for (const piece of layout.pieces) {
      const pLayer = piece.layerId || "layer-1";
      if (pLayer !== layerId) {
        const entry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
          id: piece.catalogId,
          name: piece.catalogId,
          kind: "straight",
          category: "Geraden",
          description: "",
          length: 188.3,
        };
        const sprite = createTrackGraphics(piece, entry,catalog, {
          isSelected: false,
          showTrackLabels: false,
          showColorCode: false,
        });
        ghostContainer.addChild(sprite);
      }
    }
    content.addChild(ghostContainer);
  }

  // 3. Haupt-Gleise dieser Ebene (oder alle bei "all")
  const tracksContainer = new Container();
  for (const piece of layout.pieces) {
    const pLayer = piece.layerId || "layer-1";
    if (layerId === "all" || pLayer === layerId) {
      const entry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
        id: piece.catalogId,
        name: piece.catalogId,
        kind: "straight",
        category: "Geraden",
        description: "",
        length: 188.3,
      };

      const sprite = createTrackGraphics(piece, entry,catalog, {
        isSelected: false,
        showTrackLabels,
        showColorCode: monochrome ? false : showColorCode,
      });
      tracksContainer.addChild(sprite);
    }
  }
  content.addChild(tracksContainer);

  // 4. Maßstabsbalken & Koordinatenkreuz unten links
  const decorG = new Graphics();
  const barX = bounds.minX + 40;
  const barY = bounds.maxY - 40;
  const barLength = bounds.width > 1500 ? 500 : 200;

  decorG.moveTo(barX, barY).lineTo(barX + barLength, barY);
  decorG.moveTo(barX, barY - 6).lineTo(barX, barY + 6);
  decorG.moveTo(barX + barLength, barY - 6).lineTo(barX + barLength, barY + 6);
  decorG.stroke({ width: 2.5, color: 0x334155 });

  // Außenrahmen
  decorG.rect(bounds.minX, bounds.minY, bounds.width, bounds.height);
  decorG.stroke({ width: 1.5, color: 0xe2e8f0 });

  content.addChild(decorG);

  let dataUrl = "";
  try {
    // 2x Auflösung für gestochen scharfen Druck
    dataUrl = await renderer.app.renderer.extract.base64({
      target: wrapper,
      format: "png",
      resolution: 1.5,
    });
  } catch (err) {
    console.error("Failed to extract layer canvas for print:", err);
  } finally {
    wrapper.destroy({ children: true });
  }

  return dataUrl;
}

/** Öffnet das interaktive Druckmenü & bereitet den seitenweisen Ausdruck vor. */
export async function openPrintDialog(
  layout: Layout,
  catalog: Catalog,
  renderer: Renderer,
): Promise<void> {
  const existingModal = document.getElementById("print-modal");
  if (existingModal) existingModal.remove();

  const layers = layout.layers || [
    { id: "layer-1", name: "Ground Level / Ebene 1", visible: true, locked: false, heightMm: 0 },
  ];

  // Standard-Optionen
  const printOptions: PrintPlanOptions = {
    layerIds: layers.map((l) => l.id),
    includeOverview: true,
    includeBOM: true,
    ghostOtherLayers: true,
    showTrackLabels: true,
    showColorCode: true,
    showBaseboard: true,
    monochrome: false,
    orientation: "landscape",
  };

  const modal = document.createElement("div");
  modal.id = "print-modal";
  modal.className = "rb-modal-overlay";
  const t = getTranslations().printDialog;

  modal.innerHTML = `
    <div class="rb-modal-card print-modal-card">
      <div class="rb-modal-header">
        <div class="rb-modal-title">
          <span></span>
          <span>${t.title}</span>
        </div>
        <button class="rb-modal-close-btn" id="print-modal-close">✕</button>
      </div>

      <div class="print-modal-layout">
        <!-- Linke Spalte: Druckeinstellungen -->
        <div class="print-config-pane">
          <div class="print-section-title">${t.sectionLayers}</div>
          <div class="print-layer-checklist" id="print-layer-checklist">
            ${layers
              .map((lay) => {
                const count = layout.pieces.filter((p) => (p.layerId || "layer-1") === lay.id).length;
                const hMm = lay.heightMm ?? 0;
                const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;
                const trackWord = count === 1 ? getTranslations().selectionBar.pieceSingular : getTranslations().selectionBar.piecePlural;
                return `
                <label class="print-layer-item">
                  <input type="checkbox" value="${lay.id}" checked class="print-chk-layer" />
                  <span class="print-color-pill" style="background: ${lay.color || '#38bdf8'};"></span>
                  <div class="print-item-text">
                    <span class="print-item-name">${escapeHtml(lay.name)}</span>
                    <span class="print-item-meta">${hSign} mm · ${count} ${trackWord}</span>
                  </div>
                </label>
              `;
              })
              .join("")}
          </div>

          <div class="print-section-title">${t.sectionAdditional}</div>
          <div class="print-option-row">
            <label class="print-chk-label">
              <input type="checkbox" id="chk-include-overview" checked />
              <span>${t.chkOverview}</span>
            </label>
          </div>
          <div class="print-option-row">
            <label class="print-chk-label">
              <input type="checkbox" id="chk-include-bom" checked />
              <span>${t.chkBOM}</span>
            </label>
          </div>

          <div class="print-section-title">${t.sectionVisual}</div>
          <div class="print-option-row">
            <label class="print-chk-label">
              <input type="checkbox" id="chk-ghost-layers" checked />
              <span>${t.chkGhost}</span>
            </label>
          </div>
          <div class="print-option-row">
            <label class="print-chk-label">
              <input type="checkbox" id="chk-track-numbers" checked />
              <span>${t.chkLabels}</span>
            </label>
          </div>
          <div class="print-option-row">
            <label class="print-chk-label">
              <input type="checkbox" id="chk-baseboard" checked />
              <span>${t.chkBaseboard}</span>
            </label>
          </div>
          <div class="print-option-row">
            
          </div>

          <div class="print-section-title">${t.sectionOrientation}</div>
          <div class="print-radio-group">
            <label class="print-radio-label">
              <input type="radio" name="page-orientation" value="landscape" checked />
              <span>${t.optLandscape}</span>
            </label>
            <label class="print-radio-label">
              <input type="radio" name="page-orientation" value="portrait" />
              <span>${t.optPortrait}</span>
            </label>
          </div>
        </div>

        <!-- Rechte Spalte: Vorschau-Liste -->
        <div class="print-preview-pane">
          <div class="print-preview-header">
            <span>${t.previewHeader}</span>
            <span class="print-page-count-badge" id="print-page-count-badge">${t.calculating}</span>
          </div>
          <div class="print-preview-scroll" id="print-preview-scroll">
            <div class="print-loading-msg">${t.renderingMsg}</div>
          </div>
        </div>
      </div>

      <div class="rb-modal-footer">
        <button class="rb-btn rb-btn-secondary" id="print-btn-cancel">${t.closeBtn}</button>
        <button class="rb-btn rb-btn-primary" id="print-btn-export-pdf" title="${t.exportPdfTooltip}">
          <span>📄</span>
          <span>${t.exportPdfBtn}</span>
        </button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#print-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#print-btn-cancel") as HTMLButtonElement;
  const systemBtn = modal.querySelector("#print-btn-system") as HTMLButtonElement;
  const exportPdfBtn = modal.querySelector("#print-btn-export-pdf") as HTMLButtonElement;
  const previewScroll = modal.querySelector("#print-preview-scroll") as HTMLElement;
  const pageBadge = modal.querySelector("#print-page-count-badge") as HTMLElement;

  const closeModal = () => modal.remove();
  closeBtn.addEventListener("click", closeModal);
  cancelBtn.addEventListener("click", closeModal);

  // Event Listener für Optionen
  const updateOptionsFromUI = () => {
    const selectedCheckboxes = modal.querySelectorAll<HTMLInputElement>(".print-chk-layer:checked");
    printOptions.layerIds = Array.from(selectedCheckboxes).map((cb) => cb.value);
    printOptions.includeOverview = (modal.querySelector("#chk-include-overview") as HTMLInputElement).checked;
    printOptions.includeBOM = (modal.querySelector("#chk-include-bom") as HTMLInputElement).checked;
    printOptions.ghostOtherLayers = (modal.querySelector("#chk-ghost-layers") as HTMLInputElement).checked;
    printOptions.showTrackLabels = (modal.querySelector("#chk-track-numbers") as HTMLInputElement).checked;
    printOptions.showBaseboard = (modal.querySelector("#chk-baseboard") as HTMLInputElement).checked;
    const orientRadio = modal.querySelector<HTMLInputElement>("input[name='page-orientation']:checked");
    printOptions.orientation = (orientRadio?.value as "landscape" | "portrait") || "landscape";
  };

  // Preview rendern
  const refreshPreview = async () => {
    updateOptionsFromUI();
    previewScroll.innerHTML = `<div class="print-loading-msg">Generating crisp layer previews...</div>`;

    const bounds = computeLayoutPrintBounds(layout);
    const pages: {
      type: "overview" | "layer" | "bom";
      title: string;
      subtitle: string;
      layerId?: string;
      layer?: Layer;
      imgUrl?: string;
    }[] = [];

    // 1. Overview Page
    if (printOptions.includeOverview) {
      const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
        layerId: "all",
        bounds,
        showBaseboard: printOptions.showBaseboard,
        showTrackLabels: printOptions.showTrackLabels,
        showColorCode: !printOptions.monochrome,
        ghostOtherLayers: false,
        monochrome: printOptions.monochrome,
      });
      pages.push({
        type: "overview",
        title: t.overviewTitle,
        subtitle: t.overviewSubtitle.replace("{count}", String(layout.pieces.length)).replace("{scale}", layout.scale),
        imgUrl,
      });
    }

    // 2. Individual Layer Pages
    for (const layerId of printOptions.layerIds) {
      const layer = layers.find((l) => l.id === layerId);
      if (!layer) continue;
      const layerPieces = layout.pieces.filter((p) => (p.layerId || "layer-1") === layer.id);
      const hMm = layer.heightMm ?? 0;
      const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;

      const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
        layerId: layer.id,
        bounds,
        showBaseboard: printOptions.showBaseboard,
        showTrackLabels: printOptions.showTrackLabels,
        showColorCode: !printOptions.monochrome,
        ghostOtherLayers: printOptions.ghostOtherLayers,
        monochrome: printOptions.monochrome,
      });

      pages.push({
        type: "layer",
        title: layer.name,
        subtitle: t.layerSubtitle.replace("{height}", hSign).replace("{count}", String(layerPieces.length)),
        layerId: layer.id,
        layer,
        imgUrl,
      });
    }

    // 3. Bill of Materials Page
    if (printOptions.includeBOM) {
      pages.push({
        type: "bom",
        title: t.bomTitle,
        subtitle: t.bomSubtitle.replace("{count}", String(layout.pieces.length)),
      });
    }

    const sSuffix = pages.length === 1 ? "" : "s";
    pageBadge.textContent = t.pagesCount.replace("{count}", String(pages.length)).replace("{s}", sSuffix);
    previewScroll.innerHTML = "";

    pages.forEach((pg, idx) => {
      const card = document.createElement("div");
      card.className = "print-preview-card";
      card.innerHTML = `
        <div class="preview-card-header">
          <span class="preview-page-idx">${t.pageIdx.replace("{idx}", String(idx + 1))}</span>
          <span class="preview-page-title">${escapeHtml(pg.title)}</span>
          <span class="preview-page-meta">${escapeHtml(pg.subtitle)}</span>
        </div>
        <div class="preview-thumbnail-container ${printOptions.orientation}">
          ${
            pg.imgUrl
              ? `<img src="${pg.imgUrl}" class="preview-thumbnail-img" />`
              : `<div class="preview-bom-mock">
                  <div class="preview-bom-mock-table">
                    <div class="mock-row header-row"><span>Part</span><span>Name</span><span>Qty</span></div>
                    <div class="mock-row"><span>24188</span><span>Straight track 188.3 mm</span><span>...</span></div>
                    <div class="mock-row"><span>24172</span><span>Straight track 171.7 mm</span><span>...</span></div>
                    <div class="mock-row"><span>24224</span><span>Curved track R2 24.3°</span><span>...</span></div>
                  </div>
                </div>`
          }
        </div>
      `;
      previewScroll.appendChild(card);
    });
  };

  // Checkbox-Änderungen überwachen
  modal.querySelectorAll("input").forEach((inp) => {
    inp.addEventListener("change", () => refreshPreview());
  });

 

  // 2. Direkt als PDF exportieren
  exportPdfBtn.addEventListener("click", async () => {
    exportPdfBtn.disabled = true;
    exportPdfBtn.innerHTML = `<span>⏳</span><span>Generating PDF...</span>`;

    const isLandscape = printOptions.orientation === "landscape";
    const defaultPdfName = `${layout.name || "Railbench"} - Plan.pdf`;

    // Pfad A: Electron Nativer PDF Export
    if ((window as any).electronAPI?.exportPDF) {
      await mountPrintPagesToDom(layout, catalog, renderer, printOptions);
      try {
        const result = await (window as any).electronAPI.exportPDF(defaultPdfName, isLandscape);
        if (result && result.success) {
          closeModal();
          return;
        }
      } catch (err: any) {
        console.warn("Electron exportPDF failed, falling back to pure JS PDF generation:", err);
      }
    }

    // Pfad B: Reiner JS Multi-Page PDF Export (Web & Fallback)
    try {
      const bounds = computeLayoutPrintBounds(layout);
      const layers = layout.layers || [];
      const totalPages =
        printOptions.layerIds.length +
        (printOptions.includeOverview ? 1 : 0) +
        (printOptions.includeBOM ? 1 : 0);
      let pageIdx = 1;
      const dateStr = new Date().toLocaleDateString();
      const pdfImages: { bytes: Uint8Array; width: number; height: number }[] = [];

      // 1. Overview Page
      if (printOptions.includeOverview) {
        const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
          layerId: "all",
          bounds,
          showBaseboard: printOptions.showBaseboard,
          showTrackLabels: printOptions.showTrackLabels,
          showColorCode: !printOptions.monochrome,
          ghostOtherLayers: false,
          monochrome: printOptions.monochrome,
        });

        const { totalPieces, totalLengthMeters } = getPieceListSummary(layout, catalog);
        const pageCanvas = await renderFullPageToCanvas(
          layout.name,
          `Overall Layout Overview · Scale ${layout.scale}`,
          {
            name: "All Layers Combined",
            color: "#0284c7",
            elev: `${layers.length} Layers`,
            countText: `${totalPieces} tracks · ${totalLengthMeters.toFixed(2)} m total length`,
          },
          imgUrl,
          `Active Layers: ${layers.map((l) => `${l.name} (${l.heightMm ?? 0} mm)`).join(" · ")}`,
          `Page ${pageIdx} of ${totalPages} · Date: ${dateStr}`,
          isLandscape,
        );
        const jpeg = await canvasToJpegBytes(pageCanvas);
        pdfImages.push(jpeg);
        pageIdx++;
      }

      // 2. Individual Layer Pages
      for (const layerId of printOptions.layerIds) {
        const layer = layers.find((l) => l.id === layerId);
        if (!layer) continue;
        const hMm = layer.heightMm ?? 0;
        const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;
        const { items, totalPieces, totalLengthMeters } = getLayerPieceListSummary(layout, catalog, layer.id);

        const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
          layerId: layer.id,
          bounds,
          showBaseboard: printOptions.showBaseboard,
          showTrackLabels: printOptions.showTrackLabels,
          showColorCode: !printOptions.monochrome,
          ghostOtherLayers: printOptions.ghostOtherLayers,
          monochrome: printOptions.monochrome,
        });

        const chipsText = items
          .slice(0, 10)
          .map((it) => `${it.catalogId}: ${it.count}×`)
          .join("   ");

        const pageCanvas = await renderFullPageToCanvas(
          layout.name,
          `Layer: ${layer.name} · Scale: ${layout.scale}`,
          {
            name: layer.name,
            color: layer.color || "#38bdf8",
            elev: `Height: ${hSign} mm`,
            countText: `${totalPieces} tracks · ${totalLengthMeters.toFixed(2)} m on this level`,
          },
          imgUrl,
          `Parts on this level: ${chipsText || "No tracks assigned to this layer"}`,
          `Page ${pageIdx} of ${totalPages} · Date: ${dateStr}`,
          isLandscape,
        );
        const jpeg = await canvasToJpegBytes(pageCanvas);
        pdfImages.push(jpeg);
        pageIdx++;
      }

      // 3. BOM Page
      if (printOptions.includeBOM) {
        const bomCanvas = renderBomPageToCanvas(
          layout,
          catalog,
          `Page ${pageIdx} of ${totalPages} · Date: ${dateStr}`,
          isLandscape,
        );
        const jpeg = await canvasToJpegBytes(bomCanvas);
        pdfImages.push(jpeg);
      }

      // PDF kompilieren und Download auslösen
      const pdfBytes = buildPdfFromImages(pdfImages, isLandscape);
      const blob = new Blob([pdfBytes as unknown as BlobPart], { type: "application/pdf" });
      downloadBlob(blob, defaultPdfName);
      closeModal();
    } catch (err: any) {
      alert("Error generating PDF: " + (err?.message ?? err));
      exportPdfBtn.disabled = false;
      exportPdfBtn.innerHTML = `<span>📄</span><span>Direct Export as PDF</span>`;
    }
  });

  // Initialer Render der Vorschauen
  refreshPreview();
}

/** Erzeugt den HTML-DOM-Mount für den Ausdruck bzw. Electron printToPDF */
async function mountPrintPagesToDom(
  layout: Layout,
  catalog: Catalog,
  renderer: Renderer,
  printOptions: PrintPlanOptions,
): Promise<{ totalPages: number }> {
  const bounds = computeLayoutPrintBounds(layout);
  const dateStr = new Date().toLocaleDateString();
  const layers = layout.layers || [];

  let mount = document.getElementById("print-mount-point");
  if (!mount) {
    mount = document.createElement("div");
    mount.id = "print-mount-point";
    document.body.appendChild(mount);
  }
  mount.innerHTML = "";

  const totalPages =
    printOptions.layerIds.length +
    (printOptions.includeOverview ? 1 : 0) +
    (printOptions.includeBOM ? 1 : 0);
  let currentPageIndex = 1;

  // 1. Overview Page
  if (printOptions.includeOverview) {
    const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
      layerId: "all",
      bounds,
      showBaseboard: printOptions.showBaseboard,
      showTrackLabels: printOptions.showTrackLabels,
      showColorCode: !printOptions.monochrome,
      ghostOtherLayers: false,
      monochrome: printOptions.monochrome,
    });

    const { totalPieces, totalLengthMeters } = getPieceListSummary(layout, catalog);

    const overviewPage = document.createElement("div");
    overviewPage.className = `print-page ${printOptions.orientation}`;
    overviewPage.innerHTML = `
      <div class="print-header">
        <div class="print-header-info">
          <div class="print-brand-tag">🚂 Railbench Track Planner</div>
          <h1 class="print-layout-title">${escapeHtml(layout.name)}</h1>
          <div class="print-subline">Overall Layout Overview · Scale <strong>${layout.scale}</strong></div>
        </div>
        <div class="print-header-layer">
          <div class="print-layer-chip" style="border-left-color: #0284c7;">
            <span class="print-layer-name">All Layers Combined</span>
            <span class="print-layer-elev">${layers.length} Layers</span>
          </div>
          <div class="print-layer-count">${totalPieces} tracks · ${totalLengthMeters.toFixed(2)} m total length</div>
        </div>
      </div>

      <div class="print-body">
        <img src="${imgUrl}" class="print-img" alt="Overview plan" />
      </div>

      <div class="print-footer">
        <div class="print-footer-layers-legend">
          <strong>Active Layers:</strong>
          ${layers
            .map(
              (l) => `
            <span class="print-legend-pill">
              <span class="legend-color-dot" style="background: ${l.color || '#38bdf8'};"></span>
              <span>${escapeHtml(l.name)} (${l.heightMm ?? 0} mm)</span>
            </span>
          `,
            )
            .join(" ")}
        </div>
        <div class="print-page-num">
          Page ${currentPageIndex} of ${totalPages} · Date: ${dateStr}
        </div>
      </div>
    `;
    mount.appendChild(overviewPage);
    currentPageIndex++;
  }

  // 2. Individual Layer Pages
  for (const layerId of printOptions.layerIds) {
    const layer = layers.find((l) => l.id === layerId);
    if (!layer) continue;
    const hMm = layer.heightMm ?? 0;
    const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;
    const { items, totalPieces, totalLengthMeters } = getLayerPieceListSummary(layout, catalog, layer.id);

    const imgUrl = await renderLayerToDataUrl(layout, catalog, renderer, {
      layerId: layer.id,
      bounds,
      showBaseboard: printOptions.showBaseboard,
      showTrackLabels: printOptions.showTrackLabels,
      showColorCode: !printOptions.monochrome,
      ghostOtherLayers: printOptions.ghostOtherLayers,
      monochrome: printOptions.monochrome,
    });

    const chipsHtml = items
      .slice(0, 14)
      .map((it) => `<span class="print-bom-chip"><strong>${it.catalogId}</strong>: ${it.count}×</span>`)
      .join(" ");

    const layerPage = document.createElement("div");
    layerPage.className = `print-page ${printOptions.orientation}`;
    layerPage.innerHTML = `
      <div class="print-header">
        <div class="print-header-info">
          <div class="print-brand-tag">🚂 Railbench Track Planner</div>
          <h1 class="print-layout-title">${escapeHtml(layout.name)}</h1>
          <div class="print-subline">Scale: <strong>${layout.scale}</strong> · Date: ${dateStr}</div>
        </div>
        <div class="print-header-layer">
          <div class="print-layer-chip" style="border-left-color: ${layer.color || '#38bdf8'};">
            <span class="print-layer-name">${escapeHtml(layer.name)}</span>
            <span class="print-layer-elev">Height: <strong>${hSign} mm</strong></span>
          </div>
          <div class="print-layer-count">${totalPieces} tracks · ${totalLengthMeters.toFixed(2)} m on this level</div>
        </div>
      </div>

      <div class="print-body">
        <img src="${imgUrl}" class="print-img" alt="${escapeHtml(layer.name)} plan" />
      </div>

      <div class="print-footer">
        <div class="print-footer-bom">
          <span class="print-bom-label">Parts on this level:</span>
          <div class="print-chips-container">${chipsHtml || "<em>No tracks assigned to this layer</em>"}</div>
        </div>
        <div class="print-page-num">
          Page ${currentPageIndex} of ${totalPages}
        </div>
      </div>
    `;
    mount.appendChild(layerPage);
    currentPageIndex++;
  }

  // 3. Bill of Materials (BOM) Page
  if (printOptions.includeBOM) {
    const { items, totalPieces, totalLengthMeters } = getPieceListSummary(layout, catalog);

    const bomPage = document.createElement("div");
    bomPage.className = `print-page ${printOptions.orientation}`;
    bomPage.innerHTML = `
      <div class="print-header">
        <div class="print-header-info">
          <div class="print-brand-tag">🚂 Railbench Track Planner</div>
          <h1 class="print-layout-title">${escapeHtml(layout.name)} – Bill of Materials</h1>
          <div class="print-subline">Complete Part List & Summary · Scale <strong>${layout.scale}</strong></div>
        </div>
        <div class="print-header-layer">
          <div class="print-layer-chip" style="border-left-color: #10b981;">
            <span class="print-layer-name">Stückliste / Inventory</span>
            <span class="print-layer-elev">${totalPieces} Items</span>
          </div>
          <div class="print-layer-count">${totalLengthMeters.toFixed(2)} m total trackage</div>
        </div>
      </div>

      <div class="print-body print-bom-table-container">
        <table class="print-bom-table">
          <thead>
            <tr>
              <th>Part No.</th>
              <th>Description</th>
              <th>Category</th>
              <th>Geometry / Specification</th>
              <th style="text-align: right;">Quantity</th>
              <th style="text-align: right;">Length (m)</th>
            </tr>
          </thead>
          <tbody>
            ${items
              .map(
                (it) => `
              <tr>
                <td><strong>${escapeHtml(it.catalogId)}</strong></td>
                <td>${escapeHtml(it.name)}</td>
                <td>${escapeHtml(it.category)}</td>
                <td>${escapeHtml(it.specification)}</td>
                <td style="text-align: right;"><strong>${it.count}</strong></td>
                <td style="text-align: right;">${it.totalLengthMeters.toFixed(2)} m</td>
              </tr>
            `,
              )
              .join("")}
          </tbody>
          <tfoot>
            <tr>
              <td colspan="4"><strong>Grand Total:</strong></td>
              <td style="text-align: right;"><strong>${totalPieces}</strong></td>
              <td style="text-align: right;"><strong>${totalLengthMeters.toFixed(2)} m</strong></td>
            </tr>
          </tfoot>
        </table>
      </div>

      <div class="print-footer">
        <div class="print-footer-bom">
          <span>Layout: ${escapeHtml(layout.name)} (${layout.scale}) · Feed Voltage: ${layout.feedVoltage ?? 16}V</span>
        </div>
        <div class="print-page-num">
          Page ${currentPageIndex} of ${totalPages} · Date: ${dateStr}
        </div>
      </div>
    `;
    mount.appendChild(bomPage);
  }

  return { totalPages };
}


