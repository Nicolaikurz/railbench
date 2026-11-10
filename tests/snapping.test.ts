import { describe, it, expect } from "vitest";
import {
  findSnap,
  findBestSnapForPiece,
  areConnectorsConnected,
  getConnectorSnapStatus,
  findSnapAboveForPiece,
  updatePieceConnections,
  getPieceConnectorHeight,
} from "@/core/snapping";
import type { TrackPiece, Layer } from "@/core/types";
import {
  distanceToTrackPiece,
  isPieceInSelectionBox,
  getPieceCenterWorld,
  rotatePieceAroundCenter,
  rotateGroupAroundCentroid,
  computeGroupCentroid,
  dist,
} from "@/core/geometry";
import { Selection } from "@/editor/selection";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import { createTrackGraphics } from "@/render/track-sprite";
import {
  getLocalConnectors,
  catalogRegistry,
  maerklinCCatalog,
  trixCCatalog,
  pikoACatalog,
  tilligTTCatalog,
  fleischmannPiccoloCatalog,
  type CatalogEntry,
} from "@/catalog/catalog";

describe("findSnap", () => {
  it("rastet an gegenläufigem, freiem Connector ein", () => {
    const piece: TrackPiece = {
      id: "a", catalogId: "g200",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [{ x: 200, y: 0, angle: 0 }],
    };
    const hit = findSnap({ x: 205, y: 2, angle: 180 }, [piece]);
    expect(hit?.pieceId).toBe("a");
  });

  it("berechnet Kreuzung 24640 (KW) Anschlusspunkte mit 24,3°", () => {
    const entry: CatalogEntry = {
      id: "24640",
      name: "Kreuzung KW",
      kind: "crossing",
      length: 188.3,
      angle: 24.3,
      category: "Kreuzungen",
      description: "Kreuzung",
    };
    const connectors = getLocalConnectors(entry);
    expect(connectors.length).toBe(4);
    // Gleis 1 (horizontal zentriert)
    expect(connectors[0]!.x).toBeCloseTo(-94.15, 2);
    expect(connectors[0]!.y).toBeCloseTo(0, 2);
    expect(connectors[0]!.angle).toBe(180);
    expect(connectors[1]!.x).toBeCloseTo(94.15, 2);
    expect(connectors[1]!.y).toBeCloseTo(0, 2);
    expect(connectors[1]!.angle).toBe(0);

    // Gleis 2 (Winkel 24,3°)
    const h = 188.3 / 2;
    const rad = (24.3 * Math.PI) / 180;
    expect(connectors[2]!.x).toBeCloseTo(-h * Math.cos(rad), 2);
    expect(connectors[2]!.y).toBeCloseTo(-h * Math.sin(rad), 2);
    expect(connectors[2]!.angle).toBeCloseTo(204.3, 1);
    expect(connectors[3]!.x).toBeCloseTo(h * Math.cos(rad), 2);
    expect(connectors[3]!.y).toBeCloseTo(h * Math.sin(rad), 2);
    expect(connectors[3]!.angle).toBeCloseTo(24.3, 1);
  });

  it("berechnet DKW 24624 Anschlusspunkte beginnend bei (0,0) passend zu Weichen", () => {
    const entry: CatalogEntry = {
      id: "24624",
      name: "DKW",
      kind: "dkw",
      length: 188.3,
      angle: 24.3,
      branchRadius: 437.5,
      branchAngle: 24.3,
      category: "Kreuzungen",
      description: "Doppelkreuzungsweiche",
    };
    const connectors = getLocalConnectors(entry);
    expect(connectors.length).toBe(4);
    // Hauptgleis von (0, 0) bis (188.3, 0)
    expect(connectors[0]!.x).toBe(0);
    expect(connectors[0]!.y).toBe(0);
    expect(connectors[0]!.angle).toBe(180);
    expect(connectors[1]!.x).toBeCloseTo(188.3, 2);
    expect(connectors[1]!.y).toBe(0);
    expect(connectors[1]!.angle).toBe(0);

    // Kreuzendes Gleis durch Mittelpunkt (94.15, 0)
    const h = 188.3 / 2;
    const rad = (24.3 * Math.PI) / 180;
    expect(connectors[2]!.x).toBeCloseTo(h - h * Math.cos(rad), 2);
    expect(connectors[2]!.y).toBeCloseTo(-h * Math.sin(rad), 2);
    expect(connectors[2]!.angle).toBeCloseTo(204.3, 1);
    expect(connectors[3]!.x).toBeCloseTo(h + h * Math.cos(rad), 2);
    expect(connectors[3]!.y).toBeCloseTo(h * Math.sin(rad), 2);
    expect(connectors[3]!.angle).toBeCloseTo(24.3, 1);

    // Prüfen, ob beide Abzweigbögen innerhalb der DKW-Geometrie liegen
    // Bogen 1 (von 0,0 nach rechts/unten zu Anschluss 4)
    const midTurn1 = { x: 437.5 * Math.sin((12.15 * Math.PI) / 180), y: 437.5 * (1 - Math.cos((12.15 * Math.PI) / 180)) };
    const distTurn1 = distanceToTrackPiece(midTurn1, { x: 0, y: 0, rotation: 0 }, entry);
    expect(distTurn1).toBeCloseTo(0, 1);

    // Bogen 2 (von l,0 nach links/oben zu Anschluss 3)
    const midTurn2 = {
      x: 188.3 - 437.5 * Math.sin((12.15 * Math.PI) / 180),
      y: -437.5 * (1 - Math.cos((12.15 * Math.PI) / 180)),
    };
    const distTurn2 = distanceToTrackPiece(midTurn2, { x: 0, y: 0, rotation: 0 }, entry);
    expect(distTurn2).toBeCloseTo(0, 1);
  });

  it("berechnet Bogenweiche 24671 (links) mit R1=360mm und integrierter Ausgleichsgerade 77.5mm", () => {
    const entry = maerklinCCatalog.entries.find((e) => e.id === "24671");
    expect(entry).toBeDefined();
    expect(entry!.kind).toBe("curved-turnout-left");

    const connectors = getLocalConnectors(entry!);
    expect(connectors.length).toBe(3);

    // Wurzel (Einfahrt)
    expect(connectors[0]!.x).toBe(0);
    expect(connectors[0]!.y).toBe(0);
    expect(connectors[0]!.angle).toBe(180);

    // Innenbogen R1=360 mm, 30° (nach links / +y)
    expect(connectors[1]!.x).toBeCloseTo(180.0, 1);
    expect(connectors[1]!.y).toBeCloseTo(48.23, 1);
    expect(connectors[1]!.angle).toBeCloseTo(30, 1);

    // Außenstrang: 77,5 mm Gerade + R1=360 mm / 30°
    expect(connectors[2]!.x).toBeCloseTo(257.5, 1);
    expect(connectors[2]!.y).toBeCloseTo(48.23, 1);
    expect(connectors[2]!.angle).toBeCloseTo(30, 1);

    // Differenz in x entspricht genau der integrierten Geraden 24077 (77,5 mm)
    expect(connectors[2]!.x - connectors[1]!.x).toBeCloseTo(77.5, 1);
    expect(connectors[2]!.y - connectors[1]!.y).toBeCloseTo(0.0, 1);

    // Distanztest zu beiden Strängen:
    // 1. Punkt auf Innenbogen (bei 15°)
    const ptInner = {
      x: 360 * Math.sin((15 * Math.PI) / 180),
      y: 360 * (1 - Math.cos((15 * Math.PI) / 180)),
    };
    expect(distanceToTrackPiece(ptInner, { x: 0, y: 0, rotation: 0 }, entry!)).toBeCloseTo(0, 1);

    // 2. Punkt auf geradem Zwischenstück des Außenstrangs (z.B. bei x = 40)
    const ptLead = { x: 40, y: 0 };
    expect(distanceToTrackPiece(ptLead, { x: 0, y: 0, rotation: 0 }, entry!)).toBeCloseTo(0, 1);

    // 3. Punkt auf Bogen des Außenstrangs (bei 15° ab x = 77.5)
    const ptOuter = {
      x: 77.5 + 360 * Math.sin((15 * Math.PI) / 180),
      y: 360 * (1 - Math.cos((15 * Math.PI) / 180)),
    };
    expect(distanceToTrackPiece(ptOuter, { x: 0, y: 0, rotation: 0 }, entry!)).toBeCloseTo(0, 1);
  });

  it("berechnet Bogenweiche 24672 (rechts) mit R1=360mm und integrierter Ausgleichsgerade 77.5mm nach rechts (-y)", () => {
    const entry = maerklinCCatalog.entries.find((e) => e.id === "24672");
    expect(entry).toBeDefined();
    expect(entry!.kind).toBe("curved-turnout-right");

    const connectors = getLocalConnectors(entry!);
    expect(connectors.length).toBe(3);

    // Wurzel (Einfahrt)
    expect(connectors[0]!.x).toBe(0);
    expect(connectors[0]!.y).toBe(0);
    expect(connectors[0]!.angle).toBe(180);

    // Innenbogen R1=360 mm, 30° (nach rechts / -y)
    expect(connectors[1]!.x).toBeCloseTo(180.0, 1);
    expect(connectors[1]!.y).toBeCloseTo(-48.23, 1);
    expect(connectors[1]!.angle).toBeCloseTo(330, 1);

    // Außenstrang: 77,5 mm Gerade + R1=360 mm / 30° (nach rechts / -y)
    expect(connectors[2]!.x).toBeCloseTo(257.5, 1);
    expect(connectors[2]!.y).toBeCloseTo(-48.23, 1);
    expect(connectors[2]!.angle).toBeCloseTo(330, 1);

    expect(connectors[2]!.x - connectors[1]!.x).toBeCloseTo(77.5, 1);
  });

  it("berechnet schlanke Bogenweiche 24771 (links) mit R3=515mm und integrierter Ausgleichsgerade 64.3mm", () => {
    const entry = maerklinCCatalog.entries.find((e) => e.id === "24771");
    expect(entry).toBeDefined();
    expect(entry!.kind).toBe("curved-turnout-left");

    const connectors = getLocalConnectors(entry!);
    expect(connectors.length).toBe(3);

    // Wurzel
    expect(connectors[0]!.x).toBe(0);
    expect(connectors[0]!.y).toBe(0);
    expect(connectors[0]!.angle).toBe(180);

    // Innenbogen R3=515mm, 30°
    const rad30 = (30 * Math.PI) / 180;
    const expX1 = 515 * Math.sin(rad30); // 257.5
    const expY1 = 515 * (1 - Math.cos(rad30)); // 68.996
    expect(connectors[1]!.x).toBeCloseTo(expX1, 1);
    expect(connectors[1]!.y).toBeCloseTo(expY1, 1);
    expect(connectors[1]!.angle).toBeCloseTo(30, 1);

    // Außenstrang: 64.3mm Gerade + R3 30°
    expect(connectors[2]!.x).toBeCloseTo(64.3 + expX1, 1);
    expect(connectors[2]!.y).toBeCloseTo(expY1, 1);
    expect(connectors[2]!.angle).toBeCloseTo(30, 1);
  });

  it("umfasst das vollständige Märklin C-Gleis Sortiment mit 49 Gleisen", () => {
    expect(maerklinCCatalog.entries.length).toBe(49);

    // Überprüfe Flexgleis 24900
    const flexTrack = maerklinCCatalog.entries.find((e) => e.id === "24900");
    expect(flexTrack).toBeDefined();
    expect(flexTrack?.kind).toBe("flex");
    expect(flexTrack?.length).toBe(900.0);

    // Überprüfe wichtige Neuzugänge
    const track360 = maerklinCCatalog.entries.find((e) => e.id === "24360");
    expect(track360?.length).toBe(360.0);

    const dkwSlim = maerklinCCatalog.entries.find((e) => e.id === "24720");
    expect(dkwSlim?.angle).toBe(12.1);
    expect(dkwSlim?.length).toBe(236.1);

    const crossingSlim = maerklinCCatalog.entries.find((e) => e.id === "24740");
    expect(crossingSlim?.angle).toBe(12.1);

    const crossingWide = maerklinCCatalog.entries.find((e) => e.id === "24649");
    expect(crossingWide?.angle).toBe(48.6);

    const bumperLit = maerklinCCatalog.entries.find((e) => e.id === "24978");
    expect(bumperLit?.length).toBe(77.5);

    const roadbedEnd = maerklinCCatalog.entries.find((e) => e.id === "24001");
    expect(roadbedEnd?.length).toBe(16.5);

    const turntable = maerklinCCatalog.entries.find((e) => e.id === "74862");
    expect(turntable).toBeDefined();
    expect(turntable?.kind).toBe("turntable");
    expect(turntable?.radius).toBe(189);
    expect(turntable?.localConnectors).toHaveLength(30);
  });
});

describe("catalogRegistry and track systems", () => {
  it("enthält die vordefinierten Gleissysteme (Märklin C H0, Trix C H0, Piko A H0, Roco-Line H0, Fleischmann N, Tillig TT)", () => {
    const list = catalogRegistry.getAll();
    expect(list.length).toBeGreaterThanOrEqual(6);

    expect(catalogRegistry.get("maerklin-c-h0")).toBeDefined();
    expect(catalogRegistry.get("trix-c-h0")).toBeDefined();
    expect(catalogRegistry.get("piko-a-h0")).toBeDefined();
    expect(catalogRegistry.get("roco-line-h0")).toBeDefined();
    expect(catalogRegistry.get("tillig-tt")).toBeDefined();
    expect(catalogRegistry.get("fleischmann-piccolo-n")).toBeDefined();
  });

  it("enthält die vollständige Trix C-Gleis Bibliothek basierend auf Märklin C-Gleis Geometrie", () => {
    const cat = catalogRegistry.get("trix-c-h0");
    expect(cat).toBeDefined();
    expect(cat?.manufacturer).toBe("Trix");
    expect(cat?.system).toBe("C-Gleis");
    expect(cat?.scale).toBe("H0");
    expect(cat?.gauge).toBe(16.5);
    expect(cat?.hasRoadbed).toBe(true);

    expect(trixCCatalog.entries.length).toBe(47);

    // Wichtige Trix Gleise überprüfen
    const tTurntable = trixCCatalog.entries.find((e) => e.id === "66862");
    expect(tTurntable).toBeDefined();
    expect(tTurntable?.kind).toBe("turntable");
    const t188 = trixCCatalog.entries.find((e) => e.id === "62188");
    expect(t188?.length).toBe(188.3);
    expect(t188?.kind).toBe("straight");

    const t224 = trixCCatalog.entries.find((e) => e.id === "62224");
    expect(t224?.radius).toBe(437.5);
    expect(t224?.angle).toBe(24.3);

    const tTurnout = trixCCatalog.entries.find((e) => e.id === "62611");
    expect(tTurnout?.branchRadius).toBe(437.5);
    expect(tTurnout?.branchAngle).toBe(24.3);

    const tDkw = trixCCatalog.entries.find((e) => e.id === "62624");
    expect(tDkw?.kind).toBe("dkw");
    expect(tDkw?.length).toBe(188.3);

    const tSlimCrossing = trixCCatalog.entries.find((e) => e.id === "62740");
    expect(tSlimCrossing?.angle).toBe(12.1);
  });

  it("findet Gleisstücke systemübergreifend mit findEntry", () => {
    // Märklin C
    const mStraight = catalogRegistry.findEntry("24188");
    expect(mStraight?.kind).toBe("straight");
    expect(mStraight?.gauge).toBe(16.5);

    // Trix C
    const tStraight = catalogRegistry.findEntry("62188");
    expect(tStraight?.kind).toBe("straight");
    expect(tStraight?.length).toBe(188.3);
    expect(tStraight?.gauge).toBe(16.5);
    expect(tStraight?.hasRoadbed).toBe(true);

    // Piko A
    const pStraight = catalogRegistry.findEntry("55201");
    expect(pStraight?.name).toContain("G231");
    expect(pStraight?.hasRoadbed).toBe(false);

    // Fleischmann N
    const fCurve = catalogRegistry.findEntry("9120");
    expect(fCurve?.radius).toBe(192);
    expect(fCurve?.gauge).toBe(9.0);
  });

  it("kann benutzerdefinierte Kataloge aus JSON laden und registrieren", () => {
    const customJson = JSON.stringify({
      id: "test-gartenbahn-g",
      name: "LGB Spur G",
      manufacturer: "LGB",
      system: "Messinggleis",
      scale: "G",
      gauge: 45.0,
      hasRoadbed: false,
      entries: [
        {
          id: "10000",
          name: "Gerades Gleis 300 mm",
          kind: "straight",
          category: "Geraden",
          length: 300,
          description: "Standard Gerade Spur G",
        },
      ],
    });

    const loaded = catalogRegistry.loadFromJSON(customJson);
    expect(loaded.id).toBe("test-gartenbahn-g");
    expect(loaded.gauge).toBe(45.0);
    expect(loaded.entries.length).toBe(1);

    expect(catalogRegistry.get("test-gartenbahn-g")).toBeDefined();
    expect(catalogRegistry.findEntry("10000")?.length).toBe(300);
  });
});

describe("isPieceInSelectionBox & multi-selection", () => {
  const g188: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188,3 mm",
    kind: "straight",
    category: "Geraden",
    length: 188.3,
    description: "Standardgerade",
  };

  it("erkennt Gleise, die innerhalb der Auswahlbox liegen", () => {
    const piece: TrackPiece = {
      id: "p1",
      catalogId: "24188",
      transform: { x: 50, y: 50, rotation: 0 },
      connectors: [
        { x: 50, y: 50, angle: 180 },
        { x: 238.3, y: 50, angle: 0 },
      ],
    };

    // Box umschließt das Gleis komplett
    expect(
      isPieceInSelectionBox(piece, g188, { x: 0, y: 0 }, { x: 300, y: 100 }),
    ).toBe(true);

    // Box liegt weit weg vom Gleis
    expect(
      isPieceInSelectionBox(piece, g188, { x: 500, y: 500 }, { x: 600, y: 600 }),
    ).toBe(false);

    // Box schneidet nur einen Teil des Gleises
    expect(
      isPieceInSelectionBox(piece, g188, { x: 100, y: 40 }, { x: 150, y: 60 }),
    ).toBe(true);
  });

  it("unterstützt Mehrfachauswahl im Selection-Model", () => {
    const sel = new Selection();
    sel.setMany(["p1", "p2", "p3"]);
    expect(sel.all().length).toBe(3);
    expect(sel.has("p2")).toBe(true);

    sel.add("p4");
    expect(sel.all().length).toBe(4);
    expect(sel.has("p4")).toBe(true);
  });

  it("liefert konsistente Farben für identische Gleise im Bauplan-Modus", () => {
    const color1 = getTrackBlueprintColor("24188");
    const color2 = getTrackBlueprintColor("24188");
    expect(color1.hex).toBe(color2.hex);
    expect(color1.css).toBe(color2.css);

    const colorDifferent = getTrackBlueprintColor("24611");
    expect(colorDifferent.css).toBeDefined();
    expect(colorDifferent.badgeBg).toBeDefined();
  });

  it("unterstützt die 3 Farbmodi: off (realistisch), types (Gleistypen) und layers (Ebenenfarben)", () => {
    const piece: TrackPiece = {
      id: "p1",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
      layerId: "layer-2",
    };

    const spriteOff = createTrackGraphics(piece, g188,undefined, { colorMode: "off" });
    expect(spriteOff).toBeDefined();

    const spriteTypes = createTrackGraphics(piece, g188,undefined, { colorMode: "types" });
    expect(spriteTypes).toBeDefined();

    const spriteLayers = createTrackGraphics(piece, g188,undefined, {
      colorMode: "layers",
      layerColor: "#ec4899",
    });
    expect(spriteLayers).toBeDefined();
  });
});

describe("snap in place & center rotation", () => {
  const g188: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188.3 mm",
    kind: "straight",
    length: 188.3,
    category: "Gerade",
    description: "Gerade",
  };

  const r1_30: CatalogEntry = {
    id: "24130",
    name: "Bogen R1 30°",
    kind: "curve",
    radius: 360,
    angle: 30,
    category: "Kurven",
    description: "Kurve",
  };

  it("rotatePieceAroundCenter dreht ein Einzelgleis exakt um seinen geometrischen Mittelpunkt", () => {
    const origTransform = { x: 100, y: 200, rotation: 0 };
    const centerBefore = getPieceCenterWorld(g188, origTransform);

    // Drehung um 90 Grad
    const newTransform90 = rotatePieceAroundCenter(g188, origTransform, 90);
    expect(newTransform90.rotation).toBe(90);

    const centerAfter90 = getPieceCenterWorld(g188, newTransform90);
    expect(centerAfter90.x).toBeCloseTo(centerBefore.x, 2);
    expect(centerAfter90.y).toBeCloseTo(centerBefore.y, 2);

    // Drehung um weitere 45 Grad (135 Grad insgesamt)
    const newTransform135 = rotatePieceAroundCenter(g188, newTransform90, 45);
    expect(newTransform135.rotation).toBe(135);

    const centerAfter135 = getPieceCenterWorld(g188, newTransform135);
    expect(centerAfter135.x).toBeCloseTo(centerBefore.x, 2);
    expect(centerAfter135.y).toBeCloseTo(centerBefore.y, 2);
  });

  it("rotatePieceAroundCenter funktioniert auch für Bogengleise präzise um das Bogen-Centrum", () => {
    const origTransform = { x: 50, y: 50, rotation: 15 };
    const centerBefore = getPieceCenterWorld(r1_30, origTransform);

    const newTransform = rotatePieceAroundCenter(r1_30, origTransform, 60);
    expect(newTransform.rotation).toBe(75);

    const centerAfter = getPieceCenterWorld(r1_30, newTransform);
    expect(centerAfter.x).toBeCloseTo(centerBefore.x, 2);
    expect(centerAfter.y).toBeCloseTo(centerBefore.y, 2);
  });

  it("rotateGroupAroundCentroid dreht mehrere Gleise starr als eine Einheit um ihren gemeinsamen Schwerpunkt", () => {
    // 2 Geraden hintereinander:
    // Gleis 1: (0, 0, 0) -> Zentrum bei (94.15, 0)
    // Gleis 2: (188.3, 0, 0) -> Zentrum bei (282.45, 0)
    // Gemeinsamer Schwerpunkt: (188.3, 0)
    const t1 = { x: 0, y: 0, rotation: 0 };
    const t2 = { x: 188.3, y: 0, rotation: 0 };

    const items = [
      { id: "g1", type: "piece" as const, transform: t1, entry: g188 },
      { id: "g2", type: "piece" as const, transform: t2, entry: g188 },
    ];

    const centroid = computeGroupCentroid(items);
    expect(centroid.x).toBeCloseTo(188.3, 2);
    expect(centroid.y).toBeCloseTo(0, 2);

    // 90° Drehung um den Schwerpunkt
    const newTransforms = rotateGroupAroundCentroid(items, 90, centroid);
    const newT1 = newTransforms.get("g1")!;
    const newT2 = newTransforms.get("g2")!;

    expect(newT1.rotation).toBe(90);
    expect(newT2.rotation).toBe(90);

    const c1 = getPieceCenterWorld(g188, newT1);
    const c2 = getPieceCenterWorld(g188, newT2);

    // Abstand zwischen den Zentren beider Gleise bleibt exakt 188.3 mm (starre Verbindung!)
    expect(dist(c1, c2)).toBeCloseTo(188.3, 2);

    // Neuer Schwerpunkt der gedrehten Gruppe bleibt identisch
    expect((c1.x + c2.x) / 2).toBeCloseTo(centroid.x, 2);
    expect((c1.y + c2.y) / 2).toBeCloseTo(centroid.y, 2);
  });

  it("findBestSnapForPiece rastet mit Position UND Rotation nahtlos ein ('snap in place')", () => {
    const fixedPiece: TrackPiece = {
      id: "fixed",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Kandidat wird herangeführt, liegt bei (195, 8) und ist 35° verdreht
    const candidateT = { x: 195, y: 8, rotation: 35 };
    const snap = findBestSnapForPiece(g188, candidateT, [fixedPiece], undefined, 45, 60);

    expect(snap).not.toBeNull();
    // Position dockt exakt an (188.3, 0) an
    expect(snap!.snappedTransform.x).toBeCloseTo(188.3, 2);
    expect(snap!.snappedTransform.y).toBeCloseTo(0, 2);
    // Drehwinkel dockt exakt an 0° an
    expect(snap!.snappedTransform.rotation).toBeCloseTo(0, 2);
  });

  it("findBestSnapForPiece unterstützt ignorePieceIds als Set bei Gruppenbewegungen", () => {
    const fixedPiece: TrackPiece = {
      id: "p-fixed",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };
    const movingPiece1: TrackPiece = {
      id: "p-dragged-1",
      catalogId: "24188",
      transform: { x: 188.3, y: 0, rotation: 0 },
      connectors: [
        { x: 188.3, y: 0, angle: 180 },
        { x: 376.6, y: 0, angle: 0 },
      ],
    };

    const draggedSet = new Set(["p-dragged-1", "p-dragged-2"]);
    const snap = findBestSnapForPiece(
      g188,
      { x: 190, y: 3, rotation: 0 },
      [fixedPiece, movingPiece1],
      draggedSet,
    );

    // Rastet an fixedPiece an, ignoriert movingPiece1
    expect(snap?.targetPieceId).toBe("p-fixed");
  });

  it("dreht Straßenstücke (Geraden, Kurven, Kreuzungen) exakt im geometrischen Straßen-Centrum", () => {
    const roadStraight: CatalogEntry = {
      id: "ST-S200",
      name: "Gerade Straße 200 mm",
      kind: "straight",
      length: 200,
      isRoad: true,
      roadWidth: 60,
      category: "Geraden",
      description: "Straße",
    };

    const roadCross: CatalogEntry = {
      id: "ST-X-90",
      name: "Straßenkreuzung 90°",
      kind: "crossing",
      length: 200,
      isRoad: true,
      roadWidth: 60,
      category: "Kreuzungen",
      description: "Kreuzung",
    };

    // Gerade Straße: Länge 200 mm, Zentrum bei lokal (100, 0)
    const tStraight = { x: 50, y: 50, rotation: 0 };
    const centerBeforeS = getPieceCenterWorld(roadStraight, tStraight);
    expect(centerBeforeS.x).toBeCloseTo(150, 2);
    expect(centerBeforeS.y).toBeCloseTo(50, 2);

    const rotatedStraight = rotatePieceAroundCenter(roadStraight, tStraight, 45);
    expect(rotatedStraight.rotation).toBe(45);
    const centerAfterS = getPieceCenterWorld(roadStraight, rotatedStraight);
    expect(centerAfterS.x).toBeCloseTo(centerBeforeS.x, 2);
    expect(centerAfterS.y).toBeCloseTo(centerBeforeS.y, 2);

    // Straßenkreuzung: Zentrum bei lokal (100, 0)
    const tCross = { x: 200, y: 200, rotation: 0 };
    const centerBeforeX = getPieceCenterWorld(roadCross, tCross);
    expect(centerBeforeX.x).toBeCloseTo(300, 2);
    expect(centerBeforeX.y).toBeCloseTo(200, 2);

    const rotatedCross = rotatePieceAroundCenter(roadCross, tCross, 90);
    expect(rotatedCross.rotation).toBe(90);
    const centerAfterX = getPieceCenterWorld(roadCross, rotatedCross);
    expect(centerAfterX.x).toBeCloseTo(centerBeforeX.x, 2);
    expect(centerAfterX.y).toBeCloseTo(centerBeforeX.y, 2);
  });

  it("rastet Straßenstücke nahtlos aneinander ein ('snap in place')", () => {
    const roadPieceA: TrackPiece = {
      id: "road-1",
      catalogId: "ST-S200",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 200, y: 0, angle: 0 },
      ],
    };

    const roadEntry: CatalogEntry = {
      id: "ST-S200",
      name: "Gerade Straße 200 mm",
      kind: "straight",
      length: 200,
      isRoad: true,
      roadWidth: 60,
      category: "Geraden",
      description: "Straße",
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 200, y: 0, angle: 0 },
      ],
    };

    // Straßenkandidat wird herangeführt mit 40° Fehlausrichtung bei (208, 12)
    const snap = findBestSnapForPiece(roadEntry, { x: 208, y: 12, rotation: 40 }, [roadPieceA]);
    expect(snap).not.toBeNull();
    expect(snap!.snappedTransform.x).toBeCloseTo(200, 2);
    expect(snap!.snappedTransform.y).toBeCloseTo(0, 2);
    expect(snap!.snappedTransform.rotation).toBeCloseTo(0, 2);
  });

  it("trennt Gleise und Straßen strikt voneinander beim Snapping", () => {
    const trackPiece: TrackPiece = {
      id: "track-1",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    const roadEntry: CatalogEntry = {
      id: "ST-S200",
      name: "Gerade Straße 200 mm",
      kind: "straight",
      length: 200,
      isRoad: true,
      roadWidth: 60,
      category: "Geraden",
      description: "Straße",
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 200, y: 0, angle: 0 },
      ],
    };

    // Straße darf NICHT an Schienengleis einrasten
    const snapRoadToTrack = findBestSnapForPiece(roadEntry, { x: 188.3, y: 0, rotation: 0 }, [trackPiece]);
    expect(snapRoadToTrack).toBeNull();
  });

  it("unterstützt ignoreTargetConnectors für flüssiges Wegziehen ohne Magnet-Blockade", () => {
    const fixedPiece: TrackPiece = {
      id: "p-fixed",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Kandidat zieht gerade leicht weg (5mm Verschiebung)
    const candT = { x: 188.3 + 5, y: 3, rotation: 0 };

    // Mit ignoreTargetConnectors: snap ignoriert den Ausgangs-Connector und blockiert die Bewegung nicht
    const snapIgnored = findBestSnapForPiece(
      g188,
      candT,
      [fixedPiece],
      undefined,
      45,
      60,
      [{ pieceId: "p-fixed", connectorIndex: 1 }],
    );
    expect(snapIgnored).toBeNull();

    // Ohne ignoreTargetConnectors: snap würde sofort wieder einrasten
    const snapNormal = findBestSnapForPiece(g188, candT, [fixedPiece], undefined, 45, 60);
    expect(snapNormal).not.toBeNull();
  });

  describe("Höhen-Snapping & Snap-Punkt-Farbstatus (grau, gelb, grün)", () => {
    const layers: Layer[] = [
      { id: "ground", name: "Grundebene", heightMm: 0, visible: true, locked: false },
      { id: "bridge", name: "Brückenebene", heightMm: 80, visible: true, locked: false },
    ];

    it("areConnectorsConnected prüft Position, Gegenwinkel und gleiche Höhe", () => {
      const c1 = { x: 188.3, y: 0, angle: 0 };
      const c2 = { x: 188.3, y: 0, angle: 180 };

      // Gleiche Höhe: verbunden
      expect(areConnectorsConnected(c1, c2, 0, 0)).toBe(true);
      expect(areConnectorsConnected(c1, c2, 80, 80)).toBe(true);

      // Unterschiedliche Höhe: NICHT verbunden
      expect(areConnectorsConnected(c1, c2, 0, 80)).toBe(false);
      expect(areConnectorsConnected(c1, c2, 0, 1.5)).toBe(false);

      // Falscher Winkel: NICHT verbunden
      const cWrongAngle = { x: 188.3, y: 0, angle: 0 };
      expect(areConnectorsConnected(c1, cWrongAngle, 0, 0)).toBe(false);

      // Falsche Position: NICHT verbunden
      const cFar = { x: 200, y: 0, angle: 180 };
      expect(areConnectorsConnected(c1, cFar, 0, 0)).toBe(false);
    });

    it("färbt verbundene Gleise grau ('status: connected', 'color: grey')", () => {
      const p1: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };
      const p2: TrackPiece = {
        id: "p2",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 188.3, y: 0, angle: 180 },
          { x: 376.6, y: 0, angle: 0 },
        ],
      };

      const statusP1Conn1 = getConnectorSnapStatus(p1, 1, [p1, p2], layers);
      expect(statusP1Conn1.status).toBe("connected");
      expect(statusP1Conn1.color).toBe("grey");
      expect(statusP1Conn1.connectedTo?.pieceId).toBe("p2");

      const statusP2Conn0 = getConnectorSnapStatus(p2, 0, [p1, p2], layers);
      expect(statusP2Conn0.status).toBe("connected");
      expect(statusP2Conn0.color).toBe("grey");

      // Freier äußerer Andockpunkt bleibt grün
      const statusP1Conn0 = getConnectorSnapStatus(p1, 0, [p1, p2], layers);
      expect(statusP1Conn0.status).toBe("free");
      expect(statusP1Conn0.color).toBe("green");
    });

    it("stellt gelben Snap-Punkt dar, wenn ein Gleis über einem Andockpunkt liegt ('status: above', 'color: yellow')", () => {
      // Unteres Gleis auf Grundebene (Höhe 0) mit Connector bei (188.3, 0)
      const groundTrack: TrackPiece = {
        id: "ground-track",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      // Brückengleis auf Brückenebene (Höhe 80), verläuft senkrecht über (188.3, 0)
      const bridgeTrack: TrackPiece = {
        id: "bridge-track",
        catalogId: "24188",
        transform: { x: 188.3, y: -94.15, rotation: 90 },
        layerId: "bridge",
        connectors: [
          { x: 188.3, y: -94.15, angle: 270 },
          { x: 188.3, y: 94.15, angle: 90 },
        ],
      };

      const status = getConnectorSnapStatus(groundTrack, 1, [groundTrack, bridgeTrack], layers);
      expect(status.status).toBe("above");
      expect(status.color).toBe("yellow");
      expect(status.abovePieceId).toBe("bridge-track");
    });

    it("verhindert Snapping bei unterschiedlicher Höhe ('connection should be on the same hight')", () => {
      const fixedGroundTrack: TrackPiece = {
        id: "fixed-ground",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const bridgeCandidatePiece: TrackPiece = {
        id: "moving-bridge",
        catalogId: "24188",
        transform: { x: 190, y: 2, rotation: 5 },
        layerId: "bridge", // Höhe 80 mm!
        connectors: [
          { x: 190, y: 2, angle: 185 },
          { x: 378.3, y: 2, angle: 5 },
        ],
      };

      // Versuch, Brückengleis (80mm) an Erdgleis (0mm) zu snappen
      const snapDifferentHeight = findBestSnapForPiece(
        g188,
        bridgeCandidatePiece.transform,
        [fixedGroundTrack],
        undefined,
        45,
        60,
        undefined,
        bridgeCandidatePiece,
        layers,
      );
      // Darf NICHT einrasten!
      expect(snapDifferentHeight).toBeNull();

      // Gleicher Versuch mit einem Erdgleis (0mm an 0mm)
      const groundCandidatePiece: TrackPiece = {
        id: "moving-ground",
        catalogId: "24188",
        transform: { x: 190, y: 2, rotation: 5 },
        layerId: "ground", // Höhe 0 mm!
        connectors: [
          { x: 190, y: 2, angle: 185 },
          { x: 378.3, y: 2, angle: 5 },
        ],
      };
      const snapSameHeight = findBestSnapForPiece(
        g188,
        groundCandidatePiece.transform,
        [fixedGroundTrack],
        undefined,
        45,
        60,
        undefined,
        groundCandidatePiece,
        layers,
      );
      // Rastet erfolgreich ein!
      expect(snapSameHeight).not.toBeNull();
      expect(snapSameHeight!.snappedTransform.x).toBeCloseTo(188.3, 2);
    });

    it("findSnapAboveForPiece erkennt, wenn ein bewegtes Gleis über einem Andockpunkt liegt", () => {
      const fixedGroundTrack: TrackPiece = {
        id: "fixed-ground",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const bridgeCandidatePiece: TrackPiece = {
        id: "moving-bridge",
        catalogId: "24188",
        transform: { x: 190, y: 2, rotation: 0 },
        layerId: "bridge", // Höhe 80 mm
        connectors: [],
      };

      const snapAbove = findSnapAboveForPiece(
        g188,
        bridgeCandidatePiece.transform,
        [fixedGroundTrack],
        bridgeCandidatePiece,
        layers,
        35,
      );

      expect(snapAbove).not.toBeNull();
      expect(snapAbove?.targetPieceId).toBe("fixed-ground");
      expect(snapAbove?.targetConnectorIndex).toBe(1);
      expect(snapAbove?.movingHeight).toBe(80);
      expect(snapAbove?.targetHeight).toBe(0);
    });

    it("updatePieceConnections aktualisiert connectedTo für Gleise gleicher Höhe und ignoriert unterschiedliche Höhen", () => {
      const p1: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };
      const p2: TrackPiece = {
        id: "p2",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        layerId: "ground",
        connectors: [
          { x: 188.3, y: 0, angle: 180 },
          { x: 376.6, y: 0, angle: 0 },
        ],
      };
      const pBridge: TrackPiece = {
        id: "pBridge",
        catalogId: "24188",
        transform: { x: 376.6, y: 0, rotation: 0 },
        layerId: "bridge", // 80 mm!
        connectors: [
          { x: 376.6, y: 0, angle: 180 },
          { x: 564.9, y: 0, angle: 0 },
        ],
      };

      updatePieceConnections([p1, p2, pBridge], layers);

      // p1 und p2 sind verbunden (Höhe 0 == 0)
      expect(p1.connectors[1]!.connectedTo).toEqual({ pieceId: "p2", connectorIndex: 0 });
      expect(p2.connectors[0]!.connectedTo).toEqual({ pieceId: "p1", connectorIndex: 1 });

      // p2 und pBridge sind NICHT verbunden (Höhe 0 != 80)
      expect(p2.connectors[1]!.connectedTo).toBeUndefined();
      expect(pBridge.connectors[0]!.connectedTo).toBeUndefined();
    });
  });
});
