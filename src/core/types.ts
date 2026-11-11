export interface Vec2 { x: number; y: number }

/** Anschlusspunkt eines Gleisstücks (Weltkoordinaten, mm). angle in Grad, zeigt vom Stück weg. */
export interface Connector {
  x: number;
  y: number;
  angle: number;
  connectedTo?: { pieceId: string; connectorIndex: number };
}

export interface Transform { x: number; y: number; rotation: number }

export interface TrackPiece {
  id: string;
  catalogId: string;
  transform: Transform;
  connectors: Connector[];
  elevation?: { start: number; end: number };
  powerFeed?: boolean;
  isIsolator?: boolean;
  isolatedConnectorIndex?: number;
  hasCatenary?: boolean;
  catenaryPowerFeed?: boolean;
  catenaryIsolator?: boolean;
  catenaryIsolatedConnectorIndex?: number;
  hasLantern?: boolean;
  hasDecoder?: boolean;
  turnoutCurrentMa?: number;
  customName?: string;
  customLength?: number;
  customRadius?: number;
  customAngle?: number;
  groupId?: string;
  layerId?: string;
}

export type ColorMode = "off" | "types" | "layers";

export type LayerWoodPlateMode = "none" | "table" | "tight";

export interface Layer {
  id: string;
  name: string;
  color?: string;
  visible: boolean;
  locked: boolean;
  heightMm?: number;
  woodPlateMode?: LayerWoodPlateMode;
  woodPlateThicknessMm?: number;
}

export type BaseboardStyle = "wood" | "dark-wood" | "plywood" | "slate" | "white" | "grass";
export type BaseboardShape = "rect" | "l-shape" | "u-shape" | "t-shape";

export interface BaseboardConfig {
  id?: string;
  name?: string;
  enabled: boolean;
  shape?: BaseboardShape;
  width: number; // in mm, z.B. 2400
  depth: number; // in mm, z.B. 1200
  // L-Form Parameter
  lWingWidth?: number;
  lWingDepth?: number;
  lSide?: "left" | "right";
  // U-Form Parameter
  uLeftWingWidth?: number;
  uRightWingWidth?: number;
  uBackDepth?: number;
  // T-Form Parameter
  tBarDepth?: number;
  tStemWidth?: number;
  positionMode: "center" | "top-left" | "custom";
  customX?: number; // in mm
  customY?: number; // in mm
  style: BaseboardStyle;
  cornerRadius?: number; // in mm, z.B. 12
  showDimensions?: boolean; // z.B. true
}

export type ModelCategory = "buildings" | "nature" | "railway" | "custom";

export interface Model3D {
  id: string;
  name: string;
  category: ModelCategory;
  transform: Transform; // x, y, rotation in degrees
  elevation?: number;   // height offset above layer (mm)
  scale?: number;       // uniform scale factor (default: 1.0)
  layerId?: string;
  width: number;        // footprint width in mm
  depth: number;        // footprint depth in mm
  height: number;       // vertical height in mm
  modelType: "builtin" | "glb" | "gltf";
  builtinId?: string;
  fileData?: string;    // base64 data URI for glb / text for gltf
  fileName?: string;
}

export interface Layout {
  name: string;
  scale: "H0" | "TT" | "N" | "Z" | "1" | "G" | "0";
  pieces: TrackPiece[];
  models?: Model3D[];
  feedVoltage?: number;
  catenaryFeedVoltage?: number;
  catenaryCircuitMode?: "separate" | "parallel";
  locoCurrentMa?: number; // Stromaufnahme pro Lok in mA (50 - 1000 mA, max. 1A)
  locoCount?: number;     // Anzahl gleichzeitig fahrender Loks (Standard: 1)
  trainCurrentAmps?: number; // Gesamtstromlast in Ampere
  turnoutLanternCurrentMa?: number; // Stromaufnahme pro Weichenlaterne in mA (Standard: 20 mA)
  turnoutDecoderCurrentMa?: number; // Ruhestrom pro Weichendecoder in mA (Standard: 15 mA)
  layers?: Layer[];
  activeLayerId?: string;
  baseboard?: BaseboardConfig;
  baseboards?: BaseboardConfig[];
}
