import * as THREE from "three";
import { buildStation3D } from "./station";
import { buildSignalBox3D } from "./signal-box";
import { buildEngineShed3D } from "./engine-shed";
import { buildTownHouse3D } from "./town-house";
import { buildWaterTower3D } from "./water-tower";
import { buildLightSignal3D } from "./light-signal";
import { buildBufferStop3D } from "./buffer-stop";
import { buildPineTree3D } from "./pine-tree";
import { buildDeciduousTree3D } from "./deciduous-tree";
import { buildPlatformLamp3D } from "./platform-lamp";
import { buildGenericBuilding3D } from "./generic-building";
import { buildTurntableTrainhall3D } from "./turntable-trainhall";
import { buildSkyscraper3D } from "./skyscraper";
import { buildIndustryBuilding3D } from "./industry-building";

export {
  buildStation3D,
  buildSignalBox3D,
  buildEngineShed3D,
  buildTownHouse3D,
  buildWaterTower3D,
  buildLightSignal3D,
  buildBufferStop3D,
  buildPineTree3D,
  buildDeciduousTree3D,
  buildPlatformLamp3D,
  buildGenericBuilding3D,
  buildTurntableTrainhall3D,
  buildSkyscraper3D,
  buildIndustryBuilding3D,
};

export type Model3DBuilder = (group: THREE.Group) => void;

export const BUILTIN_MODEL_BUILDERS: Record<string, Model3DBuilder> = {
  station: buildStation3D,
  "signal-box": buildSignalBox3D,
  "engine-shed": buildEngineShed3D,
  "town-house": buildTownHouse3D,
  "water-tower": buildWaterTower3D,
  "light-signal": buildLightSignal3D,
  "buffer-stop": buildBufferStop3D,
  "pine-tree": buildPineTree3D,
  "deciduous-tree": buildDeciduousTree3D,
  "platform-lamp": buildPlatformLamp3D,
  "turntable-trainhall": buildTurntableTrainhall3D,
  skyscraper: buildSkyscraper3D,
  "industry-building": buildIndustryBuilding3D,
};
