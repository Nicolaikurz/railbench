import type { Layout } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getPieceListSummary } from "@/io/export";
import { APP_NAME, getAppVersionTag } from "@/version";

/** Zeichnet eine vollständige Einzelseite (Header, Plan-Grafik, Footer) auf einen A4-Canvas */
export function renderFullPageToCanvas(
  title: string,
  subtitle: string,
  layerBadge: { name: string; color: string; elev: string; countText: string },
  imgUrl: string,
  chipsText: string,
  footerPageText: string,
  landscape: boolean,
): Promise<HTMLCanvasElement> {
  return new Promise((resolve) => {
    const w = landscape ? 2480 : 1754;
    const h = landscape ? 1754 : 2480;
    const canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext("2d")!;

    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, w, h);

    const pad = 70;
    let y = pad;

    // Header
    ctx.fillStyle = "#0284c7";
    ctx.font = "bold 26px Inter, system-ui, sans-serif";
    ctx.fillText(`🚂 ${APP_NAME} Track Planner ${getAppVersionTag()}`, pad, y + 24);

    ctx.fillStyle = "#0f172a";
    ctx.font = "bold 44px Inter, system-ui, sans-serif";
    ctx.fillText(title, pad, y + 72);

    ctx.fillStyle = "#64748b";
    ctx.font = "22px Inter, system-ui, sans-serif";
    ctx.fillText(subtitle, pad, y + 104);

    // Layer Badge (Top Right)
    const badgeW = 420;
    const badgeH = 100;
    const badgeX = w - pad - badgeW;
    ctx.fillStyle = "#f8fafc";
    ctx.fillRect(badgeX, y + 10, badgeW, badgeH);
    ctx.strokeStyle = "#e2e8f0";
    ctx.lineWidth = 1.5;
    ctx.strokeRect(badgeX, y + 10, badgeW, badgeH);

    ctx.fillStyle = layerBadge.color || "#38bdf8";
    ctx.fillRect(badgeX, y + 10, 8, badgeH);

    ctx.fillStyle = "#0f172a";
    ctx.font = "bold 24px Inter, system-ui, sans-serif";
    ctx.fillText(layerBadge.name, badgeX + 22, y + 44);

    ctx.fillStyle = "#334155";
    ctx.font = "20px Inter, system-ui, sans-serif";
    ctx.fillText(layerBadge.elev, badgeX + 22, y + 72);

    ctx.fillStyle = "#64748b";
    ctx.font = "18px Inter, system-ui, sans-serif";
    ctx.fillText(layerBadge.countText, badgeX + 22, y + 96);

    y += 125;

    // Header Border
    ctx.strokeStyle = "#0f172a";
    ctx.lineWidth = 3.5;
    ctx.beginPath();
    ctx.moveTo(pad, y);
    ctx.lineTo(w - pad, y);
    ctx.stroke();

    // Footer
    const footerH = 90;
    const footerY = h - pad - footerH;

    ctx.strokeStyle = "#cbd5e1";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(pad, footerY);
    ctx.lineTo(w - pad, footerY);
    ctx.stroke();

    ctx.fillStyle = "#475569";
    ctx.font = "18px Inter, system-ui, sans-serif";
    ctx.fillText(chipsText, pad, footerY + 45);

    ctx.textAlign = "right";
    ctx.fillStyle = "#64748b";
    ctx.fillText(footerPageText, w - pad, footerY + 45);
    ctx.textAlign = "left";

    // Layout Image in middle
    const bodyTop = y + 20;
    const bodyHeight = footerY - bodyTop - 20;
    const bodyWidth = w - pad * 2;

    const img = new Image();
    img.onload = () => {
      const imgAspect = img.width / img.height;
      const bodyAspect = bodyWidth / bodyHeight;
      let drawW = bodyWidth;
      let drawH = bodyHeight;
      if (imgAspect > bodyAspect) {
        drawW = bodyWidth;
        drawH = bodyWidth / imgAspect;
      } else {
        drawH = bodyHeight;
        drawW = bodyHeight * imgAspect;
      }
      const drawX = pad + (bodyWidth - drawW) / 2;
      const drawY = bodyTop + (bodyHeight - drawH) / 2;
      ctx.drawImage(img, drawX, drawY, drawW, drawH);
      resolve(canvas);
    };
    img.src = imgUrl;
  });
}

/** Zeichnet die Stückliste (BOM) formatfüllend auf einen A4-Canvas */
export function renderBomPageToCanvas(
  layout: Layout,
  catalog: Catalog,
  footerPageText: string,
  landscape: boolean,
): HTMLCanvasElement {
  const w = landscape ? 2480 : 1754;
  const h = landscape ? 1754 : 2480;
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;

  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, w, h);

  const pad = 70;
  let y = pad;

  // Header
  ctx.fillStyle = "#0284c7";
  ctx.font = "bold 26px Inter, system-ui, sans-serif";
  ctx.fillText(`🚂 ${APP_NAME} Track Planner ${getAppVersionTag()}`, pad, y + 24);

  ctx.fillStyle = "#0f172a";
  ctx.font = "bold 44px Inter, system-ui, sans-serif";
  ctx.fillText(`${layout.name} – Bill of Materials`, pad, y + 72);

  ctx.fillStyle = "#64748b";
  ctx.font = "22px Inter, system-ui, sans-serif";
  ctx.fillText(`Scale: ${layout.scale} · Feed Voltage: ${layout.feedVoltage ?? 16}V`, pad, y + 104);

  const { items, totalPieces, totalLengthMeters } = getPieceListSummary(layout, catalog);
  const badgeW = 380;
  const badgeH = 100;
  const badgeX = w - pad - badgeW;
  ctx.fillStyle = "#f8fafc";
  ctx.fillRect(badgeX, y + 10, badgeW, badgeH);
  ctx.strokeStyle = "#e2e8f0";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(badgeX, y + 10, badgeW, badgeH);
  ctx.fillStyle = "#10b981";
  ctx.fillRect(badgeX, y + 10, 8, badgeH);

  ctx.fillStyle = "#0f172a";
  ctx.font = "bold 24px Inter, system-ui, sans-serif";
  ctx.fillText("Stückliste / Inventory", badgeX + 22, y + 46);
  ctx.fillStyle = "#334155";
  ctx.font = "20px Inter, system-ui, sans-serif";
  ctx.fillText(`${totalPieces} Pieces Total`, badgeX + 22, y + 74);
  ctx.fillStyle = "#64748b";
  ctx.font = "18px Inter, system-ui, sans-serif";
  ctx.fillText(`${totalLengthMeters.toFixed(2)} m total length`, badgeX + 22, y + 96);

  y += 125;

  ctx.strokeStyle = "#0f172a";
  ctx.lineWidth = 3.5;
  ctx.beginPath();
  ctx.moveTo(pad, y);
  ctx.lineTo(w - pad, y);
  ctx.stroke();
  y += 35;

  // Table header
  const tableW = w - pad * 2;
  ctx.fillStyle = "#f1f5f9";
  ctx.fillRect(pad, y, tableW, 48);
  ctx.strokeStyle = "#cbd5e1";
  ctx.lineWidth = 1.5;
  ctx.strokeRect(pad, y, tableW, 48);

  ctx.fillStyle = "#0f172a";
  ctx.font = "bold 20px Inter, system-ui, sans-serif";
  ctx.fillText("Part No.", pad + 20, y + 31);
  ctx.fillText("Description", pad + 200, y + 31);
  ctx.fillText("Category", pad + 900, y + 31);
  ctx.fillText("Specification", pad + 1200, y + 31);
  ctx.textAlign = "right";
  ctx.fillText("Quantity", w - pad - 240, y + 31);
  ctx.fillText("Length (m)", w - pad - 20, y + 31);
  ctx.textAlign = "left";
  y += 48;

  // Rows
  const rowH = 40;
  const maxRows = Math.floor((h - pad - 120 - y) / rowH) - 1;
  const displayItems = items.slice(0, maxRows);

  ctx.font = "18px Inter, system-ui, sans-serif";
  displayItems.forEach((it, idx) => {
    if (idx % 2 === 1) {
      ctx.fillStyle = "#f8fafc";
      ctx.fillRect(pad, y, tableW, rowH);
    }
    ctx.strokeStyle = "#e2e8f0";
    ctx.strokeRect(pad, y, tableW, rowH);

    ctx.fillStyle = "#0f172a";
    ctx.font = "bold 18px Inter, system-ui, sans-serif";
    ctx.fillText(it.catalogId, pad + 20, y + 26);

    ctx.font = "18px Inter, system-ui, sans-serif";
    ctx.fillStyle = "#334155";
    ctx.fillText(it.name.slice(0, 48), pad + 200, y + 26);
    ctx.fillText(it.category, pad + 900, y + 26);
    ctx.fillText(it.specification.slice(0, 36), pad + 1200, y + 26);

    ctx.textAlign = "right";
    ctx.font = "bold 18px Inter, system-ui, sans-serif";
    ctx.fillStyle = "#0f172a";
    ctx.fillText(it.count.toString(), w - pad - 240, y + 26);
    ctx.font = "18px Inter, system-ui, sans-serif";
    ctx.fillText(`${it.totalLengthMeters.toFixed(2)} m`, w - pad - 20, y + 26);
    ctx.textAlign = "left";

    y += rowH;
  });

  // Grand Total row
  ctx.fillStyle = "#e2e8f0";
  ctx.fillRect(pad, y, tableW, 48);
  ctx.strokeStyle = "#0f172a";
  ctx.lineWidth = 2;
  ctx.strokeRect(pad, y, tableW, 48);

  ctx.fillStyle = "#0f172a";
  ctx.font = "bold 20px Inter, system-ui, sans-serif";
  ctx.fillText("Grand Total:", pad + 20, y + 31);
  ctx.textAlign = "right";
  ctx.fillText(`${totalPieces} Pieces`, w - pad - 240, y + 31);
  ctx.fillText(`${totalLengthMeters.toFixed(2)} m`, w - pad - 20, y + 31);
  ctx.textAlign = "left";

  // Footer
  const footerY = h - pad - 50;
  ctx.strokeStyle = "#cbd5e1";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(pad, footerY);
  ctx.lineTo(w - pad, footerY);
  ctx.stroke();

  ctx.fillStyle = "#64748b";
  ctx.font = "18px Inter, system-ui, sans-serif";
  ctx.fillText(`Layout: ${layout.name} (${layout.scale})`, pad, footerY + 35);
  ctx.textAlign = "right";
  ctx.fillText(footerPageText, w - pad, footerY + 35);
  ctx.textAlign = "left";

  return canvas;
}

/** Konvertiert einen Canvas in komprimierte JPEG-Bytes für Standard-PDF-Einbettung */
export function canvasToJpegBytes(
  canvas: HTMLCanvasElement,
  quality = 0.92,
): Promise<{ bytes: Uint8Array; width: number; height: number }> {
  return new Promise((resolve) => {
    const dataUrl = canvas.toDataURL("image/jpeg", quality);
    const b64 = dataUrl.split(",")[1]!;
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) {
      bytes[i] = bin.charCodeAt(i);
    }
    resolve({ bytes, width: canvas.width, height: canvas.height });
  });
}

/**
 * Erzeugt eine standardkonforme, mehrseitige PDF-1.4 Datei direkt im Browser
 * mit eingebetteten hochauflösenden JPEG-Seiten (ISO 32000-konform, 0 externe Abhängigkeiten).
 */
export function buildPdfFromImages(
  images: { bytes: Uint8Array; width: number; height: number }[],
  landscape: boolean,
): Uint8Array {
  const pageWidth = landscape ? 841.89 : 595.28;
  const pageHeight = landscape ? 595.28 : 841.89;
  const numPages = images.length;

  const chunks: Uint8Array[] = [];
  const offsets: number[] = [];
  let currentOffset = 0;

  function write(str: string) {
    const enc = new TextEncoder().encode(str);
    chunks.push(enc);
    currentOffset += enc.length;
  }

  function writeBytes(bytes: Uint8Array) {
    chunks.push(bytes);
    currentOffset += bytes.length;
  }

  function recordObj(objNum: number) {
    offsets[objNum] = currentOffset;
  }

  write("%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

  // 1 0 obj: Catalog
  recordObj(1);
  write("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

  // 2 0 obj: Pages
  recordObj(2);
  const kidsStr = images.map((_, i) => `${3 + i * 3} 0 R`).join(" ");
  write(`2 0 obj\n<< /Type /Pages /Count ${numPages} /Kids [ ${kidsStr} ] >>\nendobj\n`);

  for (let i = 0; i < numPages; i++) {
    const pageObjNum = 3 + i * 3;
    const contentObjNum = pageObjNum + 1;
    const imageObjNum = pageObjNum + 2;
    const img = images[i]!;

    // Page Object
    recordObj(pageObjNum);
    write(
      `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 ${pageWidth.toFixed(2)} ${pageHeight.toFixed(2)} ] /Resources << /XObject << /Im${i} ${imageObjNum} 0 R >> >> /Contents ${contentObjNum} 0 R >>\nendobj\n`,
    );

    // Content Stream
    recordObj(contentObjNum);
    const contentStream = `q ${pageWidth.toFixed(2)} 0 0 ${pageHeight.toFixed(2)} 0 0 cm /Im${i} Do Q\n`;
    write(
      `${contentObjNum} 0 obj\n<< /Length ${contentStream.length} >>\nstream\n${contentStream}endstream\nendobj\n`,
    );

    // Image XObject
    recordObj(imageObjNum);
    write(
      `${imageObjNum} 0 obj\n<< /Type /XObject /Subtype /Image /Width ${img.width} /Height ${img.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${img.bytes.length} >>\nstream\n`,
    );
    writeBytes(img.bytes);
    write("\nendstream\nendobj\n");
  }

  // Cross-reference table
  const startXref = currentOffset;
  const totalObjs = 2 + numPages * 3;
  write(`xref\n0 ${totalObjs + 1}\n`);
  write("0000000000 65535 f \n");
  for (let o = 1; o <= totalObjs; o++) {
    const off = offsets[o] || 0;
    const offStr = off.toString().padStart(10, "0");
    write(`${offStr} 00000 n \n`);
  }

  write(
    `trailer\n<< /Size ${totalObjs + 1} /Root 1 0 R >>\nstartxref\n${startXref}\n%%EOF\n`,
  );

  const totalLength = chunks.reduce((acc, c) => acc + c.length, 0);
  const result = new Uint8Array(totalLength);
  let pos = 0;
  for (const c of chunks) {
    result.set(c, pos);
    pos += c.length;
  }

  return result;
}

/** Hilfsfunktion zum Starten des Datei-Downloads im Browser */
export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 2500);
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
