import * as THREE from "three";

/** 4. FACHWERKHAUS (110 x 90 x 125 mm) */
export function buildTownHouse3D(group: THREE.Group): void {
  // Materialien
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.8 });
  const baseMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, roughness: 0.8 });
  const beamMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.9 });
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x7c2d12, roughness: 0.8 });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x7dd3fc, roughness: 0.1, metalness: 0.6 });
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x991b1b, roughness: 0.6 });
  const fenceMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, roughness: 0.7 });

  // Landschaftsplatte mit Garten (Oberkante y = 3)
  const garden = new THREE.Mesh(new THREE.BoxGeometry(160, 3, 140), grassMat);
  garden.position.set(0, 1.5, 10);
  group.add(garden);

  // Kiesweg von der Haustür (z = 40) bis zur Pforte (z = 78)
  const walk = new THREE.Mesh(new THREE.BoxGeometry(28, 1, 38), pathMat);
  walk.position.set(0, 3.2, 59);
  group.add(walk);

  // Gartenzaun, symmetrisch, Pforte = Wegbreite
  for (const s of [-1, 1]) {
    for (let fx = 19; fx <= 70; fx += 5.5) {
      const picket = new THREE.Mesh(new THREE.BoxGeometry(2.5, 16, 2), fenceMat);
      picket.position.set(s * fx, 11, 78);
      group.add(picket);
    }
    for (const ry of [7, 14]) {
      const rail = new THREE.Mesh(new THREE.BoxGeometry(55, 1.5, 1), fenceMat);
      rail.position.set(s * 42.5, ry + 3, 77);
      group.add(rail);
    }
    for (const px of [15, 70]) {
      const post = new THREE.Mesh(new THREE.BoxGeometry(3, 20, 3), fenceMat);
      post.position.set(s * px, 13, 78);
      group.add(post);
    }
  }

  // Erdgeschoss verputzt (steht auf dem Rasen, y = 3 bis 41.5)
  const ground = new THREE.Mesh(new THREE.BoxGeometry(100, 38.5, 80), baseMat);
  ground.position.set(0, 22.25, 0);
  group.add(ground);

  // Obergeschoss (kragt 2.5 mm aus, y = 41.5 bis 76.5)
  const upper = new THREE.Mesh(new THREE.BoxGeometry(105, 35, 85), baseMat);
  upper.position.set(0, 59, 0);
  group.add(upper);

  // Schwellbalken unten / Rähm oben (bündig mit den Fachwerkständern)
  const beamH1 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 87), beamMat);
  beamH1.position.set(0, 41.5, 0);
  group.add(beamH1);

  const beamH2 = new THREE.Mesh(new THREE.BoxGeometry(106, 3, 87), beamMat);
  beamH2.position.set(0, 75, 0);
  group.add(beamH2);

  // Fenster-Helfer: lokal +Z zeigt aus der Wand heraus, v = Wandfläche
  function addWindow(parent: THREE.Object3D, u: number, y: number, v: number, w = 11, h = 13): void {
    const frame = new THREE.Mesh(new THREE.BoxGeometry(w + 3, h + 3, 1), beamMat);
    frame.position.set(u, y, v + 0.5);
    parent.add(frame);

    const glass = new THREE.Mesh(new THREE.BoxGeometry(w, h, 1.4), glassMat);
    glass.position.set(u, y, v + 0.7);
    parent.add(glass);

    const barV = new THREE.Mesh(new THREE.BoxGeometry(0.6, h, 1.6), beamMat);
    barV.position.set(u, y, v + 0.8);
    parent.add(barV);

    const barH = new THREE.Mesh(new THREE.BoxGeometry(w, 0.6, 1.6), beamMat);
    barH.position.set(u, y, v + 0.8);
    parent.add(barH);
  }

  // Fassaden: Fenster (EG + OG) und Fachwerk (OG) auf allen vier Seiten
  const frontPosts = [-51, -25.5, 0, 25.5, 51];
  const sidePosts = [-41, -14, 14, 41];
  const facades = [
    { rot: 0, gv: 40, uv: 42.5, gWin: [-25, 25], posts: frontPosts, uWin: [-38.25, -12.75, 12.75, 38.25] },
    { rot: Math.PI, gv: 40, uv: 42.5, gWin: [-30, -10, 10, 30], posts: frontPosts, uWin: [-38.25, -12.75, 12.75, 38.25] },
    { rot: Math.PI / 2, gv: 50, uv: 52.5, gWin: [-20, 20], posts: sidePosts, uWin: [-27.5, 0, 27.5] },
    { rot: -Math.PI / 2, gv: 50, uv: 52.5, gWin: [-20, 20], posts: sidePosts, uWin: [-27.5, 0, 27.5] },
  ];

  for (const f of facades) {
    const g = new THREE.Group();
    g.rotation.y = f.rot;
    group.add(g);

    for (const u of f.gWin) addWindow(g, u, 24, f.gv);
    for (const u of f.uWin) addWindow(g, u, 60, f.uv);

    const first = f.posts[0];
    const last = f.posts[f.posts.length - 1];
    for (const u of f.posts) {
      // Ständer zwischen Schwelle (y = 43) und Rähm (y = 73.5)
      const post = new THREE.Mesh(new THREE.BoxGeometry(2.5, 30.5, 1), beamMat);
      post.position.set(u, 58.25, f.uv + 0.5);
      g.add(post);

      // Kopfbänder (Streben) links und rechts vom Ständer
      for (const s of [-1, 1]) {
        if ((s < 0 && u === first) || (s > 0 && u === last)) continue;
        const brace = new THREE.Mesh(new THREE.BoxGeometry(7.8, 1.8, 1), beamMat);
        brace.position.set(u + s * 4, 70.75, f.uv + 0.5);
        brace.rotation.z = s * (Math.PI / 4);
        g.add(brace);
      }
    }
  }

  // Haustür (Vorderseite +Z, Wand bei z = 40)
  const doorFrame = new THREE.Mesh(new THREE.BoxGeometry(16, 28, 1), beamMat);
  doorFrame.position.set(0, 17, 40.5);
  group.add(doorFrame);

  const door = new THREE.Mesh(new THREE.BoxGeometry(12, 26, 1.2), doorMat);
  door.position.set(0, 16, 40.7);
  group.add(door);

  const doorGlass = new THREE.Mesh(new THREE.BoxGeometry(6, 9, 1.4), glassMat);
  doorGlass.position.set(0, 22, 40.7);
  group.add(doorGlass);

  const handle = new THREE.Mesh(new THREE.SphereGeometry(0.8, 8, 8), new THREE.MeshStandardMaterial({ color: 0xca8a04, metalness: 0.8, roughness: 0.3 }));
  handle.position.set(3.8, 16, 41.8);
  group.add(handle);

  const step = new THREE.Mesh(new THREE.BoxGeometry(20, 1.5, 5), new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.8 }));
  step.position.set(0, 4.45, 42.5);
  group.add(step);

  // Steiles Satteldach (Firstrichtung X)
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-46, 0);
  roofShape.lineTo(0, 45);
  roofShape.lineTo(46, 0);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 112, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-56, 75, 0);
  group.add(new THREE.Mesh(roofGeo, roofMat));

  // Verputzte Giebelflächen, parallel zur Dachkante eingerückt
  const gableShape = new THREE.Shape();
  gableShape.moveTo(-42, 0);
  gableShape.lineTo(0, 41.1);
  gableShape.lineTo(42, 0);
  gableShape.closePath();
  for (const gx of [56, -56.6]) {
    const gableGeo = new THREE.ExtrudeGeometry(gableShape, { depth: 0.6, bevelEnabled: false });
    gableGeo.rotateY(Math.PI / 2);
    gableGeo.translate(gx, 76.5, 0);
    group.add(new THREE.Mesh(gableGeo, baseMat));
  }

  // Schornstein mit Abdeckplatte (Oberkante y = 124)
  const brickMat = new THREE.MeshStandardMaterial({ color: 0xb91c1c, roughness: 0.9 });
  const chim = new THREE.Mesh(new THREE.BoxGeometry(12, 34, 12), brickMat);
  chim.position.set(20, 105, 0);
  group.add(chim);

  const chimCap = new THREE.Mesh(new THREE.BoxGeometry(14, 2, 14), new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.8 }));
  chimCap.position.set(20, 123, 0);
  group.add(chimCap);
}