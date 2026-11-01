import { describe, it, expect } from "vitest";
import { findSnap } from "@/core/snapping";
import type { TrackPiece } from "@/core/types";
import { distanceToTrackPiece, isPieceInSelectionBox } from "@/core/geometry";
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

  it("umfasst das vollständige Märklin C-Gleis Sortiment mit 48 Gleisen", () => {
    expect(maerklinCCatalog.entries.length).toBe(48);

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

    const spriteOff = createTrackGraphics(piece, g188, { colorMode: "off" });
    expect(spriteOff).toBeDefined();

    const spriteTypes = createTrackGraphics(piece, g188, { colorMode: "types" });
    expect(spriteTypes).toBeDefined();

    const spriteLayers = createTrackGraphics(piece, g188, {
      colorMode: "layers",
      layerColor: "#ec4899",
    });
    expect(spriteLayers).toBeDefined();
  });
});
