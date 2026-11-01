import * as THREE from "three";
import type { Model3D } from "@/core/types";

/**
 * Erzeugt fotorealistische, maßstäbliche prozedurale Three.js 3D-Modelle für alle Standard-Modellbahn-Objekte.
 */
export function createBuiltinModel3DMesh(builtinId: string, _model: Model3D): THREE.Group {
  const group = new THREE.Group();

  switch (builtinId) {
    case "station":
      buildStation3D(group);
      break;
    case "signal-box":
      buildSignalBox3D(group);
      break;
    case "engine-shed":
      buildEngineShed3D(group);
      break;
    case "town-house":
      buildTownHouse3D(group);
      break;
    case "water-tower":
      buildWaterTower3D(group);
      break;
    case "light-signal":
      buildLightSignal3D(group);
      break;
    case "buffer-stop":
      buildBufferStop3D(group);
      break;
    case "pine-tree":
      buildPineTree3D(group);
      break;
    case "deciduous-tree":
      buildDeciduousTree3D(group);
      break;
    case "platform-lamp":
      buildPlatformLamp3D(group);
      break;
    default:
      buildGenericBuilding3D(group);
      break;
  }

  // Schatten für alle Mesh-Bestandteile aktivieren
  group.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      obj.castShadow = true;
      obj.receiveShadow = true;
    }
  });

  return group;
}

/** 1. BAHNHOF NEUSTADT (320 x 140 x 120 mm) */
function buildStation3D(group: THREE.Group) {
  // Fundament
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.BoxGeometry(320, 4, 140), baseMat);
  base.position.set(0, 2, 0);
  group.add(base);

  // Hauptgebäude (Backstein / Sandstein)
  const wallMat = new THREE.MeshStandardMaterial({ color: 0xc87d55, roughness: 0.75 });
  const mainBuilding = new THREE.Mesh(new THREE.BoxGeometry(200, 70, 90), wallMat);
  mainBuilding.position.set(-20, 39, -10);
  group.add(mainBuilding);

  // Giebeldach Hauptgebäude
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x991b1b, roughness: 0.65 });
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-50, 0);
  roofShape.lineTo(0, 35);
  roofShape.lineTo(50, 0);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 216, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-128, 74, -10);
  const roof = new THREE.Mesh(roofGeo, roofMat);
  group.add(roof);

  // Uhrturm
  const tower = new THREE.Mesh(new THREE.BoxGeometry(45, 100, 45), wallMat);
  tower.position.set(-20, 54, -10);
  group.add(tower);

  // Uhrturm-Spitzdach
  const spireGeo = new THREE.ConeGeometry(32, 38, 4);
  spireGeo.rotateY(Math.PI / 4);
  const spireMat = new THREE.MeshStandardMaterial({ color: 0x1e3a5f, roughness: 0.4, metalness: 0.3 });
  const spire = new THREE.Mesh(spireGeo, spireMat);
  spire.position.set(-20, 123, -10);
  group.add(spire);

  // Bahnsteig-Vordach mit Stützen (Track side)
  const canopyMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.4 });
  const canopy = new THREE.Mesh(new THREE.BoxGeometry(280, 3, 40), canopyMat);
  canopy.position.set(0, 42, 45);
  group.add(canopy);

  // Vordach-Pfeiler
  const pillarMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  for (let x = -120; x <= 120; x += 60) {
    const pillar = new THREE.Mesh(new THREE.CylinderGeometry(2, 2, 40, 8), pillarMat);
    pillar.position.set(x, 22, 58);
    group.add(pillar);
  }

  // Fenster & Türen
  const winMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.1, metalness: 0.9 });
  for (let x = -95; x <= 55; x += 30) {
    const win = new THREE.Mesh(new THREE.BoxGeometry(14, 20, 1), winMat);
    win.position.set(x, 48, 36);
    group.add(win);
  }
}

/** 2. STELLWERK (120 x 75 x 110 mm) */
function buildSignalBox3D(group: THREE.Group) {
  // Untergeschoss (Klinker)
  const brickMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.85 });
  const groundFloor = new THREE.Mesh(new THREE.BoxGeometry(100, 45, 65), brickMat);
  groundFloor.position.set(0, 22.5, 0);
  group.add(groundFloor);

  // Obergeschoss (Ausstellend mit großen Fenstern)
  const topWallMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.6 });
  const topFloor = new THREE.Mesh(new THREE.BoxGeometry(112, 35, 75), topWallMat);
  topFloor.position.set(0, 62.5, 0);
  group.add(topFloor);

  // Panorama-Fensterband
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.15, metalness: 0.85 });
  const frontGlass = new THREE.Mesh(new THREE.BoxGeometry(106, 20, 1), glassMat);
  frontGlass.position.set(0, 66, 38);
  group.add(frontGlass);

  const backGlass = new THREE.Mesh(new THREE.BoxGeometry(106, 20, 1), glassMat);
  backGlass.position.set(0, 66, -38);
  group.add(backGlass);

  // Flachdach mit Dachrand
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.7 });
  const roof = new THREE.Mesh(new THREE.BoxGeometry(122, 5, 85), roofMat);
  roof.position.set(0, 82.5, 0);
  group.add(roof);

  // Außentreppe aus Stahl
  const stairMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.5, metalness: 0.6 });
  const stairs = new THREE.Mesh(new THREE.BoxGeometry(15, 45, 25), stairMat);
  stairs.position.set(58, 22.5, 0);
  stairs.rotation.z = -0.55;
  group.add(stairs);
}

/** 3. LOKSCHUPPEN (250 x 130 x 105 mm) */
function buildEngineShed3D(group: THREE.Group) {
  const wallMat = new THREE.MeshStandardMaterial({ color: 0x7c2d12, roughness: 0.85 });
  const shed = new THREE.Mesh(new THREE.BoxGeometry(240, 65, 120), wallMat);
  shed.position.set(0, 32.5, 0);
  group.add(shed);

  // Torgewölbe (schwarz / geöffnet)
  const portalMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.9 });
  const portal = new THREE.Mesh(new THREE.BoxGeometry(2, 50, 48), portalMat);
  portal.position.set(-121, 25, 0);
  group.add(portal);

  // Torgewölbebogen oben
  const arch = new THREE.Mesh(new THREE.CylinderGeometry(24, 24, 2, 16, 1, false, 0, Math.PI), portalMat);
  arch.rotateZ(Math.PI / 2);
  arch.rotateY(Math.PI / 2);
  arch.position.set(-121, 50, 0);
  group.add(arch);

  // Runddach / Tonnendach
  const roofGeo = new THREE.CylinderGeometry(68, 68, 246, 24, 1, false, 0, Math.PI);
  roofGeo.rotateZ(Math.PI / 2);
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.65 });
  const roof = new THREE.Mesh(roofGeo, roofMat);
  roof.position.set(0, 65, 0);
  group.add(roof);

  // Schornstein / Rauchabzug
  const chimMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.5 });
  const chimney = new THREE.Mesh(new THREE.CylinderGeometry(6, 7, 30, 12), chimMat);
  chimney.position.set(50, 95, 0);
  group.add(chimney);
}

/** 4. FACHWERKHAUS (110 x 90 x 125 mm) */
function buildTownHouse3D(group: THREE.Group) {
  // Erdgeschoss verputzt
  const baseMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, roughness: 0.8 });
  const ground = new THREE.Mesh(new THREE.BoxGeometry(100, 40, 80), baseMat);
  ground.position.set(0, 20, 0);
  group.add(ground);

  // Obergeschoss Fachwerk
  const upper = new THREE.Mesh(new THREE.BoxGeometry(105, 35, 85), baseMat);
  upper.position.set(0, 57.5, 0);
  group.add(upper);

  // Dunkelbraune Holzbalken
  const beamMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.9 });
  const beamH1 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 86), beamMat);
  beamH1.position.set(0, 40, 0);
  group.add(beamH1);

  const beamH2 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 86), beamMat);
  beamH2.position.set(0, 75, 0);
  group.add(beamH2);

  // Steiles Satteldach
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-46, 0);
  roofShape.lineTo(0, 45);
  roofShape.lineTo(46, 0);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 112, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-56, 75, 0);
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x991b1b, roughness: 0.6 });
  const roof = new THREE.Mesh(roofGeo, roofMat);
  group.add(roof);

  // Schornstein
  const chim = new THREE.Mesh(new THREE.BoxGeometry(12, 35, 12), new THREE.MeshStandardMaterial({ color: 0xb91c1c }));
  chim.position.set(20, 105, 0);
  group.add(chim);
}

/** 5. WASSERTURM (85 x 85 x 170 mm) */
function buildWaterTower3D(group: THREE.Group) {
  // Steinsockel
  const stoneMat = new THREE.MeshStandardMaterial({ color: 0x78716c, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.CylinderGeometry(30, 36, 90, 16), stoneMat);
  base.position.set(0, 45, 0);
  group.add(base);

  // Wasserbehälter (Stahlkessel)
  const tankMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.45, metalness: 0.35 });
  const tank = new THREE.Mesh(new THREE.CylinderGeometry(40, 40, 50, 20), tankMat);
  tank.position.set(0, 115, 0);
  group.add(tank);

  // Kegeldach
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5 });
  const roof = new THREE.Mesh(new THREE.ConeGeometry(44, 30, 20), roofMat);
  roof.position.set(0, 155, 0);
  group.add(roof);

  // Wasserkran-Schwenkarm
  const pipeMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.3, metalness: 0.8 });
  const arm = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 40, 8), pipeMat);
  arm.rotation.z = Math.PI / 2;
  arm.position.set(40, 85, 0);
  group.add(arm);

  const spout = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 20, 8), pipeMat);
  spout.position.set(58, 75, 0);
  group.add(spout);
}

/** 6. LICHTSIGNAL (25 x 25 x 85 mm) */
function buildLightSignal3D(group: THREE.Group) {
  // Betonsockel
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.BoxGeometry(20, 4, 20), baseMat);
  base.position.set(0, 2, 0);
  group.add(base);

  // Signalmast
  const mastMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.4, metalness: 0.7 });
  const mast = new THREE.Mesh(new THREE.CylinderGeometry(2, 2.5, 75, 8), mastMat);
  mast.position.set(0, 39.5, 0);
  group.add(mast);

  // Schaltschrank unten
  const boxMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.5, metalness: 0.5 });
  const box = new THREE.Mesh(new THREE.BoxGeometry(10, 16, 8), boxMat);
  box.position.set(0, 12, 0);
  group.add(box);

  // Signalschirm (schwarz mit runder Kante)
  const shieldMat = new THREE.MeshStandardMaterial({ color: 0x09090b, roughness: 0.7 });
  const shield = new THREE.Mesh(new THREE.BoxGeometry(12, 24, 2), shieldMat);
  shield.position.set(0, 68, 4);
  group.add(shield);

  // Grüne & rote Optik (mit Leuchten)
  const redMat = new THREE.MeshStandardMaterial({ color: 0xef4444, emissive: 0xef4444, emissiveIntensity: 0.6 });
  const redLight = new THREE.Mesh(new THREE.CylinderGeometry(2, 2, 2, 12), redMat);
  redLight.rotation.x = Math.PI / 2;
  redLight.position.set(0, 73, 5);
  group.add(redLight);

  const greenMat = new THREE.MeshStandardMaterial({ color: 0x22c55e, emissive: 0x22c55e, emissiveIntensity: 0.6 });
  const greenLight = new THREE.Mesh(new THREE.CylinderGeometry(2, 2, 2, 12), greenMat);
  greenLight.rotation.x = Math.PI / 2;
  greenLight.position.set(0, 63, 5);
  group.add(greenLight);
}

/** 7. PRELLBOCK (25 x 35 x 25 mm) */
function buildBufferStop3D(group: THREE.Group) {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x374151, roughness: 0.6, metalness: 0.6 });
  
  // A-Träger links & rechts
  const beamLeft = new THREE.Mesh(new THREE.BoxGeometry(3, 24, 30), steelMat);
  beamLeft.rotation.x = -0.4;
  beamLeft.position.set(-8.5, 12, 0);
  group.add(beamLeft);

  const beamRight = new THREE.Mesh(new THREE.BoxGeometry(3, 24, 30), steelMat);
  beamRight.rotation.x = -0.4;
  beamRight.position.set(8.5, 12, 0);
  group.add(beamRight);

  // Holz-Pufferbohle
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 });
  const bufferPlank = new THREE.Mesh(new THREE.BoxGeometry(26, 7, 6), woodMat);
  bufferPlank.position.set(0, 16, -10);
  group.add(bufferPlank);

  // Rote Signal-Kreisscheibe
  const diskMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.5 });
  const disk = new THREE.Mesh(new THREE.CylinderGeometry(4.5, 4.5, 1, 16), diskMat);
  disk.rotation.x = Math.PI / 2;
  disk.position.set(0, 16, -13.5);
  group.add(disk);
}

/** 8. FICHTE / TANNE (55 x 55 x 135 mm) */
function buildPineTree3D(group: THREE.Group) {
  // Stamm
  const trunkMat = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });
  const trunk = new THREE.Mesh(new THREE.CylinderGeometry(3.5, 6, 35, 8), trunkMat);
  trunk.position.set(0, 17.5, 0);
  group.add(trunk);

  // 4 gestufte Nadelkegel
  const needleColors = [0x14532d, 0x166534, 0x15803d, 0x16a34a];
  const radii = [27, 23, 18, 12];
  const heights = [40, 36, 32, 28];
  const yOffsets = [42, 65, 88, 110];

  for (let i = 0; i < 4; i++) {
    const mat = new THREE.MeshStandardMaterial({ color: needleColors[i], roughness: 0.95 });
    const cone = new THREE.Mesh(new THREE.ConeGeometry(radii[i], heights[i], 12), mat);
    cone.position.set(0, yOffsets[i]!, 0);
    group.add(cone);
  }
}

/** 9. LAUBBAUM (80 x 80 x 120 mm) */
function buildDeciduousTree3D(group: THREE.Group) {
  const trunkMat = new THREE.MeshStandardMaterial({ color: 0x443525, roughness: 0.9 });
  const trunk = new THREE.Mesh(new THREE.CylinderGeometry(5, 9, 50, 8), trunkMat);
  trunk.position.set(0, 25, 0);
  group.add(trunk);

  // Organische Laubkrone aus mehreren Kugeln
  const crownMat = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const mainCrown = new THREE.Mesh(new THREE.DodecahedronGeometry(36, 1), crownMat);
  mainCrown.position.set(0, 75, 0);
  group.add(mainCrown);

  const subCrown1 = new THREE.Mesh(new THREE.DodecahedronGeometry(22, 1), crownMat);
  subCrown1.position.set(16, 70, 12);
  group.add(subCrown1);

  const subCrown2 = new THREE.Mesh(new THREE.DodecahedronGeometry(20, 1), crownMat);
  subCrown2.position.set(-14, 82, -10);
  group.add(subCrown2);
}

/** 10. BAHNSTEIGLEUCHTE (20 x 20 x 70 mm) */
function buildPlatformLamp3D(group: THREE.Group) {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  // Mast
  const mast = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 2, 60, 8), steelMat);
  mast.position.set(0, 30, 0);
  group.add(mast);

  // Schwanenhals-Bogen
  const neckGeo = new THREE.TorusGeometry(8, 1.2, 8, 16, Math.PI / 2);
  neckGeo.rotateZ(Math.PI / 2);
  const neck = new THREE.Mesh(neckGeo, steelMat);
  neck.position.set(8, 60, 0);
  group.add(neck);

  // Lampenschirm
  const shadeMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.5 });
  const shade = new THREE.Mesh(new THREE.ConeGeometry(5, 3, 12), shadeMat);
  shade.position.set(16, 62, 0);
  group.add(shade);

  // Warmweißes Leuchtmittel
  const glowMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.9 });
  const bulb = new THREE.Mesh(new THREE.SphereGeometry(1.8, 8, 8), glowMat);
  bulb.position.set(16, 60.5, 0);
  group.add(bulb);
}

/** Fallback generisches Gebäude */
function buildGenericBuilding3D(group: THREE.Group) {
  const mat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.7 });
  const box = new THREE.Mesh(new THREE.BoxGeometry(60, 50, 60), mat);
  box.position.set(0, 25, 0);
  group.add(box);
}

