import * as THREE from "three";
import { applyModelScale, type ModelScale } from "@/core/model-scale";
import type { Model3D } from "@/core/types";

export function createBuiltinModel3DMesh(
  builtinId: string,
  _model: Model3D,
  scale: ModelScale = "H0",
): THREE.Group {
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
  applyModelScale(group, scale);
  return group;
}

/** 1. BAHNHOF NEUSTADT (320 x 140 x 120 mm) */
function buildStation3D(group: THREE.Group) {
  // Landschaftsplatte / Vorplatz & Grünfläche um den Bahnhof
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const grassBase = new THREE.Mesh(new THREE.BoxGeometry(380, 3, 200), grassMat);
  grassBase.position.set(0, 1.5, 5);
  group.add(grassBase);

  // Weg / Vorplatz zum Bahnhofs-Eingang
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.75 });
  const curbMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.8 });
  
  // Hauptweg / Fußweg von außen zum Haupteingang
  const mainPath = new THREE.Mesh(new THREE.BoxGeometry(60, 1, 95), pathMat);
  mainPath.position.set(-20, 3.2, -55);
  group.add(mainPath);

  // Randsteine des Weges
  const curbL = new THREE.Mesh(new THREE.BoxGeometry(3, 2, 95), curbMat);
  curbL.position.set(-51, 3.5, -55);
  group.add(curbL);

  const curbR = new THREE.Mesh(new THREE.BoxGeometry(3, 2, 95), curbMat);
  curbR.position.set(11, 3.5, -55);
  group.add(curbR);

  // Blumenbeet & Büsche am Weg
  const bushMat1 = new THREE.MeshStandardMaterial({ color: 0x166534, roughness: 0.9 });
  const bushMat2 = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const bushOffsets = [-70, -85, 30, 45, 60];
  for (const bx of bushOffsets) {
    const bush = new THREE.Mesh(new THREE.DodecahedronGeometry(8, 1), bx > 0 ? bushMat1 : bushMat2);
    bush.position.set(bx, 8, -55);
    group.add(bush);
  }

  // Bäume am Vorplatz
  const treeTrunk = new THREE.Mesh(new THREE.CylinderGeometry(2.5, 4, 30, 8), new THREE.MeshStandardMaterial({ color: 0x451a03 }));
  treeTrunk.position.set(-135, 15, -60);
  group.add(treeTrunk);
  const treeCrown = new THREE.Mesh(new THREE.DodecahedronGeometry(22, 1), new THREE.MeshStandardMaterial({ color: 0x15803d }));
  treeCrown.position.set(-135, 42, -60);
  group.add(treeCrown);

  // Fundament des Gebäudes
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.BoxGeometry(320, 4, 140), baseMat);
  base.position.set(0, 3.5, 0);
  group.add(base);

  // Hauptgebäude (Backstein / Sandstein)
  const wallMat = new THREE.MeshStandardMaterial({ color: 0xc87d55, roughness: 0.75 });
  const mainBuilding = new THREE.Mesh(new THREE.BoxGeometry(200, 70, 90), wallMat);
  mainBuilding.position.set(-20, 40.5, -10);
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
  roofGeo.translate(-128, 75.5, -10);
  const roof = new THREE.Mesh(roofGeo, roofMat);
  group.add(roof);

  // Uhrturm
  const tower = new THREE.Mesh(new THREE.BoxGeometry(45, 100, 45), wallMat);
  tower.position.set(-20, 55.5, -10);
  group.add(tower);

  // Uhrturm-Spitzdach
  const spireGeo = new THREE.ConeGeometry(32, 38, 4);
  spireGeo.rotateY(Math.PI / 4);
  const spireMat = new THREE.MeshStandardMaterial({ color: 0x1e3a5f, roughness: 0.4, metalness: 0.3 });
  const spire = new THREE.Mesh(spireGeo, spireMat);
  spire.position.set(-20, 124.5, -10);
  group.add(spire);

  // Bahnsteig-Vordach mit Stützen (Track side)
  const canopyMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.4 });
  const canopy = new THREE.Mesh(new THREE.BoxGeometry(280, 3, 40), canopyMat);
  canopy.position.set(0, 43.5, 45);
  group.add(canopy);

  // Vordach-Pfeiler
  const pillarMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  for (let x = -120; x <= 120; x += 60) {
    const pillar = new THREE.Mesh(new THREE.CylinderGeometry(2, 2, 40, 8), pillarMat);
    pillar.position.set(x, 23.5, 58);
    group.add(pillar);
  }

  // Fenster & Türen
  const winMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.1, metalness: 0.9 });
  for (let x = -95; x <= 55; x += 30) {
    const win = new THREE.Mesh(new THREE.BoxGeometry(14, 20, 1), winMat);
    win.position.set(x, 49.5, 36);
    group.add(win);
  }
}

/** 2. STELLWERK (120 x 75 x 110 mm) */
function buildSignalBox3D(group: THREE.Group) {
  // Landschaftsplatte um das Stellwerk
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const grass = new THREE.Mesh(new THREE.BoxGeometry(180, 3, 130), grassMat);
  grass.position.set(10, 1.5, 0);
  group.add(grass);

  // Weg zur Treppe
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.8 });
  const path = new THREE.Mesh(new THREE.BoxGeometry(35, 1, 80), pathMat);
  path.position.set(65, 3.2, 25);
  group.add(path);

  // Büsche & Grasbüschel
  const bushMat = new THREE.MeshStandardMaterial({ color: 0x166534, roughness: 0.9 });
  const bush1 = new THREE.Mesh(new THREE.DodecahedronGeometry(9, 1), bushMat);
  bush1.position.set(-45, 9, 35);
  group.add(bush1);

  // Untergeschoss (Klinker)
  const brickMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.85 });
  const groundFloor = new THREE.Mesh(new THREE.BoxGeometry(100, 45, 65), brickMat);
  groundFloor.position.set(0, 24, 0);
  group.add(groundFloor);

  // Obergeschoss (Ausstellend mit großen Fenstern)
  const topWallMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.6 });
  const topFloor = new THREE.Mesh(new THREE.BoxGeometry(112, 35, 75), topWallMat);
  topFloor.position.set(0, 64, 0);
  group.add(topFloor);

  // Panorama-Fensterband
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.15, metalness: 0.85 });
  const frontGlass = new THREE.Mesh(new THREE.BoxGeometry(106, 20, 1), glassMat);
  frontGlass.position.set(0, 67.5, 38);
  group.add(frontGlass);

  const backGlass = new THREE.Mesh(new THREE.BoxGeometry(106, 20, 1), glassMat);
  backGlass.position.set(0, 67.5, -38);
  group.add(backGlass);

  // Flachdach mit Dachrand
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.7 });
  const roof = new THREE.Mesh(new THREE.BoxGeometry(122, 5, 85), roofMat);
  roof.position.set(0, 84, 0);
  group.add(roof);

  // Außentreppe aus Stahl
  const stairMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.5, metalness: 0.6 });
  const stairs = new THREE.Mesh(new THREE.BoxGeometry(15, 45, 25), stairMat);
  stairs.position.set(58, 24, 0);
  stairs.rotation.z = -0.55;
  group.add(stairs);
}

/** 3. LOKSCHUPPEN (250 x 130 x 105 mm) MIT LANDSCHAFT, WEG & WALD */
function buildEngineShed3D(group: THREE.Group) {
  // A. Landschaftliche Grundfläche um den Lokschuppen (Wiese & Betriebshof)
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const grassBase = new THREE.Mesh(new THREE.BoxGeometry(390, 3, 260), grassMat);
  grassBase.position.set(15, 1.5, 0);
  group.add(grassBase);

  // Schotter- & Sandbereich vor dem Tor (Gleiszufahrt)
  const yardMat = new THREE.MeshStandardMaterial({ color: 0x52525b, roughness: 0.95 });
  const gateYard = new THREE.Mesh(new THREE.BoxGeometry(60, 1, 90), yardMat);
  gateYard.position.set(-145, 3.2, 0);
  group.add(gateYard);

  // B. Der Weg zum Gebäude (Geschwungener Kies- & Pflasterweg zum Seiteneingang)
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.75 });
  const curbMat = new THREE.MeshStandardMaterial({ color: 0x78716c, roughness: 0.8 });

  // Wegabschnitt 1 (von der Grundstückskante nach innen)
  const pathPart1 = new THREE.Mesh(new THREE.BoxGeometry(36, 1.2, 60), pathMat);
  pathPart1.position.set(-75, 3.2, 85);
  group.add(pathPart1);

  // Wegabschnitt 2 (Bogen zur Seitenwand)
  const pathPart2 = new THREE.Mesh(new THREE.BoxGeometry(100, 1.2, 34), pathMat);
  pathPart2.position.set(-10, 3.2, 65);
  group.add(pathPart2);

  // Randsteine entlang des Weges
  const curb1 = new THREE.Mesh(new THREE.BoxGeometry(2, 2.5, 60), curbMat);
  curb1.position.set(-94, 3.5, 85);
  group.add(curb1);

  const curb2 = new THREE.Mesh(new THREE.BoxGeometry(100, 2.5, 2), curbMat);
  curb2.position.set(-10, 3.5, 83);
  group.add(curb2);

  // Granit-Eingangsstufe vor der Personaltür
  const stepMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.7 });
  const step = new THREE.Mesh(new THREE.BoxGeometry(26, 4, 14), stepMat);
  step.position.set(20, 3.5, 64);
  group.add(step);

  // Seitentür / Personaleingang im Gebäude
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
  const sideDoor = new THREE.Mesh(new THREE.BoxGeometry(20, 38, 2), doorMat);
  sideDoor.position.set(20, 24, 61);
  group.add(sideDoor);

  const doorFrame = new THREE.Mesh(new THREE.BoxGeometry(24, 42, 1), new THREE.MeshStandardMaterial({ color: 0x27272a }));
  doorFrame.position.set(20, 25, 60.5);
  group.add(doorFrame);

  // Hoflaterne am Weg (mit warmem Lichtschein)
  const lampPoleMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.35, metalness: 0.8 });
  const lampPole = new THREE.Mesh(new THREE.CylinderGeometry(1.8, 2.2, 55, 8), lampPoleMat);
  lampPole.position.set(-45, 29, 82);
  group.add(lampPole);

  const lampHead = new THREE.Mesh(new THREE.ConeGeometry(6, 4, 12), lampPoleMat);
  lampHead.position.set(-45, 57, 82);
  group.add(lampHead);

  const bulb = new THREE.Mesh(
    new THREE.SphereGeometry(2.5, 8, 8),
    new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.95 }),
  );
  bulb.position.set(-45, 54, 82);
  group.add(bulb);

  // C. Wald & Bepflanzung um den Lokschuppen herum (Fichten, Kiefern & Birken mit sicherem Abstand)
  const bushMatDark = new THREE.MeshStandardMaterial({ color: 0x14532d, roughness: 0.92 });
  const bushMatLight = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const barkMat = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });

  // Büsche entlang der Rückwand und Seiten (ohne in die Wand zu ragen)
  const bushPositions = [
    { x: 145, z: 20, r: 10 },
    { x: 148, z: -25, r: 11 },
    { x: 145, z: -60, r: 10 },
    { x: 80, z: -88, r: 10 },
    { x: 20, z: -88, r: 9 },
    { x: -50, z: -88, r: 10 },
    { x: -65, z: 82, r: 9 },
  ];
  for (const b of bushPositions) {
    const bush = new THREE.Mesh(new THREE.DodecahedronGeometry(b.r, 1), b.r > 10 ? bushMatDark : bushMatLight);
    bush.position.set(b.x, b.r * 0.8 + 2, b.z);
    group.add(bush);
  }

  // Waldhain um den Lokschuppen (mit ausreichend Abstand zum Baukörper)
  const treeList = [
    { x: 172, z: 88, type: "pine", scale: 1.15 },
    { x: 178, z: 35, type: "pine", scale: 1.25 },
    { x: 172, z: -40, type: "pine", scale: 1.05 },
    { x: 162, z: -98, type: "pine", scale: 0.95 },
    { x: 110, z: 112, type: "birch", scale: 1.1 },
    { x: 38, z: -108, type: "birch", scale: 1.0 },
  ];

  for (const t of treeList) {
    if (t.type === "pine") {
      const trunk = new THREE.Mesh(new THREE.CylinderGeometry(3 * t.scale, 5 * t.scale, 30 * t.scale, 8), barkMat);
      trunk.position.set(t.x, 15 * t.scale, t.z);
      group.add(trunk);

      const c1 = new THREE.Mesh(new THREE.ConeGeometry(24 * t.scale, 32 * t.scale, 8), bushMatDark);
      c1.position.set(t.x, 36 * t.scale, t.z);
      group.add(c1);

      const c2 = new THREE.Mesh(new THREE.ConeGeometry(18 * t.scale, 26 * t.scale, 8), bushMatDark);
      c2.position.set(t.x, 52 * t.scale, t.z);
      group.add(c2);
    } else {
      const trunk = new THREE.Mesh(new THREE.CylinderGeometry(3 * t.scale, 4.5 * t.scale, 35 * t.scale, 8), new THREE.MeshStandardMaterial({ color: 0xe2e8f0, roughness: 0.7 }));
      trunk.position.set(t.x, 17.5 * t.scale, t.z);
      group.add(trunk);

      const crown = new THREE.Mesh(new THREE.DodecahedronGeometry(22 * t.scale, 1), bushMatLight);
      crown.position.set(t.x, 48 * t.scale, t.z);
      group.add(crown);
    }
  }

  // Rustikaler Holzzaun an der Waldgrenze
  const fenceWoodMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 });
  const railTop = new THREE.Mesh(new THREE.BoxGeometry(200, 2.5, 2), fenceWoodMat);
  railTop.position.set(65, 16, 122);
  group.add(railTop);

  const railBot = new THREE.Mesh(new THREE.BoxGeometry(200, 2.5, 2), fenceWoodMat);
  railBot.position.set(65, 8, 122);
  group.add(railBot);

  // Zaunpfosten alle 35 mm
  for (let zx = -30; zx <= 160; zx += 35) {
    const post = new THREE.Mesh(new THREE.BoxGeometry(4, 20, 4), fenceWoodMat);
    post.position.set(zx, 11, 122);
    group.add(post);
  }

  // D. Hauptgebäude des Lokschuppens (Backstein / Klinker)
  const wallMat = new THREE.MeshStandardMaterial({ color: 0x7c2d12, roughness: 0.85 });
  const shed = new THREE.Mesh(new THREE.BoxGeometry(240, 65, 120), wallMat);
  shed.position.set(0, 34, 0);
  group.add(shed);

  // Torgewölbe (schwarz / geöffnet)
  const portalMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.9 });
  const portal = new THREE.Mesh(new THREE.BoxGeometry(2, 50, 48), portalMat);
  portal.position.set(-121, 26.5, 0);
  group.add(portal);

  // Torgewölbebogen oben
  const arch = new THREE.Mesh(new THREE.CylinderGeometry(24, 24, 2, 16, 1, false, 0, Math.PI), portalMat);
  arch.rotateZ(Math.PI / 2);
  arch.rotateY(Math.PI / 2);
  arch.position.set(-121, 51.5, 0);
  group.add(arch);

  // Runddach / Tonnendach
  const roofGeo = new THREE.CylinderGeometry(68, 68, 246, 24, 1, false, 0, Math.PI);
  roofGeo.rotateZ(Math.PI / 2);
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.65 });
  const roof = new THREE.Mesh(roofGeo, roofMat);
  roof.position.set(0, 66.5, 0);
  group.add(roof);

  // Schornstein / Rauchabzug
  const chimMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.5 });
  const chimney = new THREE.Mesh(new THREE.CylinderGeometry(6, 7, 30, 12), chimMat);
  chimney.position.set(50, 96.5, 0);
  group.add(chimney);
}

/** 4. FACHWERKHAUS (110 x 90 x 125 mm) */
function buildTownHouse3D(group: THREE.Group) {
  // Landschaftsplatte mit Garten & Weg
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const garden = new THREE.Mesh(new THREE.BoxGeometry(160, 3, 140), grassMat);
  garden.position.set(0, 1.5, 10);
  group.add(garden);

  // Kiesweg zur Haustür
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.8 });
  const walk = new THREE.Mesh(new THREE.BoxGeometry(28, 1, 45), pathMat);
  walk.position.set(0, 3.2, 55);
  group.add(walk);

  // Haustür
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.8 });
  const door = new THREE.Mesh(new THREE.BoxGeometry(16, 26, 1), doorMat);
  door.position.set(0, 15, 41);
  group.add(door);

  // Gartenzaun
  const fenceMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, roughness: 0.7 });
  for (let fx = -70; fx <= 70; fx += 16) {
    if (Math.abs(fx) < 18) continue; // Pforte frei
    const picket = new THREE.Mesh(new THREE.BoxGeometry(2.5, 16, 2), fenceMat);
    picket.position.set(fx, 9, 78);
    group.add(picket);
  }

  // Erdgeschoss verputzt
  const baseMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, roughness: 0.8 });
  const ground = new THREE.Mesh(new THREE.BoxGeometry(100, 40, 80), baseMat);
  ground.position.set(0, 21.5, 0);
  group.add(ground);

  // Obergeschoss Fachwerk
  const upper = new THREE.Mesh(new THREE.BoxGeometry(105, 35, 85), baseMat);
  upper.position.set(0, 59, 0);
  group.add(upper);

  // Dunkelbraune Holzbalken
  const beamMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.9 });
  const beamH1 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 86), beamMat);
  beamH1.position.set(0, 41.5, 0);
  group.add(beamH1);

  const beamH2 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 86), beamMat);
  beamH2.position.set(0, 76.5, 0);
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

