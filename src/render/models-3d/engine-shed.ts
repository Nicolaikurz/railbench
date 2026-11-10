import * as THREE from "three";

/** 3. LOKSCHUPPEN (250 x 130 x 105 mm) MIT LANDSCHAFT, WEG & WALD */
export function buildEngineShed3D(group: THREE.Group): void {
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
