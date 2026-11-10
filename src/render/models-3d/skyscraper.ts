import * as THREE from "three";

/**
 * Moderner Wolkenkratzer (Skyscraper)
 * Abmessungen BxTxH: 130 x 110 x 280 mm
 *
 * Merkmale:
 * - Granit-Vorplatz mit Eingangsvordach & Pflanzkübeln
 * - Untere Eingangsebene mit Glasportal & Säulen
 * - Hauptturm mit Glasfassade & metallenen Fassadenstreben
 * - Gestufter oberer Turmabschnitt mit Dachterrasse
 * - Technikaufbauten (Klimaanlagen, Lüftungsgitter)
 * - Funk- & Antennenmast mit rotem Flugsicherungsfeuer auf der Spitze
 */
export function buildSkyscraper3D(group: THREE.Group): void {
  // Materialien
  const plazaMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.8 });
  const curbMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.7 });
  const planterMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.85 });
  const bushMat = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  const metalTrimMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.35, metalness: 0.65 });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.1, metalness: 0.9 });
  const darkGlassMat = new THREE.MeshStandardMaterial({ color: 0x0369a1, roughness: 0.15, metalness: 0.85 });
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.75 });
  const beaconMat = new THREE.MeshStandardMaterial({ color: 0xef4444, emissive: 0xef4444, emissiveIntensity: 0.95 });
  const warmLightMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.7 });

  // 1. Vorplatz & Fundament
  const basePlaza = new THREE.Mesh(new THREE.BoxGeometry(150, 4, 130), plazaMat);
  basePlaza.position.set(0, 2, 0);
  group.add(basePlaza);

  // Randsteine & Treppenstufe
  const step = new THREE.Mesh(new THREE.BoxGeometry(70, 2, 16), curbMat);
  step.position.set(0, 3, 58);
  group.add(step);

  // Pflanzkübel mit Zierbüschen vor dem Portal
  for (const px of [-45, 45]) {
    const planter = new THREE.Mesh(new THREE.BoxGeometry(16, 6, 16), planterMat);
    planter.position.set(px, 5, 52);
    group.add(planter);

    const bush = new THREE.Mesh(new THREE.DodecahedronGeometry(7, 1), bushMat);
    bush.position.set(px, 12, 52);
    group.add(bush);
  }

  // 2. Erdgeschoss / Lobby (Hohe Decke mit Kolonnaden)
  const lobbyHeight = 24;
  const lobbyBase = new THREE.Mesh(new THREE.BoxGeometry(110, lobbyHeight, 90), glassMat);
  lobbyBase.position.set(0, 4 + lobbyHeight / 2, 0);
  group.add(lobbyBase);

  // Vordach über Haupteingang
  const canopy = new THREE.Mesh(new THREE.BoxGeometry(54, 2.5, 24), steelMat);
  canopy.position.set(0, 4 + 18, 52);
  group.add(canopy);

  // Beleuchtetes Eingangsschild
  const sign = new THREE.Mesh(new THREE.BoxGeometry(40, 4, 1.5), warmLightMat);
  sign.position.set(0, 4 + 21, 53);
  group.add(sign);

  // Pfeiler an der Fassade (Erdgeschoss)
  for (const cx of [-55, -28, 0, 28, 55]) {
    const col = new THREE.Mesh(new THREE.BoxGeometry(4, lobbyHeight, 4), steelMat);
    col.position.set(cx, 4 + lobbyHeight / 2, 45);
    group.add(col);
  }

  // Zwischengesims / Zwischendecke
  const floorSlab1 = new THREE.Mesh(new THREE.BoxGeometry(114, 3, 94), steelMat);
  floorSlab1.position.set(0, 4 + lobbyHeight + 1.5, 0);
  group.add(floorSlab1);

  // 3. Hauptturm (Sektion 1: Etagen 2 bis 14)
  const tower1Height = 140;
  const tower1Y = 4 + lobbyHeight + 3 + tower1Height / 2;

  // Glas-Baukörper
  const tower1Core = new THREE.Mesh(new THREE.BoxGeometry(108, tower1Height, 88), glassMat);
  tower1Core.position.set(0, tower1Y, 0);
  group.add(tower1Core);

  // Horizontale Geschoss-Bänder (Spandrels)
  const numFloors1 = 10;
  for (let f = 1; f <= numFloors1; f++) {
    const fy = 4 + lobbyHeight + 3 + (tower1Height * f) / (numFloors1 + 1);
    const spandrel = new THREE.Mesh(new THREE.BoxGeometry(109, 2.2, 89), metalTrimMat);
    spandrel.position.set(0, fy, 0);
    group.add(spandrel);
  }

  // Vertikale Fassadenprofile / Mullions
  for (let mx = -48; mx <= 48; mx += 16) {
    const mullionFront = new THREE.Mesh(new THREE.BoxGeometry(1.6, tower1Height, 2), steelMat);
    mullionFront.position.set(mx, tower1Y, 44.5);
    group.add(mullionFront);

    const mullionBack = new THREE.Mesh(new THREE.BoxGeometry(1.6, tower1Height, 2), steelMat);
    mullionBack.position.set(mx, tower1Y, -44.5);
    group.add(mullionBack);
  }

  for (let mz = -36; mz <= 36; mz += 18) {
    const mullionLeft = new THREE.Mesh(new THREE.BoxGeometry(2, tower1Height, 1.6), steelMat);
    mullionLeft.position.set(-54.5, tower1Y, mz);
    group.add(mullionLeft);

    const mullionRight = new THREE.Mesh(new THREE.BoxGeometry(2, tower1Height, 1.6), steelMat);
    mullionRight.position.set(54.5, tower1Y, mz);
    group.add(mullionRight);
  }

  // 4. Gestufter oberer Turmabschnitt (Sektion 2 mit Setback / Rücksprung)
  const setbackY = 4 + lobbyHeight + 3 + tower1Height;
  const setbackSlab = new THREE.Mesh(new THREE.BoxGeometry(112, 3, 92), steelMat);
  setbackSlab.position.set(0, setbackY + 1.5, 0);
  group.add(setbackSlab);

  // Sicherheitsgeländer der Dachterrasse
  const railingMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.3, metalness: 0.8 });
  const railingL = new THREE.Mesh(new THREE.BoxGeometry(2, 6, 90), railingMat);
  railingL.position.set(-53, setbackY + 5, 0);
  group.add(railingL);
  const railingR = new THREE.Mesh(new THREE.BoxGeometry(2, 6, 90), railingMat);
  railingR.position.set(53, setbackY + 5, 0);
  group.add(railingR);

  const tower2Height = 65;
  const tower2Y = setbackY + 3 + tower2Height / 2;
  const tower2 = new THREE.Mesh(new THREE.BoxGeometry(84, tower2Height, 68), darkGlassMat);
  tower2.position.set(0, tower2Y, 0);
  group.add(tower2);

  // Horizontale Bänder Sektion 2
  for (let f = 1; f <= 5; f++) {
    const fy = setbackY + 3 + (tower2Height * f) / 6;
    const band = new THREE.Mesh(new THREE.BoxGeometry(85, 2, 69), metalTrimMat);
    band.position.set(0, fy, 0);
    group.add(band);
  }

  // 5. Dachabschluss & Technikaufbauten
  const roofTopY = setbackY + 3 + tower2Height;
  const roofBase = new THREE.Mesh(new THREE.BoxGeometry(86, 3, 70), steelMat);
  roofBase.position.set(0, roofTopY + 1.5, 0);
  group.add(roofBase);

  // Aufzugsschacht / Technikraum
  const mechRoom = new THREE.Mesh(new THREE.BoxGeometry(38, 16, 32), roofMat);
  mechRoom.position.set(-8, roofTopY + 10, 0);
  group.add(mechRoom);

  // Lüftungs- / Klimaanlagenblöcke
  const acMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.5, metalness: 0.6 });
  for (const acz of [-10, 10]) {
    const acUnit = new THREE.Mesh(new THREE.BoxGeometry(16, 8, 12), acMat);
    acUnit.position.set(22, roofTopY + 6, acz);
    group.add(acUnit);
  }

  // 6. Funkmast & Antennenspitze mit Flugwarnleuchte
  const mastHeight = 35;
  const mast = new THREE.Mesh(new THREE.CylinderGeometry(1.2, 2.5, mastHeight, 8), steelMat);
  mast.position.set(-8, roofTopY + 17 + mastHeight / 2, 0);
  group.add(mast);

  // Antennenspitze
  const needle = new THREE.Mesh(new THREE.CylinderGeometry(0.5, 1, 14, 8), metalTrimMat);
  needle.position.set(-8, roofTopY + 17 + mastHeight + 7, 0);
  group.add(needle);

  // Flugsicherungs-Warnleuchte (Beacon)
  const beacon = new THREE.Mesh(new THREE.SphereGeometry(2, 8, 8), beaconMat);
  beacon.position.set(-8, roofTopY + 17 + mastHeight + 14, 0);
  group.add(beacon);
}
