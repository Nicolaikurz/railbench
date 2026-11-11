import type { Connector, TrackPiece, Transform, Model3D } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { getLocalConnectors } from "@/catalog/catalog";
import { dist, deg2rad, normAngle, rotate, transformPoint } from "./geometry";
import type { BestPieceSnap } from "./snapping";

export interface BuildingDoorConnector {
  x: number;
  y: number;
  inwardAngle: number;   // Winkel IN das Gebäude / den Stand hinein
  outwardAngle: number;  // Winkel AUS dem Gebäude heraus (dem herannahenden Gleis entgegen)
  stallIndex: number;
  modelId: string;
  builtinId?: string;
}

export interface EngineShedSnapResult {
  snappedTransform: Transform;
  targetPiece: TrackPiece;
  connectorIndex: number;
  dist: number;
  stallIndex?: number;
}

/**
 * Gibt den Anschlusspunkt (Connector) am Einfahrtsportal des Lokschuppens zurück.
 * Der Connector liegt am vorderen Tor (lokal: x = -shedLength/2, y = 0)
 * und zeigt mit seinem Winkel nach außen dem herannahenden Gleis entgegen (rotation + 180°).
 */
export function getEngineShedDoorConnector(
  shed: { transform: Transform; width?: number },
  shedLength = 240,
): Connector {
  const length = shed.width ?? shedLength;
  const localDoor = { x: -length / 2, y: 0 };
  const worldDoor = transformPoint(localDoor, shed.transform);
  const worldAngle = normAngle(shed.transform.rotation + 180);

  return {
    x: Math.round(worldDoor.x * 10) / 10,
    y: Math.round(worldDoor.y * 10) / 10,
    angle: worldAngle,
  };
}

/**
 * Liefert alle Tor-Connectoren für Gebäude mit Gleisanschluss:
 * - "engine-shed" (1 Tor bei x = -120 mm)
 * - "turntable-trainhall" (3 Tore bei -12°, 0°, +12°)
 */
export function getBuildingDoorConnectors(model: Model3D): BuildingDoorConnector[] {
  if (model.builtinId === "engine-shed") {
    const length = model.width ?? 240;
    const localDoor = { x: -length / 2, y: 0 };
    const worldDoor = transformPoint(localDoor, model.transform);
    const inward = normAngle(model.transform.rotation);
    const outward = normAngle(model.transform.rotation + 180);

    return [
      {
        x: Math.round(worldDoor.x * 10) / 10,
        y: Math.round(worldDoor.y * 10) / 10,
        inwardAngle: inward,
        outwardAngle: outward,
        stallIndex: 0,
        modelId: model.id,
        builtinId: model.builtinId,
      },
    ];
  }

  if (model.builtinId === "turntable-trainhall") {
    // Ringlokschuppen hat 3 Stände bei -12°, 0°, +12°
    // TT_Z = -350, R_FRONT = 195 (Abstand vom Drehscheibenzentrum zum Torportal)
    const stalls = [
      { angleDeg: -12, x: -40.54, y: -159.35, inAngle: 78, outAngle: 258 },
      { angleDeg: 0, x: 0, y: -155.0, inAngle: 90, outAngle: 270 },
      { angleDeg: 12, x: 40.54, y: -159.35, inAngle: 102, outAngle: 282 },
    ];

    return stalls.map((s, idx) => {
      const worldDoor = transformPoint({ x: s.x, y: s.y }, model.transform);
      const inward = normAngle(s.inAngle + model.transform.rotation);
      const outward = normAngle(s.outAngle + model.transform.rotation);
      return {
        x: Math.round(worldDoor.x * 10) / 10,
        y: Math.round(worldDoor.y * 10) / 10,
        inwardAngle: inward,
        outwardAngle: outward,
        stallIndex: idx,
        modelId: model.id,
        builtinId: model.builtinId,
      };
    });
  }

  return [];
}

/**
 * Prüft, ob ein Gleis-Connector geometrisch an einem Gebäude-Tor eingerastet ist.
 */
export function isConnectorConnectedToBuilding(
  conn: { x: number; y: number },
  models?: Model3D[],
  tolerance = 6.0,
): { model: Model3D; connector: BuildingDoorConnector } | null {
  if (!models) return null;
  for (const m of models) {
    const doors = getBuildingDoorConnectors(m);
    for (const d of doors) {
      if (dist(conn, d) <= tolerance) {
        return { model: m, connector: d };
      }
    }
  }
  return null;
}

/**
 * Prüft, ob ein Gebäude an mindestens einem Gleis eingerastet ist.
 */
export function isBuildingSnapped(
  model: Model3D,
  pieces: TrackPiece[],
  tolerance = 6.0,
): boolean {
  const doors = getBuildingDoorConnectors(model);
  if (doors.length === 0) return false;

  for (const door of doors) {
    for (const p of pieces) {
      for (const c of p.connectors) {
        if (dist(c, door) <= tolerance) {
          return true;
        }
      }
    }
  }
  return false;
}

/**
 * Rastet einen Lokschuppen (engine-shed) an das nächstgelegene offene Gleisende ein.
 */
export function snapEngineShedToTrack(
  candTransform: Transform,
  pieces: TrackPiece[],
  _getCatalogEntry?: (p: TrackPiece) => CatalogEntry | undefined,
  snapTolerance = 50,
  angleTolerance = 75,
  shedLength = 240,
): EngineShedSnapResult | null {
  const localDoor = { x: -shedLength / 2, y: 0 };
  const candDoor = transformPoint(localDoor, candTransform);

  let bestResult: EngineShedSnapResult | null = null;
  let bestScore = Infinity;

  for (const piece of pieces) {
    for (let ci = 0; ci < piece.connectors.length; ci++) {
      const conn = piece.connectors[ci]!;

      const d = dist(candDoor, conn);
      if (d > snapTolerance) continue;

      // Check both orientations along the track line:
      // Option A: Track approaches from outside (shed faces along conn.angle)
      // Option B: Track extends inside shed (shed faces along conn.angle + 180)
      const candRot = candTransform.rotation;
      let diffA = Math.abs(normAngle(candRot - conn.angle));
      if (diffA > 180) diffA = 360 - diffA;

      let diffB = Math.abs(normAngle(candRot - (conn.angle + 180)));
      if (diffB > 180) diffB = 360 - diffB;

      const useB = diffB < diffA;
      const angleDiff = useB ? diffB : diffA;
      const targetAngle = normAngle(useB ? conn.angle + 180 : conn.angle);

      const maxAllowedAngle = d <= 25 ? 180 : angleTolerance;
      if (angleDiff > maxAllowedAngle) continue;

      const score = d + (angleDiff / 180) * 20;
      if (score < bestScore) {
        bestScore = score;

        const snappedRot = targetAngle;
        const rad = deg2rad(snappedRot);
        const snappedX = Math.round((conn.x + (shedLength / 2) * Math.cos(rad)) * 10) / 10;
        const snappedY = Math.round((conn.y + (shedLength / 2) * Math.sin(rad)) * 10) / 10;

        bestResult = {
          snappedTransform: {
            x: snappedX,
            y: snappedY,
            rotation: snappedRot,
          },
          targetPiece: piece,
          connectorIndex: ci,
          dist: d,
        };
      }
    }
  }

  return bestResult;
}

/**
 * Rastet einen Ringlokschuppen (turntable-trainhall) an ein Gleisende ein,
 * indem einer der 3 Stände passgenau an den Gleisanschluss gelegt wird.
 */
export function snapTrainhallToTrack(
  candTransform: Transform,
  pieces: TrackPiece[],
  snapTolerance = 50,
  angleTolerance = 60,
): EngineShedSnapResult | null {
  const fakeModel: Model3D = {
    id: "temp",
    name: "Ringlokschuppen",
    category: "railway",
    modelType: "builtin",
    builtinId: "turntable-trainhall",
    transform: candTransform,
    width: 320,
    depth: 310,
    height: 115,
  };

  const doors = getBuildingDoorConnectors(fakeModel);
  let bestResult: EngineShedSnapResult | null = null;
  let bestScore = Infinity;

  for (const door of doors) {
    for (const piece of pieces) {
      for (let ci = 0; ci < piece.connectors.length; ci++) {
        const conn = piece.connectors[ci]!;

        const d = dist(door, conn);
        if (d > snapTolerance) continue;

        // Conn angle: track line can be inward or outward relative to door portal
        let diffIn = Math.abs(normAngle(door.inwardAngle - conn.angle));
        if (diffIn > 180) diffIn = 360 - diffIn;

        let diffOut = Math.abs(normAngle(door.inwardAngle - (conn.angle + 180)));
        if (diffOut > 180) diffOut = 360 - diffOut;

        const useOut = diffOut < diffIn;
        const angleDiff = useOut ? diffOut : diffIn;
        const effectiveConnAngle = normAngle(useOut ? conn.angle + 180 : conn.angle);

        const maxAllowedAngle = d <= 25 ? 180 : angleTolerance;
        if (angleDiff > maxAllowedAngle) continue;

        const score = d + (angleDiff / 180) * 20;
        if (score < bestScore) {
          bestScore = score;

          // Winkel-Korrektur für den gesamten Ringlokschuppen
          const deltaAngle = normAngle(effectiveConnAngle - door.inwardAngle);
          const snappedRot = normAngle(candTransform.rotation + deltaAngle);

          // Position des Tors in Modell-Koordinaten:
          const localStall = door.stallIndex === 0
            ? { x: -40.54, y: -159.35 }
            : door.stallIndex === 1
              ? { x: 0, y: -155.0 }
              : { x: 40.54, y: -159.35 };

          const rotOffset = rotate(localStall, snappedRot);
          const snappedX = Math.round((conn.x - rotOffset.x) * 10) / 10;
          const snappedY = Math.round((conn.y - rotOffset.y) * 10) / 10;

          bestResult = {
            snappedTransform: {
              x: snappedX,
              y: snappedY,
              rotation: snappedRot,
            },
            targetPiece: piece,
            connectorIndex: ci,
            dist: d,
            stallIndex: door.stallIndex,
          };
        }
      }
    }
  }

  return bestResult;
}

/**
 * Ermöglicht es einem bewegten Gleisstück, an Gebäude-Tore anzudocken:
 * - Lokschuppen ("engine-shed"): Zufahrtsgleis von außen ODER Gleis IM Schuppen
 * - Ringlokschuppen ("turntable-trainhall"): Alle 3 Stände (von außen ODER im Schuppen)
 */
export function snapTrackPieceToBuilding(
  entry: CatalogEntry,
  candTransform: Transform,
  models: Model3D[],
  snapTolerance = 45,
  angleTolerance = 60,
): BestPieceSnap | null {
  const locals = getLocalConnectors(entry);
  let best: BestPieceSnap | null = null;
  let bestScore = Infinity;

  for (const m of models) {
    const doors = getBuildingDoorConnectors(m);
    if (doors.length === 0) continue;

    for (const door of doors) {
      for (let mi = 0; mi < locals.length; mi++) {
        const local = locals[mi]!;
        const movingWorldPos = transformPoint(local, candTransform);
        const movingWorldAngle = normAngle(local.angle + candTransform.rotation);

        const d = dist(movingWorldPos, door);
        if (d > snapTolerance) continue;

        // OPTION 1: Gleis führt von AUSSEN an das Tor heran
        // (movingWorldAngle steht entgegengesetzt zu door.outwardAngle -> Diff ~ 180°)
        let diffOutside = Math.abs(normAngle(movingWorldAngle - door.outwardAngle) - 180);
        if (diffOutside > 180) diffOutside = 360 - diffOutside;

        // OPTION 2: Gleis liegt IM LOKSCHUPPEN (führt vom Tor nach innen)
        // (Für Connector 0 mit local.angle = 180°: movingWorldAngle steht in Richtung door.outwardAngle)
        let diffInside = Math.abs(normAngle(movingWorldAngle - door.outwardAngle));
        if (diffInside > 180) diffInside = 360 - diffInside;

        const isInside = diffInside < diffOutside;
        const diff = isInside ? diffInside : diffOutside;

        const effectiveAngleTolerance = d <= 25 ? 180 : angleTolerance;
        if (diff > effectiveAngleTolerance) continue;

        const score = d + (diff / 180) * 20;
        if (score < bestScore) {
          bestScore = score;

          // Exakte Ausrichtung berechnen
          let snappedRot: number;
          if (isInside) {
            // Gleis erstreckt sich in das Gebäude hinein
            snappedRot = normAngle(door.inwardAngle + 180 - local.angle);
          } else {
            // Gleis führt von außen an das Tor heran
            snappedRot = normAngle(door.outwardAngle + 180 - local.angle);
          }

          const offset = rotate(local, snappedRot);
          const snappedX = Math.round((door.x - offset.x) * 10) / 10;
          const snappedY = Math.round((door.y - offset.y) * 10) / 10;

          best = {
            movingConnectorIndex: mi,
            targetPieceId: m.id,
            targetConnectorIndex: door.stallIndex,
            snappedTransform: {
              x: snappedX,
              y: snappedY,
              rotation: snappedRot,
            },
            dist: d,
          };
        }
      }
    }
  }

  return best;
}

/**
 * Abwärtskompatibilitäts-Wrapper für bisherige Lokschuppen-Aufrufe.
 */
export function snapTrackPieceToEngineShed(
  entry: CatalogEntry,
  candTransform: Transform,
  models: Model3D[],
  snapTolerance = 45,
  angleTolerance = 60,
): BestPieceSnap | null {
  return snapTrackPieceToBuilding(entry, candTransform, models, snapTolerance, angleTolerance);
}
