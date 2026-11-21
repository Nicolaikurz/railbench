import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedMaterial,
} from "./primitive-cache";

/**
 * 6. LICHTSIGNAL — Authentisches DB-Lichthauptsignal (Hp0/Hp1/Hp2, 25 × 25 × 88 mm)
 * Betonsockel, Schaltkasten, Stahlmast mit weiß-rot-weißem Mastschild,
 * hintere Wartungsleiter, Signalschirm mit weißem Rand & Schuten (Blenden) über Rot/Grün/Gelb.
 */
export function buildLightSignal3D(group: THREE.Group): void {
  const concreteMat = getSharedMaterial(0x64748b, 0.9);
  const mastMat = getSharedMaterial(0x334155, 0.42, 0.72);
  const cabinetMat = getSharedMaterial(0x94a3b8, 0.55, 0.45);
  const shieldMat = getSharedMaterial(0x0f172a, 0.65, 0.2);
  const whiteMat = getSharedMaterial(0xf8fafc, 0.5);
  const redPaintMat = getSharedMaterial(0xdc2626, 0.5);
  const redLampMat = getSharedMaterial(0xef4444, 0.2, 0.1, 0xef4444, 0.9);
  const greenLampMat = getSharedMaterial(0x22c55e, 0.2, 0.1, 0x22c55e, 0.95);
  const yellowLampMat = getSharedMaterial(0xf59e0b, 0.25, 0.1, 0xf59e0b, 0.35);

  // 1. Gestufter Betonsockel
  const base = new THREE.Mesh(getSharedBoxGeometry(18, 4.5, 18), concreteMat);
  base.position.set(0, 2.25, 0);
  group.add(base);

  const footPlate = new THREE.Mesh(getSharedBoxGeometry(9, 1.5, 9), mastMat);
  footPlate.position.set(0, 5.25, 0);
  group.add(footPlate);

  // 2. Stahl-Signalmaster (H-Profil / Rohr) + Schaltkasten
  const mast = new THREE.Mesh(getSharedCylinderGeometry(1.7, 2.2, 74, 10), mastMat);
  mast.position.set(0, 42, 0);
  group.add(mast);

  const cabinet = new THREE.Mesh(getSharedBoxGeometry(9.5, 14, 7.5), cabinetMat);
  cabinet.position.set(0, 13, -1.5);
  group.add(cabinet);

  const cabRoof = new THREE.Mesh(getSharedBoxGeometry(10.5, 1.2, 8.5), mastMat);
  cabRoof.position.set(0, 20.5, -1.5);
  group.add(cabRoof);

  // 3. Weiß-rot-weißes DB-Mastschild (Vorderseite +Z, ohne Rückseiten-Z-Fighting)
  const mastSignBg = new THREE.Mesh(getSharedBoxGeometry(4.2, 22, 0.8), whiteMat);
  mastSignBg.position.set(0, 39, 2.4);
  group.add(mastSignBg);
  const mastSignRed = new THREE.Mesh(getSharedBoxGeometry(2.2, 21.5, 0.5), redPaintMat);
  mastSignRed.position.set(0, 39, 2.85);
  group.add(mastSignRed);

  // 4. Wartungsleiter & kleiner Wartungskorb auf der Rückseite (-Z, startet sauber auf dem Schaltkasten-Dach)
  const railGeo = getSharedCylinderGeometry(0.35, 0.35, 54, 6);
  for (const lx of [-1.8, 1.8]) {
    const rail = new THREE.Mesh(railGeo, mastMat);
    rail.position.set(lx, 48.1, -3.5);
    group.add(rail);
  }
  const rungGeo = getSharedBoxGeometry(3.8, 0.5, 0.5);
  for (let ry = 22; ry <= 74; ry += 5.5) {
    const rung = new THREE.Mesh(rungGeo, mastMat);
    rung.position.set(0, ry, -3.5);
    group.add(rung);
  }

  // Wartungsplattform unter dem Signalschirm
  const platform = new THREE.Mesh(getSharedBoxGeometry(11, 1.0, 7), mastMat);
  platform.position.set(0, 55, -2.5);
  group.add(platform);

  // 5. Signalschirm (schwarz mit weißem Randrahmen)
  const shieldRim = new THREE.Mesh(getSharedBoxGeometry(13.2, 27.2, 1.6), whiteMat);
  shieldRim.position.set(0, 71, 2.8);
  group.add(shieldRim);

  const shieldBody = new THREE.Mesh(getSharedBoxGeometry(11.8, 25.8, 1.8), shieldMat);
  shieldBody.position.set(0, 71, 3.4);
  group.add(shieldBody);

  // 6. Drei Signaloptiken (Grün oben, Rot Mitte, Gelb unten) inkl. Sonnenblenden (Schuten)
  const lensGeo = getSharedCylinderGeometry(1.9, 1.9, 1.6, 14);
  const hoodTopGeo = getSharedBoxGeometry(4.8, 0.7, 4.2);
  const hoodSideGeo = getSharedBoxGeometry(0.6, 2.4, 3.6);

  const aspects: Array<{ y: number; mat: THREE.Material }> = [
    { y: 78, mat: greenLampMat },
    { y: 71, mat: redLampMat },
    { y: 64, mat: yellowLampMat },
  ];

  for (const asp of aspects) {
    const lens = new THREE.Mesh(lensGeo, asp.mat);
    lens.rotation.x = Math.PI / 2;
    lens.position.set(0, asp.y, 4.2);
    group.add(lens);

    // Sonnenblende (Schute)
    const hTop = new THREE.Mesh(hoodTopGeo, shieldMat);
    hTop.position.set(0, asp.y + 2.2, 5.6);
    group.add(hTop);
    for (const sx of [-2.1, 2.1]) {
      const hSide = new THREE.Mesh(hoodSideGeo, shieldMat);
      hSide.position.set(sx, asp.y + 1.1, 5.3);
      group.add(hSide);
    }
  }
}
