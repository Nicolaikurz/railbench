import * as THREE from "three";

/**
 * 3. LOKSCHUPPEN (240 x 72 x 105 mm)
 * Einständiger Lokschuppen für Dampf- und Diesellokomotiven.
 *
 * Profilbreite 72 mm (Z-Bereich [-36, +36] mm) garantiert vollständige
 * Lichtraumprofil-Freiheit (NEM 102 >= 24 mm) zu benachbarten Parallelgleisen
 * (z. B. Standardabstand 64.3 mm oder 61.6 mm), sodass Vorbeifahrten ohne
 * Kollision möglich sind.
 *
 * Ausgestattet mit weit geöffneten zweiflügeligen Toren (open doors)
 * und detailliertem Innenraum / Inspektionsgrube.
 */
export function buildEngineShed3D(group: THREE.Group): void {
  // Material-Definitionen
  const brickMat = new THREE.MeshStandardMaterial({ color: 0x852e16, roughness: 0.85 });
  const darkInteriorMat = new THREE.MeshStandardMaterial({ color: 0x27272a, roughness: 0.95 });
  const trimMat = new THREE.MeshStandardMaterial({ color: 0xc4b5a5, roughness: 0.8 });
  const foundationMat = new THREE.MeshStandardMaterial({ color: 0x52525b, roughness: 0.9 });
  const pitMat = new THREE.MeshStandardMaterial({ color: 0x18181b, roughness: 0.95 });
  const railMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.35, metalness: 0.8 });
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.65 });
  const skylightGlassMat = new THREE.MeshStandardMaterial({
    color: 0x93c5fd,
    roughness: 0.2,
    metalness: 0.1,
    transparent: true,
    opacity: 0.55,
  });
  const windowGlassMat = new THREE.MeshStandardMaterial({
    color: 0x7dd3fc,
    roughness: 0.3,
    metalness: 0.1,
    transparent: true,
    opacity: 0.6,
  });
  const windowFrameMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.7 });
  const doorWoodMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 });
  const ironMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.6, metalness: 0.5 });
  const lampMat = new THREE.MeshStandardMaterial({
    color: 0xfef08a,
    emissive: 0xfef08a,
    emissiveIntensity: 0.9,
  });

  // 1. FUNDAMENT & BODENPLATTE (Innerhalb Z: [-36, +36] mm)
  const baseW = 242;
  const baseD = 72;
  const baseH = 3;
  const foundation = new THREE.Mesh(new THREE.BoxGeometry(baseW, baseH, baseD), foundationMat);
  foundation.position.set(0, baseH / 2, 0);
  group.add(foundation);

  // Inspektionsgrube mittig im Schuppenboden (Schotter & Arbeitsgrube)
  const pit = new THREE.Mesh(new THREE.BoxGeometry(190, 1.2, 20), pitMat);
  pit.position.set(10, baseH + 0.1, 0);
  group.add(pit);

  // Gleisstränge im Innenraum
  const railL = new THREE.Mesh(new THREE.BoxGeometry(220, 1.8, 1.5), railMat);
  railL.position.set(-10, baseH + 0.9, -8.25);
  group.add(railL);

  const railR = new THREE.Mesh(new THREE.BoxGeometry(220, 1.8, 1.5), railMat);
  railR.position.set(-10, baseH + 0.9, 8.25);
  group.add(railR);

  // Prellbock am Hallenende (X = 95)
  const bufferMat = new THREE.MeshStandardMaterial({ color: 0xb91c1c, roughness: 0.7 });
  const bufferBeam = new THREE.Mesh(new THREE.BoxGeometry(4, 6, 22), bufferMat);
  bufferBeam.position.set(95, baseH + 7, 0);
  group.add(bufferBeam);
  const bufferPostL = new THREE.Mesh(new THREE.BoxGeometry(3, 10, 3), ironMat);
  bufferPostL.position.set(95, baseH + 5, -8);
  group.add(bufferPostL);
  const bufferPostR = new THREE.Mesh(new THREE.BoxGeometry(3, 10, 3), ironMat);
  bufferPostR.position.set(95, baseH + 5, 8);
  group.add(bufferPostR);

  // 2. MAUERWERK (Hallenlänge 240 mm, Wandstärke 3 mm, Breite 70 mm [-35, +35])
  // Wandhöhe auf 97 mm erhöht, damit Oberleitung (Fahrdraht Y=72, Tragseil Y=85) und Stromabnehmer frei einfahren können
  const wallH = 97; // von Y=3 bis Y=100 mm
  const wallY = baseH + wallH / 2;

  // Linke Seitenwand (Z = -34.5 mm)
  const sideWallL = new THREE.Mesh(new THREE.BoxGeometry(240, wallH, 3), brickMat);
  sideWallL.position.set(0, wallY, -34);
  group.add(sideWallL);

  // Rechte Seitenwand (Z = +34.5 mm)
  const sideWallR = new THREE.Mesh(new THREE.BoxGeometry(240, wallH, 3), brickMat);
  sideWallR.position.set(0, wallY, 34);
  group.add(sideWallR);

  // Seitenfenster (je 4 große Rundbogenfenster auf beiden Seiten, proportional angepasst)
  const winStep = 45;
  for (let i = -1.5; i <= 1.5; i++) {
    const wx = i * winStep;
    for (const zSide of [-34, 34]) {
      const winFrame = new THREE.Mesh(new THREE.BoxGeometry(24, 48, 3.6), windowFrameMat);
      winFrame.position.set(wx, baseH + 52, zSide);
      group.add(winFrame);

      const winGlass = new THREE.Mesh(new THREE.BoxGeometry(21, 45, 3.8), windowGlassMat);
      winGlass.position.set(wx, baseH + 52, zSide);
      group.add(winGlass);

      // Sandstein-Fensterbank
      const sill = new THREE.Mesh(new THREE.BoxGeometry(27, 2.5, 4.2), trimMat);
      sill.position.set(wx, baseH + 27, zSide);
      group.add(sill);
    }
  }

  // Rückwand (X = +120 mm)
  const rearWall = new THREE.Mesh(new THREE.BoxGeometry(3, wallH, 70), brickMat);
  rearWall.position.set(118.5, wallY, 0);
  group.add(rearWall);

  // Rückwand-Giebel (Dreieck von Y=100 bis Y=124)
  const gableShape = new THREE.Shape();
  gableShape.moveTo(-35, 100);
  gableShape.lineTo(0, 124);
  gableShape.lineTo(35, 100);
  gableShape.closePath();

  const gableGeo = new THREE.ExtrudeGeometry(gableShape, { depth: 3, bevelEnabled: false });
  gableGeo.rotateY(Math.PI / 2);
  const rearGable = new THREE.Mesh(gableGeo, brickMat);
  rearGable.position.set(120, 0, 0);
  group.add(rearGable);

  // 3. FRONTPORTAL & WEIT GEÖFFNETE TORE (X = -120 mm)
  // Durchfahrtsöffnung: Breite 44 mm (Z von -22 bis +22), Höhe 89 mm (Y von 3 bis 92)
  // Garantiert volle Durchfahrtshöhe für Oberleitung (Fahrdraht 72 mm, Tragseil 85 mm)
  const portalW = 44;
  const portalH = 89;
  const pillarW = (70 - portalW) / 2; // 13 mm Pfeilerbreite links & rechts

  // Linker Portalpfeiler (Z von -35 bis -22)
  const pillarL = new THREE.Mesh(new THREE.BoxGeometry(3.5, wallH, pillarW), brickMat);
  pillarL.position.set(-118.5, wallY, -35 + pillarW / 2);
  group.add(pillarL);

  // Rechter Portalpfeiler (Z von +22 bis +35)
  const pillarR = new THREE.Mesh(new THREE.BoxGeometry(3.5, wallH, pillarW), brickMat);
  pillarR.position.set(-118.5, wallY, 35 - pillarW / 2);
  group.add(pillarR);

  // Sturz / Sandsteinbogen über dem Tor (Y von 92 bis 100)
  const lintelH = wallH - portalH;
  const lintel = new THREE.Mesh(new THREE.BoxGeometry(4, lintelH, 70), trimMat);
  lintel.position.set(-118.5, baseH + portalH + lintelH / 2, 0);
  group.add(lintel);

  // Front-Giebel über dem Tor
  const frontGable = new THREE.Mesh(gableGeo, brickMat);
  frontGable.position.set(-117, 0, 0);
  group.add(frontGable);

  // Laterne über dem Torbogen
  const lampHolder = new THREE.Mesh(new THREE.BoxGeometry(4, 2, 2), ironMat);
  lampHolder.position.set(-121, baseH + portalH + 5, 0);
  group.add(lampHolder);

  const lampBulb = new THREE.Mesh(new THREE.SphereGeometry(2.5, 8, 8), lampMat);
  lampBulb.position.set(-122.5, baseH + portalH + 4, 0);
  group.add(lampBulb);

  // ========================================================
  // 4. WEIT GEÖFFNETE TORFLÜGEL (Double-leaf open doors)
  // ========================================================
  // Jeder Flügel hat ca. 22 mm Breite, 87.5 mm Höhe, 2 mm Stärke
  const leafW = portalW / 2;
  const leafH = portalH - 1.5;
  const leafThick = 2.0;

  // Linker Torflügel:
  // Scharnier am linken Pfosten: X = -120, Z = -22.
  // Weit nach außen aufgeschwenkt (+80° bzw. +1.40 rad) flach an den Pfeiler
  const hingeL = new THREE.Group();
  hingeL.position.set(-120, baseH + leafH / 2, -portalW / 2);
  hingeL.rotation.y = 1.4; // Weit geöffnet nach außen

  // Holztorblatt
  const doorMeshL = new THREE.Mesh(new THREE.BoxGeometry(leafThick, leafH, leafW), doorWoodMat);
  doorMeshL.position.set(0, 0, leafW / 2);
  hingeL.add(doorMeshL);

  // Eisenbeschläge / Langbänder (3 Bänder für hohe Werkstatttore)
  for (const hy of [-leafH * 0.38, 0, leafH * 0.38]) {
    const hingeStrap = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.6, 2.5, leafW * 0.85), ironMat);
    hingeStrap.position.set(0, hy, leafW * 0.45);
    hingeL.add(hingeStrap);
  }

  // Diagonale Holzverstrebung
  const braceL = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.4, 2.2, leafW * 0.95), doorWoodMat);
  braceL.rotation.x = 0.55;
  braceL.position.set(0, 0, leafW / 2);
  hingeL.add(braceL);

  // Fenster im oberen Torbereich
  const doorWinL = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.8, 16, 8), windowGlassMat);
  doorWinL.position.set(0, leafH * 0.28, leafW * 0.5);
  hingeL.add(doorWinL);

  group.add(hingeL);

  // Rechter Torflügel:
  // Scharnier am rechten Pfosten: X = -120, Z = +22.
  // Weit nach außen aufgeschwenkt (-80° bzw. -1.40 rad) flach an den Pfeiler
  const hingeR = new THREE.Group();
  hingeR.position.set(-120, baseH + leafH / 2, portalW / 2);
  hingeR.rotation.y = -1.4; // Weit geöffnet nach außen

  const doorMeshR = new THREE.Mesh(new THREE.BoxGeometry(leafThick, leafH, leafW), doorWoodMat);
  doorMeshR.position.set(0, 0, -leafW / 2);
  hingeR.add(doorMeshR);

  for (const hy of [-leafH * 0.38, 0, leafH * 0.38]) {
    const hingeStrap = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.6, 2.5, leafW * 0.85), ironMat);
    hingeStrap.position.set(0, hy, -leafW * 0.45);
    hingeR.add(hingeStrap);
  }

  const braceR = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.4, 2.2, leafW * 0.95), doorWoodMat);
  braceR.rotation.x = -0.55;
  braceR.position.set(0, 0, -leafW / 2);
  hingeR.add(braceR);

  const doorWinR = new THREE.Mesh(new THREE.BoxGeometry(leafThick + 0.8, 16, 8), windowGlassMat);
  doorWinR.position.set(0, leafH * 0.28, -leafW * 0.5);
  hingeR.add(doorWinR);

  group.add(hingeR);

  // 5. SATTELDACH MIT FIRST, GLAS-OBERLICHT & RAUCHABZÜGEN
  // Exakt extrudiertes Satteldachprofil (Traufe Y = 99.5 mm, Firsthöhe Y = 124 mm)
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-36.5, 99.5);
  roofShape.lineTo(0, 124.0);
  roofShape.lineTo(36.5, 99.5);
  roofShape.lineTo(36.5, 97.5);
  roofShape.lineTo(0, 121.5);
  roofShape.lineTo(-36.5, 97.5);
  roofShape.closePath();

  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 244, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-122, 0, 0);

  const roofMesh = new THREE.Mesh(roofGeo, roofMat);
  roofMesh.castShadow = true;
  roofMesh.receiveShadow = true;
  group.add(roofMesh);

  // Firstbalken entlang des Firstes
  const ridgeBeam = new THREE.Mesh(new THREE.BoxGeometry(244, 2.5, 3), trimMat);
  ridgeBeam.position.set(0, 124.5, 0);
  group.add(ridgeBeam);

  // Dach-Oberlicht (Skylight lantern strip mittig auf dem Dach)
  const skyLen = 150;
  const skylight = new THREE.Mesh(new THREE.BoxGeometry(skyLen, 5, 12), skylightGlassMat);
  skylight.position.set(0, 126.5, 0);
  group.add(skylight);

  const skylightRidge = new THREE.Mesh(new THREE.BoxGeometry(skyLen + 2, 2, 2), ironMat);
  skylightRidge.position.set(0, 129.0, 0);
  group.add(skylightRidge);

  // 3 Rauchabzugshauben (Dampflok-Schornsteinhauben) auf dem Dach
  for (const cx of [-75, 0, 75]) {
    const ventBase = new THREE.Mesh(new THREE.CylinderGeometry(4.5, 5.5, 12, 12), ironMat);
    ventBase.position.set(cx, 130, 0);
    group.add(ventBase);

    const ventCap = new THREE.Mesh(new THREE.ConeGeometry(7, 4, 12), ironMat);
    ventCap.position.set(cx, 137, 0);
    group.add(ventCap);
  }
}
