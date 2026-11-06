import type { Connector, Transform } from "@/core/types";

export type TrackKind =
  | "straight"
  | "curve"
  | "turnout-left"
  | "turnout-right"
  | "curved-turnout-left"
  | "curved-turnout-right"
  | "uncoupler"
  | "three-way"
  | "crossing"
  | "flex"
  | "turntable"
  | "feeder"
  | "buffer-stop"
  | "dkw"
  | "isolating"
  | "accessory";

export interface LocalConnector {
  x: number;
  y: number;
  angle: number;
}

export interface CatalogEntry {
  id: string;
  name: string;
  nameEn?: string;
  kind: TrackKind;
  category: "Geraden" | "Bögen" | "Weichen" | "Kreuzungen" | "Zubehör" | string;
  description: string;
  descriptionEn?: string;
  length?: number;
  radius?: number;
  angle?: number;
  branchRadius?: number;
  branchAngle?: number;
  branchLength?: number;
  gauge?: number; // Spurweite in mm (falls abweichend vom System)
  hasRoadbed?: boolean; // Bettungsgleis (true) oder Schwellengleis (false)
  isRoad?: boolean; // Kennzeichnung als Straße / Fahrbahn
  roadWidth?: number; // Fahrbahnbreite in mm (Standard: 60mm)
  localConnectors?: LocalConnector[];
}

export interface Catalog {
  id: string;
  name: string;
  manufacturer: string;
  system: string;
  scale: "H0" | "TT" | "N" | "Z" | string;
  gauge: number; // Spurweite in mm (H0: 16.5, TT: 12.0, N: 9.0, Z: 6.5)
  hasRoadbed?: boolean; // Bettungsgleis (true) oder Schwellengleis (false)
  conductors?: number; // Leiter-Anzahl: 2 oder 3 Leiter
  isRoadSystem?: boolean; // Kennzeichnung als Straßensystem
  description?: string;
  entries: CatalogEntry[];
}
