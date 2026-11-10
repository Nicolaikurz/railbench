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

  // Außentreppe aus Stahl
  const stairMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.5, metalness: 0.6 });
  const stairs = new THREE.Mesh(new THREE.BoxGeometry(15, 45, 25), stairMat);
  stairs.position.set(58, 24, 0);
  stairs.rotation.z = -0.55;
  group.add(stairs);
}
