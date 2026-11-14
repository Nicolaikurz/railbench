import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { Viewer3D } from "@/render/viewer-3d";
import { setLanguage } from "@/i18n";
import {
  applyCameraViewPreset,
  createViewCubeWidget,
  orbitCameraByDegrees,
  updateViewCubeOrientation,
} from "@/render/viewer-3d-viewcube";
import type { Layout, TrackPiece, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";

describe("3D Bridge Rendering on Curves and Pillar Collision Detection", () => {
  const straightEntry: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188.3 mm",
    kind: "straight",
    category: "Gleise",
    description: "C-Gleis gerade",
    length: 188.3,
    gauge: 16.5,
  };

  const curveRightEntry: CatalogEntry = {
    id: "24130",
    name: "Gebogenes Gleis R1 30°",
    kind: "curve",
    category: "Kurven",
    description: "C-Gleis Kurve rechts",
    radius: 360,
    angle: 30,
    gauge: 16.5,
  };

  const curveLeftEntry: CatalogEntry = {
    id: "24130-L",
    name: "Gebogenes Gleis R1 30° links",
    kind: "curve",
    category: "Kurven",
    description: "C-Gleis Kurve links",
    radius: 360,
    angle: -30,
    gauge: 16.5,
  };

  const roadEntry: CatalogEntry = {
    id: "ST-200",
    name: "Straße gerade 200 mm",
    kind: "straight",
    category: "Straßen",
    description: "Straße",
    length: 200,
    isRoad: true,
    roadWidth: 60,
  };

  it("renders a curved bridge in 3D with extruded curved deck, curved side girders and handrails", () => {
    const viewer = Object.create(Viewer3D.prototype);
    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const curvedBridgePiece: TrackPiece = {
      id: "br-curve-1",
      catalogId: "24130",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasBridge: true,
    };

    const group = new THREE.Group();
    // Elevated bridge at 80 mm
    viewer.buildTrackBridge3D(group, curveRightEntry, curvedBridgePiece, 80, 80);

    const meshes: THREE.Mesh[] = [];
    const pillarMeshes: THREE.Mesh[] = [];

    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh) {
        meshes.push(obj);
        if (obj.geometry instanceof THREE.CylinderGeometry) {
          const p = obj.geometry.parameters;
          // Pillar radius is 8 to 11
          if (p.radiusTop === 8 && p.radiusBottom === 11) {
            pillarMeshes.push(obj);
          }
        }
      }
    });

    expect(meshes.length).toBeGreaterThan(0);
    // ExtrudeGeometry meshes for curved deck, curved girders and handrails
    const extrudeMeshes = meshes.filter((m) => m.geometry instanceof THREE.ExtrudeGeometry);
    expect(extrudeMeshes.length).toBeGreaterThanOrEqual(3); // Deck + inner girder/rail + outer girder/rail

    // Check that pillars were rendered since there is no obstacle
    expect(pillarMeshes.length).toBeGreaterThan(0);
    // Pillar height should reach from ground (0) to bottom of bridge (80 - 3.5 = 76.5)
    expect((pillarMeshes[0]!.geometry as THREE.CylinderGeometry).parameters.height).toBeCloseTo(76.5, 1);
  });

  it("does NOT render bridge pillars (Stützen) where they collide with a railway track underneath", () => {
    const viewer = Object.create(Viewer3D.prototype);

    // Elevated bridge piece (188.3 mm long, from x=0 to x=188.3 at y=0, elevation 80 mm)
    // Pillars would normally be placed at x=47 (u=0.25) and x=141 (u=0.75)
    const bridgePiece: TrackPiece = {
      id: "br-piece",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasBridge: true,
    };

    // Ground track at elevation 0 mm, passing underneath perpendicular at x=47 mm (y goes from -100 to +100)
    const underTrack: TrackPiece = {
      id: "under-track",
      catalogId: "24188",
      transform: { x: 47, y: -94, rotation: 90 }, // runs along Y through (47, 0)
      connectors: [],
      elevation: { start: 0, end: 0 },
    };

    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [bridgePiece, underTrack],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const group = new THREE.Group();
    viewer.buildTrackBridge3D(group, straightEntry, bridgePiece, 80, 80);

    const pillars: THREE.Mesh[] = [];
    group.traverse((obj) => {
      if (
        obj instanceof THREE.Mesh &&
        obj.geometry instanceof THREE.CylinderGeometry &&
        obj.geometry.parameters.radiusTop === 8
      ) {
        pillars.push(obj);
      }
    });

    // The pillar at x=47 collides with underTrack (even with small offsets).
    // The pillar at x=141 is clear and should be rendered.
    // Total pillars rendered should be exactly 1 instead of 2!
    expect(pillars.length).toBe(1);
    expect(pillars[0]!.position.x).toBeGreaterThan(100); // The non-colliding pillar at x ~141
  });

  it("does NOT render bridge pillars where they collide with a street (road) underneath", () => {
    const viewer = Object.create(Viewer3D.prototype);

    const bridgePiece: TrackPiece = {
      id: "br-piece-2",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasBridge: true,
    };

    // Road at elevation 0 mm passing underneath through x=141 mm
    const underRoad: TrackPiece = {
      id: "under-road",
      catalogId: "ST-200",
      transform: { x: 141, y: -100, rotation: 90 }, // runs through (141, 0)
      connectors: [],
      elevation: { start: 0, end: 0 },
    };

    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [bridgePiece, underRoad],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const group = new THREE.Group();
    viewer.buildTrackBridge3D(group, straightEntry, bridgePiece, 80, 80);

    const pillars: THREE.Mesh[] = [];
    group.traverse((obj) => {
      if (
        obj instanceof THREE.Mesh &&
        obj.geometry instanceof THREE.CylinderGeometry &&
        obj.geometry.parameters.radiusTop === 8
      ) {
        pillars.push(obj);
      }
    });

    // The pillar at x=141 collides with the street.
    // Only the pillar at x=47 should be rendered!
    expect(pillars.length).toBe(1);
    expect(pillars[0]!.position.x).toBeLessThan(70);
  });

  it("does NOT render bridge pillars where they collide with a building underneath", () => {
    const viewer = Object.create(Viewer3D.prototype);

    const bridgePiece: TrackPiece = {
      id: "br-piece-3",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasBridge: true,
    };

    // Building directly underneath candidate pillar at x=47 mm
    const building: Model3D = {
      id: "station-building",
      name: "Bahnhof",
      category: "buildings",
      builtinId: "station",
      modelType: "builtin",
      transform: { x: 47, y: 0, rotation: 0 },
      width: 60,
      depth: 40,
      height: 45,
      elevation: 0,
    };

    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [bridgePiece],
      models: [building],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const group = new THREE.Group();
    viewer.buildTrackBridge3D(group, straightEntry, bridgePiece, 80, 80);

    const pillars: THREE.Mesh[] = [];
    group.traverse((obj) => {
      if (
        obj instanceof THREE.Mesh &&
        obj.geometry instanceof THREE.CylinderGeometry &&
        obj.geometry.parameters.radiusTop === 8
      ) {
        pillars.push(obj);
      }
    });

    // The pillar at x=47 collides with the building footprint.
    // Only the pillar at x=141 should be rendered!
    expect(pillars.length).toBe(1);
    expect(pillars[0]!.position.x).toBeGreaterThan(100);
  });

  it("moves 3D camera to every CAD ViewCube perspective including bottom (underside) and syncs cube rotation", () => {
    const camera = new THREE.PerspectiveCamera(45, 1.5, 5, 220000);
    camera.position.set(0, 1600, 2200);
    const controlsMock = {
      target: new THREE.Vector3(0, 0, 0),
      update: () => {},
    } as any;

    let selectedPreset: string | null = null;
    const vc = createViewCubeWidget(
      (preset) => {
        selectedPreset = preset;
        applyCameraViewPreset(camera, controlsMock, preset, new THREE.Vector3(0, 0, 0));
      },
      (dAz, dPol) => {
        orbitCameraByDegrees(camera, controlsMock, dAz, dPol);
      },
    );

    expect(vc.panelEl.querySelectorAll(".v3d-viewcube-face").length).toBe(6);
    expect(vc.panelEl.querySelectorAll(".v3d-corner-btn").length).toBe(4);

    // Click 'bottom' face to look from underneath the table (negative Y)
    const bottomFace = vc.panelEl.querySelector('[data-view-preset="bottom"]') as HTMLButtonElement;
    bottomFace.click();
    expect(selectedPreset).toBe("bottom");
    expect(camera.position.y).toBeLessThan(-500);

    // Click 'top' face to look straight down from above (positive Y)
    const topFace = vc.panelEl.querySelector('[data-view-preset="top"]') as HTMLButtonElement;
    topFace.click();
    expect(selectedPreset).toBe("top");
    expect(camera.position.y).toBeGreaterThan(500);

    // Click 'left' and 'right' faces
    const leftFace = vc.panelEl.querySelector('[data-view-preset="left"]') as HTMLButtonElement;
    leftFace.click();
    expect(camera.position.x).toBeLessThan(-500);

    const rightFace = vc.panelEl.querySelector('[data-view-preset="right"]') as HTMLButtonElement;
    rightFace.click();
    expect(camera.position.x).toBeGreaterThan(500);

    // Sync CSS 3D cube orientation
    updateViewCubeOrientation(vc.cubeEl, camera, controlsMock);
    expect(vc.cubeEl.style.transform).toContain("rotateX");
    expect(vc.cubeEl.style.transform).toContain("rotateY");

    // Verify dynamic language switching to English and German
    setLanguage("en");
    expect(topFace.textContent).toBe("Top");
    expect(bottomFace.textContent).toBe("Bottom");
    expect(leftFace.textContent).toBe("Left");
    expect(rightFace.textContent).toBe("Right");

    setLanguage("de");
    expect(topFace.textContent).toBe("Oben");
    expect(bottomFace.textContent).toBe("Unten");
    expect(leftFace.textContent).toBe("Links");
    expect(rightFace.textContent).toBe("Rechts");
  });
});
