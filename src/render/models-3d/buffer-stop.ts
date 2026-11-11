import * as THREE from "three";

/** 7. PRELLBOCK (25 x 35 x 25 mm) */
export function buildBufferStop3D(group: THREE.Group): void {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x374151, roughness: 0.6, metalness: 0.6 });

  // Triangular beam profile, extruded along X
   // Side profile (Z = depth, Y = height), extruded along X for the thickness
  function createBeamProfile(thickness = 3): THREE.BufferGeometry {
    const shape = new THREE.Shape();
    shape.moveTo(-10, 0);     // front bottom, below the plank
    shape.lineTo(12.5, 0);    // rear end of the foot
    shape.lineTo(12.5, 3);    // foot height
    shape.lineTo(-10, 19);    // sloped back up to the plank top
    shape.closePath();

    const geo = new THREE.ExtrudeGeometry(shape, { depth: thickness, bevelEnabled: false });
    geo.translate(0, 0, -thickness / 2); // center the thickness around X = 0
    geo.rotateY(-Math.PI / 2);           // profile X -> world Z, extrusion -> world X
    return geo;
  }

  // Beams left & right (ground at Y = 0, no rotation needed)
  const beamLeft = new THREE.Mesh(createBeamProfile(), steelMat);
  beamLeft.position.set(-8.5, 0, 0);
  group.add(beamLeft);

  const beamRight = new THREE.Mesh(createBeamProfile(), steelMat);
  beamRight.position.set(8.5, 0, 0);
  group.add(beamRight);

  // Holz-Pufferbohle
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 });
  const bufferPlank = new THREE.Mesh(new THREE.BoxGeometry(26, 7, 6), woodMat);
  bufferPlank.position.set(0, 16, -10);
  group.add(bufferPlank);

  // Rote Signal-Kreisscheibe
  const diskMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.5 });
  const disk = new THREE.Mesh(new THREE.CylinderGeometry(4.5, 4.5, 1, 16), diskMat);
  disk.rotation.x = Math.PI / 2;
  disk.position.set(0, 16, -13.5);
  group.add(disk);
}
