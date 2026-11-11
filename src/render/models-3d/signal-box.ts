import * as THREE from "three";

/** 2. STELLWERK (120 x 75 x 110 mm) */
export function buildSignalBox3D(group: THREE.Group): void {
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

  // --- Materialien für Türen & Treppe ---
  const stairMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.5, metalness: 0.6 });
  const frameMat = new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 0.6, metalness: 0.3 });
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });

  // --- Außentreppe an der Ostwand (x = 50), steigt Richtung -Z ---
  const steps = 14;
  const stairX = 55;
  const z0 = 24, z1 = -12;      // Start (unten) / Ende (oben)
  const y0 = 3, y1 = 46.5;      // Boden / Oberkante Podest = Unterkante Obergeschoss
  const run = z0 - z1;
  const rise = y1 - y0;
  const slope = Math.atan2(rise, run);
  const len = Math.hypot(run, rise);
  const midZ = (z0 + z1) / 2;
  const midY = (y0 + y1) / 2;

  // Wangen
  for (const dx of [-3.5, 3.5]) {
    const stringer = new THREE.Mesh(new THREE.BoxGeometry(1.5, 3, len), stairMat);
    stringer.rotation.x = slope;
    stringer.position.set(stairX + dx, midY, midZ);
    group.add(stringer);
  }

  // Stufen
  for (let i = 0; i < steps; i++) {
    const tread = new THREE.Mesh(new THREE.BoxGeometry(8, 1, 3.2), stairMat);
    tread.position.set(
      stairX,
      y0 + (rise * (i + 1)) / steps - 0.5,
      z0 - (run * (i + 0.5)) / steps
    );
    group.add(tread);
  }

  // Geländer an der Außenseite der Treppe
  const railX = stairX + 3.8;
  for (const i of [1, 4, 7, 10, 13]) {
    const post = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 9, 6), stairMat);
    post.position.set(
      railX,
      y0 + (rise * (i + 1)) / steps + 4,
      z0 - (run * (i + 0.5)) / steps
    );
    group.add(post);
  }
  const handrail = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, len), stairMat);
  handrail.rotation.x = slope;
  handrail.position.set(railX, midY + 10.5, midZ);
  group.add(handrail);

  // Podest vor der Obergeschoss-Tür mit Stützen
  const landing = new THREE.Mesh(new THREE.BoxGeometry(12, 1.5, 20), stairMat);
  landing.position.set(56, 45.75, -22);
  group.add(landing);

  for (const pz of [-31, -13]) {
    const support = new THREE.Mesh(new THREE.CylinderGeometry(1.2, 1.2, 42, 8), stairMat);
    support.position.set(61, 24, pz);
    group.add(support);
  }

  // Podest-Geländer (Ost- und Nordseite)
  for (const ry of [49, 53]) {
    const railE = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, 20), stairMat);
    railE.position.set(61.6, ry, -22);
    group.add(railE);
    const railN = new THREE.Mesh(new THREE.BoxGeometry(12, 0.8, 0.8), stairMat);
    railN.position.set(56, ry, -31.6);
    group.add(railN);
  }
  for (const [px, pz] of [[61.6, -31.6], [61.6, -22], [61.6, -12.5]] as const) {
    const post = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 7, 6), stairMat);
    post.position.set(px, 50, pz);
    group.add(post);
  }

  // --- Tür im Obergeschoss (Ostwand der Bedienetage, x = 56) ---
  const upperFrame = new THREE.Mesh(new THREE.BoxGeometry(1, 28, 15), frameMat);
  upperFrame.position.set(56.5, 60.5, -22);
  group.add(upperFrame);

  const upperDoor = new THREE.Mesh(new THREE.BoxGeometry(1, 26, 12), doorMat);
  upperDoor.position.set(57, 59.5, -22);
  group.add(upperDoor);

  const doorGlass = new THREE.Mesh(new THREE.BoxGeometry(1.2, 8, 6), glassMat);
  doorGlass.position.set(57.4, 65, -22);
  group.add(doorGlass);

  const upperCanopy = new THREE.Mesh(new THREE.BoxGeometry(8, 1.5, 18), roofMat);
  upperCanopy.position.set(60, 77, -22);
  group.add(upperCanopy);

  // --- Eingang im Erdgeschoss (Südwand +Z, z = 32.5) ---
  const entryPatch = new THREE.Mesh(new THREE.BoxGeometry(22, 1, 27), pathMat);
  entryPatch.position.set(40, 3.1, 46.5);   // Anschluss an den Weg bei x = 47.5
  group.add(entryPatch);

  const groundFrame = new THREE.Mesh(new THREE.BoxGeometry(13, 33, 1), frameMat);
  groundFrame.position.set(40, 19.5, 33);
  group.add(groundFrame);

  const groundDoor = new THREE.Mesh(new THREE.BoxGeometry(10, 30, 1), doorMat);
  groundDoor.position.set(40, 18, 33.5);
  group.add(groundDoor);

  const groundStep = new THREE.Mesh(new THREE.BoxGeometry(14, 1.5, 4), stairMat);
  groundStep.position.set(40, 3.75, 35);
  group.add(groundStep);

  const groundCanopy = new THREE.Mesh(new THREE.BoxGeometry(18, 1.5, 8), roofMat);
  groundCanopy.position.set(40, 37.5, 36.5);
  group.add(groundCanopy);
}
