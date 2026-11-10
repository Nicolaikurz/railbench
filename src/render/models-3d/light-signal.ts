import * as THREE from "three";

/** 6. LICHTSIGNAL (25 x 25 x 85 mm) */
export function buildLightSignal3D(group: THREE.Group): void {
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
