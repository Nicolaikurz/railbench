import type { TrackPiece, Transform } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { normAngle } from "./geometry";

export interface TurntableSnapResult {
  snappedTransform: Transform;
  turntablePiece: TrackPiece;
  exitAngle: number;
  dist: number;
}

/**
 * Snaps a model (such as turntable-trainhall / Ringlokschuppen) to the nearest
 * turntable piece in the layout, aligning it radially to the turntable's exit tracks.
 *
 * @param candTransform The candidate transform (x, y, rotation) of the model
 * @param pieces All track pieces in the layout
 * @param getCatalogEntry Callback to retrieve catalog entry for a track piece
 * @param radialSnapTolerance Radial distance tolerance in mm (default 75 mm)
 * @param angleSnapTolerance Angular difference tolerance in degrees (default 15 deg)
 * @param modelDepth Depth of the model in mm (default 310 mm for turntable-trainhall)
 */
export function snapModelToTurntable(
  candTransform: Transform,
  pieces: TrackPiece[],
  getCatalogEntry: (p: TrackPiece) => CatalogEntry | undefined,
  radialSnapTolerance = 75,
  angleSnapTolerance = 15,
  modelDepth = 310,
): TurntableSnapResult | null {
  let bestResult: TurntableSnapResult | null = null;
  let bestDist = Infinity;

  for (const p of pieces) {
    const entry = getCatalogEntry(p);
    if (!entry || entry.kind !== "turntable") continue;

    const ttCenter = { x: p.transform.x, y: p.transform.y };
    const ttRadius = entry.radius ?? (entry.length ? entry.length / 2 : 189);

    // Target radial distance from turntable center to center of the trainhall
    // Front entrance is at ttRadius + ~6 mm, center of trainhall is at ttRadius + 6 + modelDepth / 2
    const targetDist = ttRadius + 6 + modelDepth / 2;

    const dx = candTransform.x - ttCenter.x;
    const dy = candTransform.y - ttCenter.y;
    const currentDist = Math.hypot(dx, dy);

    // Radial distance check
    if (Math.abs(currentDist - targetDist) > radialSnapTolerance) {
      continue;
    }

    // Current angle from turntable center to candidate in degrees [-180, 180]
    const currentAngleDeg = normAngle(Math.atan2(dy, dx) * (180 / Math.PI));

    // Determine available exit track angles of the turntable
    const exitAngles: number[] = [];

    if (entry.localConnectors && entry.localConnectors.length > 0) {
      for (const lc of entry.localConnectors) {
        const baseAngle = Math.atan2(lc.y, lc.x) * (180 / Math.PI);
        const worldAngle = normAngle(baseAngle + p.transform.rotation);
        exitAngles.push(worldAngle);
      }
    } else {
      const step = entry.angle ?? 12;
      for (let a = 0; a < 360; a += step) {
        exitAngles.push(normAngle(a + p.transform.rotation));
      }
    }

    // Find closest exit angle
    let closestAngle = exitAngles[0] ?? 0;
    let minAngleDiff = Infinity;

    for (const ea of exitAngles) {
      let diff = Math.abs(normAngle(currentAngleDeg - ea));
      if (diff > 180) diff = 360 - diff;
      if (diff < minAngleDiff) {
        minAngleDiff = diff;
        closestAngle = ea;
      }
    }

    if (minAngleDiff > angleSnapTolerance) {
      continue;
    }

    // Compute snapped position & rotation
    const rad = closestAngle * (Math.PI / 180);
    const snapX = Math.round(ttCenter.x + targetDist * Math.cos(rad));
    const snapY = Math.round(ttCenter.y + targetDist * Math.sin(rad));
    // Rotation: front of trainhall (facing -Y locally) faces turntable center (angle - 90°)
    const snapRot = normAngle(closestAngle - 90);

    const distToCandidate = Math.hypot(snapX - candTransform.x, snapY - candTransform.y);
    if (distToCandidate < bestDist) {
      bestDist = distToCandidate;
      bestResult = {
        snappedTransform: { x: snapX, y: snapY, rotation: snapRot },
        turntablePiece: p,
        exitAngle: closestAngle,
        dist: distToCandidate,
      };
    }
  }

  return bestResult;
}
