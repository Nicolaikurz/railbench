import { Container, Graphics, Text, TextStyle } from "pixi.js";
import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Layout, TrackPiece } from "@/core/types";
import { deg2rad, getPieceLocalCenter, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import {
  PORTAL_LIFT,
  buildPortal,
  buildTunnelSweep,
  createTunnelProfile,
  evaluateMountainLocal,
  worldToMountainLocal,
} from "./models-3d/mountain";

/**
 * Zeichnet die 2D-Tunnel-Darstellung (abgedunkelte Röhre, gestrichelte Tunnelwände,
 * Steinportal-Klammern an offenen Tunnelenden und 🚇-Symbol) für Gleise und Straßen.
 */
export function drawTunnel2D(
  container: Container,
  rawEntry: CatalogEntry,
  piece: TrackPiece,
  allPieces?: TrackPiece[],
): void {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const g = new Graphics();

  const isRoad = !!entry.isRoad;
  const gauge = entry.gauge ?? 16.5;
  const tunnelHalfW = isRoad
    ? (entry.roadWidth ?? 60) / 2 + 6
    : Math.max(20, gauge * 1.25);

  const neighborHasTunnel = (connIdx: number): boolean => {
    const conn = piece.connectors[connIdx];
    if (!conn?.connectedTo || !allPieces) return false;
    const other = allPieces.find((p) => p.id === conn.connectedTo!.pieceId);
    return !!other?.hasTunnel;
  };

  const hasStartPortal = !neighborHasTunnel(0);
  const hasEndPortal = !neighborHasTunnel(1);

  const drawPortalBracket2D = (px: number, py: number, inwardAngleRad: number) => {
    const cos = Math.cos(inwardAngleRad);
    const sin = Math.sin(inwardAngleRad);
    const nx = -sin;
    const ny = cos;

    const wingLen = 10;
    const w = tunnelHalfW + 3;

    // Linker Flügel -> Stirnwand -> Rechter Flügel
    const lxOut = px + nx * (w + 5) - cos * wingLen;
    const lyOut = py + ny * (w + 5) - sin * wingLen;
    const lxIn = px + nx * w;
    const lyIn = py + ny * w;
    const rxIn = px - nx * w;
    const ryIn = py - ny * w;
    const rxOut = px - nx * (w + 5) - cos * wingLen;
    const ryOut = py - ny * (w + 5) - sin * wingLen;

    g.moveTo(lxOut, lyOut)
      .lineTo(lxIn, lyIn)
      .lineTo(rxIn, ryIn)
      .lineTo(rxOut, ryOut);
    g.stroke({ width: 5, color: 0x334155, alpha: 0.95 });

    g.moveTo(lxOut, lyOut)
      .lineTo(lxIn, lyIn)
      .lineTo(rxIn, ryIn)
      .lineTo(rxOut, ryOut);
    g.stroke({ width: 2.6, color: 0x94a3b8, alpha: 0.98 });
  };

  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;

    // 1. Abgedunkelte Tunnel-Decke
    g.rect(0, -tunnelHalfW, l, tunnelHalfW * 2);
    g.fill({ color: 0x0f172a, alpha: 0.34 });

    // 2. Äußere Tunnelwand-Kontur
    g.moveTo(0, -tunnelHalfW).lineTo(l, -tunnelHalfW);
    g.moveTo(0, tunnelHalfW).lineTo(l, tunnelHalfW);
    g.stroke({ width: 3.2, color: 0x334155, alpha: 0.85 });

    // 3. Gestrichelte Tunnel-Mauerlinie
    const dash = 10;
    const gap = 7;
    for (let x = 2; x < l - 2; x += dash + gap) {
      const x2 = Math.min(l - 2, x + dash);
      g.moveTo(x, -tunnelHalfW).lineTo(x2, -tunnelHalfW);
      g.moveTo(x, tunnelHalfW).lineTo(x2, tunnelHalfW);
    }
    g.stroke({ width: 1.8, color: 0xcbd5e1, alpha: 0.9 });

    if (hasStartPortal) drawPortalBracket2D(0, 0, 0);
    if (hasEndPortal) drawPortalBracket2D(l, 0, Math.PI);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    const sign = a < 0 ? -1 : 1;

    const rInner = Math.max(10, r - tunnelHalfW);
    const rOuter = r + tunnelHalfW;
    const steps = Math.max(14, Math.ceil(absA / 2));

    // 1. Abgedunkelter Bogen-Korridor
    g.moveTo(0, sign * -tunnelHalfW);
    for (let i = 0; i <= steps; i++) {
      const curA = (rad * i) / steps;
      g.lineTo(rOuter * Math.sin(curA), sign * (r - rOuter * Math.cos(curA)));
    }
    for (let i = steps; i >= 0; i--) {
      const curA = (rad * i) / steps;
      g.lineTo(rInner * Math.sin(curA), sign * (r - rInner * Math.cos(curA)));
    }
    g.closePath();
    g.fill({ color: 0x0f172a, alpha: 0.34 });

    // 2. Gestrichelte Seitenwände entlang des Bogens
    for (const radiusWall of [rInner, rOuter]) {
      g.moveTo(0, sign * (r - radiusWall));
      for (let i = 1; i <= steps; i++) {
        const curA = (rad * i) / steps;
        g.lineTo(radiusWall * Math.sin(curA), sign * (r - radiusWall * Math.cos(curA)));
      }
      g.stroke({ width: 3.0, color: 0x334155, alpha: 0.85 });

      for (let i = 0; i < steps; i += 2) {
        const a0 = (rad * i) / steps;
        const a1 = (rad * Math.min(steps, i + 1)) / steps;
        g.moveTo(radiusWall * Math.sin(a0), sign * (r - radiusWall * Math.cos(a0)));
        g.lineTo(radiusWall * Math.sin(a1), sign * (r - radiusWall * Math.cos(a1)));
      }
      g.stroke({ width: 1.8, color: 0xcbd5e1, alpha: 0.9 });
    }

    if (hasStartPortal) drawPortalBracket2D(0, 0, 0);
    if (hasEndPortal) {
      const endX = r * Math.sin(rad);
      const endY = sign * r * (1 - Math.cos(rad));
      const endTangent = sign * rad;
      drawPortalBracket2D(endX, endY, endTangent + Math.PI);
    }
  } else {
    const l = entry.length ?? 188.3;
    g.rect(0, -tunnelHalfW, l, tunnelHalfW * 2);
    g.fill({ color: 0x0f172a, alpha: 0.32 });
    g.moveTo(0, -tunnelHalfW).lineTo(l, -tunnelHalfW);
    g.moveTo(0, tunnelHalfW).lineTo(l, tunnelHalfW);
    g.stroke({ width: 2.5, color: 0x64748b, alpha: 0.85 });
  }

  container.addChild(g);

  // Kleines Tunnel-Badge in der Mitte des Stücks
  const center = getPieceLocalCenter(entry);
  const badge = new Container();
  badge.position.set(center.x, center.y);
  badge.rotation = -deg2rad(piece.transform.rotation);

  const bg = new Graphics();
  bg.roundRect(-9, -7, 18, 14, 4);
  bg.fill({ color: 0x1e293b, alpha: 0.88 });
  bg.stroke({ width: 1, color: 0x94a3b8, alpha: 0.8 });
  badge.addChild(bg);

const TUNNEL_BADGE_TEXT_STYLE = new TextStyle({ fontSize: 9, fill: 0xffffff });

  const txt = new Text({
    text: "🚇",
    style: TUNNEL_BADGE_TEXT_STYLE,
  });
  txt.anchor.set(0.5);
  badge.addChild(txt);
  container.addChild(badge);
}

/**
 * Erzeugt die 3D-Tunnelröhre samt begrünter/felsiger Überdeckung und Steinportalen
 * für ein explizit als Tunnel markiertes Gleis oder eine Straße (sofern nicht bereits im Berg).
 */
export function buildPieceTunnel3D(
  group: THREE.Group,
  rawEntry: CatalogEntry,
  piece: TrackPiece,
  startElev: number,
  endElev: number,
  layout?: Layout,
): void {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const heightDiff = endElev - startElev;

  // Prüfe, ob ein Weltpunkt bereits im Inneren eines Berg-Modells liegt
  const mountains = (layout?.models ?? []).filter((m) => m.builtinId === "mountain");
  const isInsideMountain = (wx: number, wy: number): boolean => {
    for (const m of mountains) {
      const { lx, lz } = worldToMountainLocal(wx, wy, m);
      if (evaluateMountainLocal(lx, lz, m).normR <= 0.84) return true;
    }
    return false;
  };

  // Lokale Mittellinie des Gleises / der Straße berechnen
  const localPts: Array<{ x: number; y: number; z: number }> = [];
  if (isStraightTrackEntry(entry)) {
    const len = entry.length ?? 188.3;
    const steps = Math.max(4, Math.ceil(len / 35));
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      localPts.push({ x: len * t, y: heightDiff * t, z: 0 });
    }
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    const sign = a < 0 ? -1 : 1;
    const steps = Math.max(8, Math.ceil(absA / 3));
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      const curA = rad * t;
      localPts.push({
        x: r * Math.sin(curA),
        y: heightDiff * t,
        z: sign * r * (1 - Math.cos(curA)),
      });
    }
  } else {
    const len = entry.length ?? 188.3;
    localPts.push({ x: 0, y: 0, z: 0 }, { x: len * 0.5, y: heightDiff * 0.5, z: 0 }, { x: len, y: heightDiff, z: 0 });
  }

  if (localPts.length < 2) return;

  // Wenn das Stück bereits komplett in einem Berg-Massiv liegt, übernimmt der Berg den Tunnelbau
  const rotRad = deg2rad(piece.transform.rotation);
  const cosR = Math.cos(rotRad);
  const sinR = Math.sin(rotRad);
  const midLocal = localPts[Math.floor(localPts.length / 2)]!;
  const midWx = piece.transform.x + (midLocal.x * cosR - midLocal.z * sinR);
  const midWy = piece.transform.y + (midLocal.x * sinR + midLocal.z * cosR);
  if (isInsideMountain(midWx, midWy)) {
    return;
  }

  const prof = createTunnelProfile(!!entry.isRoad, !!piece.hasCatenary, entry.gauge ?? 16.5);

  const stonePortalMat = new THREE.MeshStandardMaterial({
    color: 0x52525b,
    roughness: 0.85,
    metalness: 0.12,
    side: THREE.DoubleSide,
  });
  const tunnelLiningMat = new THREE.MeshStandardMaterial({
    color: 0x27272a,
    roughness: 0.96,
    metalness: 0.05,
    side: THREE.DoubleSide,
  });
  const tunnelCoverMat = new THREE.MeshStandardMaterial({
    color: 0x3f6212,
    roughness: 0.92,
    metalness: 0.05,
    side: THREE.DoubleSide,
  });

  const path = localPts.map((p) => new THREE.Vector3(p.x, p.y + PORTAL_LIFT, p.z));
  const innerTube = buildTunnelSweep(path, prof, tunnelLiningMat, 1.0);
  if (innerTube) group.add(innerTube);

  // Begrünte / felsige Erdüberdeckung über der freistehenden Tunnelröhre
  const outerShell = buildTunnelSweep(path, prof, tunnelCoverMat, -8.0);
  if (outerShell) {
    outerShell.castShadow = true;
    outerShell.receiveShadow = true;
    group.add(outerShell);
  }

  const allPieces = layout?.pieces;
  const neighborHasTunnel = (connIdx: number): boolean => {
    const conn = piece.connectors[connIdx];
    if (!conn?.connectedTo || !allPieces) return false;
    const other = allPieces.find((p) => p.id === conn.connectedTo!.pieceId);
    if (!other) return false;
    if (other.hasTunnel) return true;
    return isInsideMountain(conn.x, conn.y);
  };

  // Start-Portal (Connector 0)
  if (!neighborHasTunnel(0)) {
    const p0 = localPts[0]!;
    const p1 = localPts[1]!;
    const dx = p1.x - p0.x;
    const dz = p1.z - p0.z;
    const len = Math.hypot(dx, dz) || 1;
    buildPortal(
      group,
      prof,
      { lx: p0.x, lz: p0.z, elev: p0.y },
      { x: dx / len, z: dz / len },
      stonePortalMat,
    );
  }

  // End-Portal (Connector 1)
  if (!neighborHasTunnel(1)) {
    const pN = localPts[localPts.length - 1]!;
    const pPrev = localPts[localPts.length - 2]!;
    const dx = pPrev.x - pN.x;
    const dz = pPrev.z - pN.z;
    const len = Math.hypot(dx, dz) || 1;
    buildPortal(
      group,
      prof,
      { lx: pN.x, lz: pN.z, elev: pN.y },
      { x: dx / len, z: dz / len },
      stonePortalMat,
    );
  }
}
