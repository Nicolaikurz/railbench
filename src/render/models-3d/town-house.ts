import * as THREE from "three";

/** 4. FACHWERKHAUS (110 x 90 x 125 mm) */
export function buildTownHouse3D(group: THREE.Group): void {
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
