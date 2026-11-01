import type { BaseboardConfig } from "./types";
import { getBaseboardBounds, getBaseboardPolygon } from "./layout";

export interface BaseboardSnapGuide {
  type: "x" | "y";
  pos: number;
  from: number;
  to: number;
  color?: number;
}

export interface BaseboardSnapResult {
  x: number;
  y: number;
  snappedX: boolean;
  snappedY: boolean;
  guides: BaseboardSnapGuide[];
}

/**
 * Berechnet das magnetische Einrasten (Snapping) einer bewegten Tischplatte an andere Tische.
 * Unterstützt:
 * 1. Kante-an-Kante (Links an Rechts, Rechts an Links, Oben an Unten, Unten an Oben)
 * 2. Bündig-Ausrichtung (Oben bündig, Unten bündig, Links bündig, Rechts bündig, Mitte zentriert)
 * 3. Eck-an-Eck Snapping auch für komplexe L-, U- und T-Formen
 */
export function snapBaseboard(
  candidateBoard: BaseboardConfig,
  candidateX: number,
  candidateY: number,
  otherBoards: BaseboardConfig[],
  threshold = 35,
): BaseboardSnapResult {
  const W_A = candidateBoard.width;
  const D_A = candidateBoard.depth;

  let bestX = candidateX;
  let bestY = candidateY;
  let minDiffX = threshold + 1;
  let minDiffY = threshold + 1;
  let snappedX = false;
  let snappedY = false;

  const guides: BaseboardSnapGuide[] = [];

  const enabledOthers = otherBoards.filter((b) => b.id !== candidateBoard.id && b.enabled !== false);
  if (enabledOthers.length === 0) {
    return { x: candidateX, y: candidateY, snappedX: false, snappedY: false, guides: [] };
  }

  // 1. Kanten- und Ausrichtungs-Snapping (Bounding Box)
  for (const b of enabledOthers) {
    const bB = getBaseboardBounds(b);

    const leftB = bB.x;
    const rightB = bB.x + bB.width;
    const topB = bB.y;
    const bottomB = bB.y + bB.depth;
    const midXB = bB.x + bB.width / 2;
    const midYB = bB.y + bB.depth / 2;

    // --- X-ACHSE ---
    // a) A rechts neben B (A.left an B.right)
    const diffRight = Math.abs(candidateX - rightB);
    if (diffRight < minDiffX) {
      minDiffX = diffRight;
      bestX = rightB;
      snappedX = true;
    }

    // b) A links neben B (A.right an B.left)
    const diffLeft = Math.abs(candidateX + W_A - leftB);
    if (diffLeft < minDiffX) {
      minDiffX = diffLeft;
      bestX = leftB - W_A;
      snappedX = true;
    }

    // c) Links bündig
    const diffFlushL = Math.abs(candidateX - leftB);
    if (diffFlushL < minDiffX) {
      minDiffX = diffFlushL;
      bestX = leftB;
      snappedX = true;
    }

    // d) Rechts bündig
    const diffFlushR = Math.abs(candidateX + W_A - rightB);
    if (diffFlushR < minDiffX) {
      minDiffX = diffFlushR;
      bestX = rightB - W_A;
      snappedX = true;
    }

    // e) Mitte bündig
    const diffMidX = Math.abs(candidateX + W_A / 2 - midXB);
    if (diffMidX < minDiffX) {
      minDiffX = diffMidX;
      bestX = midXB - W_A / 2;
      snappedX = true;
    }

    // --- Y-ACHSE ---
    // a) A unter B (A.top an B.bottom)
    const diffBottom = Math.abs(candidateY - bottomB);
    if (diffBottom < minDiffY) {
      minDiffY = diffBottom;
      bestY = bottomB;
      snappedY = true;
    }

    // b) A über B (A.bottom an B.top)
    const diffTop = Math.abs(candidateY + D_A - topB);
    if (diffTop < minDiffY) {
      minDiffY = diffTop;
      bestY = topB - D_A;
      snappedY = true;
    }

    // c) Oben bündig
    const diffFlushT = Math.abs(candidateY - topB);
    if (diffFlushT < minDiffY) {
      minDiffY = diffFlushT;
      bestY = topB;
      snappedY = true;
    }

    // d) Unten bündig
    const diffFlushB = Math.abs(candidateY + D_A - bottomB);
    if (diffFlushB < minDiffY) {
      minDiffY = diffFlushB;
      bestY = bottomB - D_A;
      snappedY = true;
    }

    // e) Mitte bündig
    const diffMidY = Math.abs(candidateY + D_A / 2 - midYB);
    if (diffMidY < minDiffY) {
      minDiffY = diffMidY;
      bestY = midYB - D_A / 2;
      snappedY = true;
    }
  }

  // 2. Polygon-Ecken-Snapping (für L-, U-, T-Formen und exakte Eck-Treffer)
  const tempCandidate: BaseboardConfig = {
    ...candidateBoard,
    positionMode: "custom",
    customX: bestX,
    customY: bestY,
  };
  const polyA = getBaseboardPolygon(tempCandidate);

  for (const b of enabledOthers) {
    const polyB = getBaseboardPolygon(b);
    for (const va of polyA) {
      for (const vb of polyB) {
        const dx = Math.abs(va.x - vb.x);
        const dy = Math.abs(va.y - vb.y);
        if (dx < threshold && dy < threshold && dx + dy < minDiffX + minDiffY) {
          bestX += (vb.x - va.x);
          bestY += (vb.y - va.y);
          snappedX = true;
          snappedY = true;
          minDiffX = dx;
          minDiffY = dy;
          break;
        }
      }
    }
  }

  // 3. Führungslinien (Guides) erzeugen
  if (snappedX) {
    for (const b of enabledOthers) {
      const bB = getBaseboardBounds(b);
      const rightB = bB.x + bB.width;
      const leftB = bB.x;

      if (Math.abs(bestX - rightB) < 1.5 || Math.abs(bestX + W_A - leftB) < 1.5 || Math.abs(bestX - leftB) < 1.5 || Math.abs(bestX + W_A - rightB) < 1.5) {
        const guidePos = Math.abs(bestX - rightB) < 1.5 ? rightB : (Math.abs(bestX - leftB) < 1.5 ? leftB : (Math.abs(bestX + W_A - leftB) < 1.5 ? leftB : rightB));
        guides.push({
          type: "x",
          pos: guidePos,
          from: Math.min(bestY, bB.y) - 60,
          to: Math.max(bestY + D_A, bB.y + bB.depth) + 60,
          color: 0x06b6d4, // Cyan
        });
        break;
      }
    }
  }

  if (snappedY) {
    for (const b of enabledOthers) {
      const bB = getBaseboardBounds(b);
      const bottomB = bB.y + bB.depth;
      const topB = bB.y;

      if (Math.abs(bestY - bottomB) < 1.5 || Math.abs(bestY + D_A - topB) < 1.5 || Math.abs(bestY - topB) < 1.5 || Math.abs(bestY + D_A - bottomB) < 1.5) {
        const guidePos = Math.abs(bestY - bottomB) < 1.5 ? bottomB : (Math.abs(bestY - topB) < 1.5 ? topB : (Math.abs(bestY + D_A - topB) < 1.5 ? topB : bottomB));
        guides.push({
          type: "y",
          pos: guidePos,
          from: Math.min(bestX, bB.x) - 60,
          to: Math.max(bestX + W_A, bB.x + bB.width) + 60,
          color: 0x06b6d4, // Cyan
        });
        break;
      }
    }
  }

  return {
    x: Math.round(bestX),
    y: Math.round(bestY),
    snappedX,
    snappedY,
    guides,
  };
}

