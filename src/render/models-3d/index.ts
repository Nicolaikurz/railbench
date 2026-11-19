import * as THREE from "three";
import { buildStation3D, buildStationLowPoly3D } from "./station";
import { buildSignalBox3D, buildSignalBoxLowPoly3D } from "./signal-box";
import { buildEngineShed3D } from "./engine-shed";
import { buildTownHouse3D, buildTownHouseLowPoly3D } from "./town-house";
import { buildWaterTower3D } from "./water-tower";
import { buildLightSignal3D } from "./light-signal";
import { buildBufferStop3D } from "./buffer-stop";
import { buildPineTree3D, buildPineTreeLowPoly3D } from "./pine-tree";
import { buildDeciduousTree3D, buildDeciduousTreeLowPoly3D } from "./deciduous-tree";
import { buildPlatformLamp3D } from "./platform-lamp";
import { buildPlatform3D } from "./platform";
import { buildGenericBuilding3D } from "./generic-building";
import { buildTurntableTrainhall3D } from "./turntable-trainhall";
import { buildSkyscraper3D } from "./skyscraper";
import { buildIndustryBuilding3D } from "./industry-building";
import { buildMountain3D, findCollidingTracksForMountain } from "./mountain";
import { buildGroundTerrain3D } from "./ground-terrain";
import { buildCoalingStation3D } from "./coaling-station";
import { buildDieselFuelStation3D } from "./diesel-fuel-station";

export {
  buildStation3D,
  buildStationLowPoly3D,
  buildSignalBox3D,
  buildSignalBoxLowPoly3D,
  buildEngineShed3D,
  buildTownHouse3D,
  buildTownHouseLowPoly3D,
  buildWaterTower3D,
  buildLightSignal3D,
  buildBufferStop3D,
  buildPineTree3D,
  buildPineTreeLowPoly3D,
  buildDeciduousTree3D,
  buildDeciduousTreeLowPoly3D,
  buildPlatformLamp3D,
  buildPlatform3D,
  buildGenericBuilding3D,
  buildTurntableTrainhall3D,
  buildSkyscraper3D,
  buildIndustryBuilding3D,
  buildMountain3D,
  buildGroundTerrain3D,
  findCollidingTracksForMountain,
  buildCoalingStation3D,
  buildDieselFuelStation3D,
};

export * from "./primitive-cache";

export type Model3DBuilder = (group: THREE.Group, model?: any, extraData?: any) => void;

export const BUILTIN_MODEL_BUILDERS: Record<string, Model3DBuilder> = {
  station: buildStation3D,
  "signal-box": buildSignalBox3D,
  "engine-shed": buildEngineShed3D,
  "town-house": buildTownHouse3D,
  "water-tower": buildWaterTower3D,
  "coaling-station": buildCoalingStation3D,
  "diesel-fuel-station": buildDieselFuelStation3D,
  "light-signal": buildLightSignal3D,
  "buffer-stop": buildBufferStop3D,
  "pine-tree": buildPineTree3D,
  "deciduous-tree": buildDeciduousTree3D,
  "platform-lamp": buildPlatformLamp3D,
  platform: buildPlatform3D,
  bahnsteig: buildPlatform3D,
  "turntable-trainhall": buildTurntableTrainhall3D,
  skyscraper: buildSkyscraper3D,
  "industry-building": buildIndustryBuilding3D,
  mountain: buildMountain3D,
  "ground-terrain": buildGroundTerrain3D,
};

export const BUILTIN_MODEL_LOW_POLY_BUILDERS: Record<string, Model3DBuilder> = {
  station: buildStationLowPoly3D,
  "signal-box": buildSignalBoxLowPoly3D,
  "town-house": buildTownHouseLowPoly3D,
  "pine-tree": buildPineTreeLowPoly3D,
  "deciduous-tree": buildDeciduousTreeLowPoly3D,
};
