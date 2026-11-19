import * as THREE from "three";

/**
 * 7. PRELLBOCK (DB-Bauart / Rawie Stahl-Schienenprellbock, ~28 x 30 x 25 mm im Maßstab H0)
 * - Offenes Stahl-A-Gestell exakt auf Spurweite (±8,25 mm) mit Schienenklammern & Querstreben
 * - Massive Holz-Pufferbohle mit Stahlbeschlägen
 * - Zwei realistische Hülsenpuffer mit Puffertellern
 * - Authentisches DB-Sh2-Schutzhaltsignal (rot mit weißem Rand) auf kleinem Signalmast
 */
export function buildBufferStop3D(group: THREE.Group): void {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.55, metalness: 0.65 });
  const darkSteelMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.45, metalness: 0.75 });
  const rustSteelMat = new THREE.MeshStandardMaterial({ color: 0x4b3832, roughness: 0.8, metalness: 0.35 });
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x6b3e1e, roughness: 0.88, metalness: 0.05 });
  const whiteMat = new THREE.MeshStandardMaterial({ color: 0xf8fafc, roughness: 0.4, metalness: 0.05 });
  const redSignalMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.45, metalness: 0.1 });

  const railHalf = 8.25; // H0-Spurweite 16,5 mm -> ±8,25 mm exakt auf den Schienenköpfen
  const beamY = 12.6;
  const beamZ = -8.0;

  // Hilfsfunktion für geneigte Stahlträger (Schienenprofil) zwischen 2 Punkten auf x = railX
  const addStrut = (
    railX: number,
    y1: number,
    z1: number,
    y2: number,
    z2: number,
    width = 2.0,
    depth = 2.2,
    mat: THREE.Material = steelMat,
  ) => {
    const dy = y2 - y1;
    const dz = z2 - z1;
    const len = Math.hypot(dy, dz);
    const strut = new THREE.Mesh(new THREE.BoxGeometry(width, len, depth), mat);
    strut.position.set(railX, (y1 + y2) / 2, (z1 + z2) / 2);
    strut.rotation.x = Math.atan2(dz, dy);
    group.add(strut);
  };

  // 1. Linkes & rechtes offenes Stahl-A-Gestell mit Schienenfuß-Klammern
  for (const side of [-1, 1]) {
    const rx = side * railHalf;

    // Grundschuh / Gleisauflage
    const shoe = new THREE.Mesh(new THREE.BoxGeometry(3.2, 1.8, 25.5), darkSteelMat);
    shoe.position.set(rx, 0.9, 1.2);
    group.add(shoe);

    // Schienenklammern vorn & hinten
    for (const clampZ of [-10.0, 2.0, 11.8]) {
      const clamp = new THREE.Mesh(new THREE.BoxGeometry(4.2, 2.2, 2.4), rustSteelMat);
      clamp.position.set(rx, 1.1, clampZ);
      group.add(clamp);
    }

    // Vordere leicht geneigte Stütze
    addStrut(rx, 1.4, -10.5, beamY + 1.2, beamZ + 0.5, 2.1, 2.3, steelMat);

    // Hintere schräge Druckstrebe (klassisches offenes Dreiecks-Fachwerk)
    addStrut(rx, beamY + 0.8, beamZ + 1.2, 1.4, 12.5, 2.1, 2.3, steelMat);

    // Mittlere Versteifungsstrebe im Dreieck
    addStrut(rx, 6.2, -8.8, 6.2, 4.2, 1.5, 1.6, darkSteelMat);
  }

  // 2. Horizontale Stahl-Querriegel zwischen linkem und rechtem Gestell
  const rearTieBar = new THREE.Mesh(new THREE.BoxGeometry(railHalf * 2 + 2.4, 2.0, 2.2), darkSteelMat);
  rearTieBar.position.set(0, 2.2, 10.5);
  group.add(rearTieBar);

  const upperBackingBar = new THREE.Mesh(new THREE.BoxGeometry(railHalf * 2 + 4.0, 3.2, 2.4), steelMat);
  upperBackingBar.position.set(0, beamY, beamZ + 2.4);
  group.add(upperBackingBar);

  // 3. Massive Holz-Pufferbohle mit Stahl-Endkappen
  const bufferPlank = new THREE.Mesh(new THREE.BoxGeometry(27.5, 4.8, 3.8), woodMat);
  bufferPlank.position.set(0, beamY, beamZ - 0.5);
  group.add(bufferPlank);

  for (const side of [-1, 1]) {
    const cap = new THREE.Mesh(new THREE.BoxGeometry(1.4, 5.2, 4.2), darkSteelMat);
    cap.position.set(side * 13.4, beamY, beamZ - 0.5);
    group.add(cap);
  }

  // 4. Zwei realistische Hülsenpuffer mit Puffertellern (Abstand ±9,8 mm)
  for (const side of [-1, 1]) {
    const px = side * 9.8;

    // Puffer-Grundplatte
    const plate = new THREE.Mesh(new THREE.BoxGeometry(4.2, 4.2, 0.8), darkSteelMat);
    plate.position.set(px, beamY, beamZ - 2.6);
    group.add(plate);

    // Pufferhülse
    const sleeve = new THREE.Mesh(new THREE.CylinderGeometry(1.45, 1.85, 2.8, 10), steelMat);
    sleeve.rotation.x = Math.PI / 2;
    sleeve.position.set(px, beamY, beamZ - 4.2);
    group.add(sleeve);

    // Pufferteller
    const head = new THREE.Mesh(new THREE.CylinderGeometry(2.5, 2.5, 0.75, 14), darkSteelMat);
    head.rotation.x = Math.PI / 2;
    head.position.set(px, beamY, beamZ - 5.8);
    group.add(head);
  }

  // 5. DB-Schutzhaltsignal (Sh 2: rote Tafel mit weißem Rand) auf kurzem Signalpfosten
  const post = new THREE.Mesh(new THREE.BoxGeometry(1.3, 5.6, 1.3), darkSteelMat);
  post.position.set(0, beamY + 4.8, beamZ - 0.2);
  group.add(post);

  // Schwarzes Signalgehäuse / Rückseite
  const signBack = new THREE.Mesh(new THREE.BoxGeometry(8.4, 6.2, 1.4), darkSteelMat);
  signBack.position.set(0, beamY + 8.6, beamZ - 0.3);
  group.add(signBack);

  // Weißer Signalrand
  const signBorder = new THREE.Mesh(new THREE.BoxGeometry(7.8, 5.6, 0.3), whiteMat);
  signBorder.position.set(0, beamY + 8.6, beamZ - 1.05);
  group.add(signBorder);

  // Rotes Sh-2-Signalfeld (leuchtet bei beleuchteten Prellböcken wie Märklin 24978)
  const signRed = new THREE.Mesh(new THREE.BoxGeometry(6.0, 3.8, 0.35), redSignalMat);
  signRed.position.set(0, beamY + 8.6, beamZ - 1.15);
  signRed.userData.isBufferSignal = true;
  group.add(signRed);
}
