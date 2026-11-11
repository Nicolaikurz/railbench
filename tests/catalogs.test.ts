import { describe, it, expect } from "vitest";
import {
  catalogRegistry,
  maerklinMCatalog,
  maerklinKCatalog,
  genericStreetsCatalog,
  computeWorldConnectors,
} from "@/catalog/catalog";
import { getPieceListSummary, exportPieceListCSV } from "@/io/export";
import type { Layout } from "@/core/types";

describe("Märklin M-Gleis & K-Gleis Catalogs", () => {
  it("registers Märklin M-Gleis and K-Gleis in the CatalogRegistry", () => {
    const all = catalogRegistry.getAll();
    const mCat = catalogRegistry.get("maerklin-m-h0");
    const kCat = catalogRegistry.get("maerklin-k-h0");

    expect(mCat).toBeDefined();
    expect(mCat?.name).toBe("Märklin H0 M-Gleis");
    expect(mCat?.scale).toBe("H0");
    expect(mCat?.gauge).toBe(16.5);
    expect(mCat?.hasRoadbed).toBe(true);
    expect(mCat?.entries.length).toBeGreaterThan(20);

    expect(kCat).toBeDefined();
    expect(kCat?.name).toBe("Märklin H0 K-Gleis");
    expect(kCat?.scale).toBe("H0");
    expect(kCat?.gauge).toBe(16.5);
    expect(kCat?.hasRoadbed).toBe(false);
    expect(kCat?.entries.length).toBeGreaterThan(25);
  });

  it("sorts catalog entries in ascending numerical order", () => {
    const cCat = catalogRegistry.get("maerklin-c-h0");
    expect(cCat).toBeDefined();
    for (let i = 0; i < cCat!.entries.length - 1; i++) {
      const a = cCat!.entries[i]!.id;
      const b = cCat!.entries[i + 1]!.id;
      expect(a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" })).toBeLessThanOrEqual(0);
    }

    const kCat = catalogRegistry.get("maerklin-k-h0");
    expect(kCat).toBeDefined();
    for (let i = 0; i < kCat!.entries.length - 1; i++) {
      const a = kCat!.entries[i]!.id;
      const b = kCat!.entries[i + 1]!.id;
      expect(a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" })).toBeLessThanOrEqual(0);
    }
  });

  it("finds specific M-Gleis entries and verifies geometry", () => {
    // 5106 standard straight (180 mm)
    const t5106 = catalogRegistry.findEntry("5106");
    expect(t5106).toBeDefined();
    expect(t5106?.length).toBe(180.0);
    expect(t5106?.kind).toBe("straight");

    // 5100 standard curve (R1 360 mm, 30 deg)
    const t5100 = catalogRegistry.findEntry("5100");
    expect(t5100).toBeDefined();
    expect(t5100?.radius).toBe(360.0);
    expect(t5100?.angle).toBe(30.0);
    expect(t5100?.kind).toBe("curve");

    // 5200 parallel curve (R2 437.4 mm, 30 deg)
    const t5200 = catalogRegistry.findEntry("5200");
    expect(t5200).toBeDefined();
    expect(t5200?.radius).toBeCloseTo(437.4, 1);
    expect(t5200?.angle).toBe(30.0);

    // 5117-L turnout
    const t5117L = catalogRegistry.findEntry("5117-L");
    expect(t5117L).toBeDefined();
    expect(t5117L?.length).toBe(180.0);
    expect(t5117L?.branchRadius).toBe(360.0);
    expect(t5117L?.branchAngle).toBe(30.0);
    expect(t5117L?.kind).toBe("turnout-left");

    // 5207 DKW / Double slip
    const t5207 = catalogRegistry.findEntry("5207");
    expect(t5207).toBeDefined();
    expect(t5207?.length).toBe(180.0);
    expect(t5207?.kind).toBe("double-slip");

    // 72941 Schiebebühne
    const t72941 = catalogRegistry.findEntry("72941");
    expect(t72941).toBeDefined();
    expect(t72941?.name).toContain("72941");
    expect(t72941?.kind).toBe("transfer-table");
    expect(t72941?.length).toBe(360);
    expect(t72941?.gauge).toBe(16.5);
    expect(t72941?.localConnectors).toHaveLength(10);
    const leftConnectors = t72941?.localConnectors?.filter((c) => c.x === 0);
    const rightConnectors = t72941?.localConnectors?.filter((c) => c.x === 360);
    expect(leftConnectors).toHaveLength(5);
    expect(rightConnectors).toHaveLength(5);
    expect(leftConnectors?.map((c) => c.y)).toEqual([0, 77.4, 154.8, 232.2, 309.6]);
    expect(rightConnectors?.map((c) => c.y)).toEqual([0, 77.4, 154.8, 232.2, 309.6]);
    const worldConnectors = computeWorldConnectors(t72941, { x: 100, y: 200, rotation: 0 });
    expect(worldConnectors).toHaveLength(10);
    expect(worldConnectors[0]).toEqual({ x: 100, y: 200, angle: 180 });
    expect(worldConnectors[5]).toEqual({ x: 460, y: 200, angle: 0 });
  });

  it("finds specific K-Gleis entries and verifies geometry", () => {
    // 2200 standard straight (180 mm)
    const t2200 = catalogRegistry.findEntry("2200");
    expect(t2200).toBeDefined();
    expect(t2200?.length).toBe(180.0);
    expect(t2200?.hasRoadbed).toBe(false);

    // 2205 flex track (900 mm)
    const t2205 = catalogRegistry.findEntry("2205");
    expect(t2205).toBeDefined();
    expect(t2205?.length).toBe(900.0);

    // 2210 curve R1 (360 mm, 30 deg)
    const t2210 = catalogRegistry.findEntry("2210");
    expect(t2210).toBeDefined();
    expect(t2210?.radius).toBe(360.0);
    expect(t2210?.angle).toBe(30.0);

    // 22711 slim turnout (225 mm, R 902.4 mm, 14.43 deg)
    const t22711 = catalogRegistry.findEntry("22711");
    expect(t22711).toBeDefined();
    expect(t22711?.length).toBe(225.0);
    expect(t22711?.branchRadius).toBeCloseTo(902.4, 1);
    expect(t22711?.branchAngle).toBeCloseTo(14.43, 2);
    expect(t22711?.kind).toBe("turnout-left");

    // 2275 slim DKW / Double slip
    const t2275 = catalogRegistry.findEntry("2275");
    expect(t2275).toBeDefined();
    expect(t2275?.length).toBe(225.0);
    expect(t2275?.kind).toBe("double-slip");
  });

  it("computes world connectors accurately for M-Gleis and K-Gleis pieces", () => {
    const straightM = maerklinMCatalog.entries.find((e) => e.id === "5106")!;
    const connsM = computeWorldConnectors(straightM, { x: 100, y: 50, rotation: 0 });
    expect(connsM.length).toBe(2);
    expect(connsM[0]!.x).toBeCloseTo(100, 1);
    expect(connsM[0]!.y).toBeCloseTo(50, 1);
    expect(connsM[1]!.x).toBeCloseTo(280, 1); // 100 + 180
    expect(connsM[1]!.y).toBeCloseTo(50, 1);

    const turnoutK = maerklinKCatalog.entries.find((e) => e.id === "22711")!;
    const connsK = computeWorldConnectors(turnoutK, { x: 0, y: 0, rotation: 0 });
    expect(connsK.length).toBe(3);
    // Main straight endpoint
    expect(connsK[1]!.x).toBeCloseTo(225.0, 1);
    expect(connsK[1]!.y).toBeCloseTo(0, 1);
    // Diverging branch endpoint
    expect(connsK[2]!.x).toBeGreaterThan(200);
    expect(connsK[2]!.y).toBeGreaterThan(10);
  });
});

describe("Generic Streets Catalog (H0 Roads)", () => {
  it("registers generic streets in the CatalogRegistry", () => {
    const roadCat = catalogRegistry.get("generic-streets-h0");
    expect(roadCat).toBeDefined();
    expect(roadCat?.isRoadSystem).toBe(true);
    expect(roadCat?.name).toContain("Straßensystem");
    expect(roadCat?.entries.length).toBe(13);
  });

  it("verifies standard road dimensions and properties", () => {
    for (const entry of genericStreetsCatalog.entries) {
      expect(entry.isRoad).toBe(true);
      expect(entry.roadWidth).toBe(60);
      expect(entry.localConnectors).toBeDefined();
      expect(entry.localConnectors!.length).toBeGreaterThanOrEqual(1);
    }
  });

  it("verifies straight roads, curves, T-junction, crossroad and cul-de-sac geometry", () => {
    // S200 straight
    const s200 = catalogRegistry.findEntry("ST-S200");
    expect(s200).toBeDefined();
    expect(s200?.length).toBe(200);
    expect(s200?.kind).toBe("straight");

    // S100 straight
    const s100 = catalogRegistry.findEntry("ST-S100");
    expect(s100).toBeDefined();
    expect(s100?.length).toBe(100);

    // S050 straight
    const s050 = catalogRegistry.findEntry("ST-S050");
    expect(s050).toBeDefined();
    expect(s050?.length).toBe(50);

    // C200-90 90-degree curve
    const c90 = catalogRegistry.findEntry("ST-C200-90");
    expect(c90).toBeDefined();
    expect(c90?.radius).toBe(200);
    expect(c90?.angle).toBe(90);
    expect(c90?.kind).toBe("curve");

    // TJ-90 T-junction
    const tj90 = catalogRegistry.findEntry("ST-TJ-90");
    expect(tj90).toBeDefined();
    expect(tj90?.length).toBe(200);
    expect(tj90?.branchLength).toBe(100);
    expect(tj90?.branchAngle).toBe(90);
    expect(tj90?.localConnectors?.length).toBe(3);

    // X-90 4-way crossroad
    const x90 = catalogRegistry.findEntry("ST-X-90");
    expect(x90).toBeDefined();
    expect(x90?.length).toBe(200);
    expect(x90?.angle).toBe(90);
    expect(x90?.kind).toBe("crossing");
    expect(x90?.localConnectors?.length).toBe(4);

    // ST-LOOP turning loop
    const loop = catalogRegistry.findEntry("ST-LOOP");
    expect(loop).toBeDefined();
    expect(loop?.kind).toBe("accessory");
    expect(loop?.localConnectors?.length).toBe(1);

    // ST-BUS bus stop
    const bus = catalogRegistry.findEntry("ST-BUS");
    expect(bus).toBeDefined();
    expect(bus?.length).toBe(200);

    // ST-CROSS rail crossing
    const cross = catalogRegistry.findEntry("ST-CROSS");
    expect(cross).toBeDefined();
    expect(cross?.length).toBe(200);
  });

  it("computes accurate world connectors for road pieces", () => {
    const s200 = genericStreetsCatalog.entries.find((e) => e.id === "ST-S200")!;
    const conns = computeWorldConnectors(s200, { x: 50, y: 100, rotation: 0 });
    expect(conns.length).toBe(2);
    expect(conns[0]!.x).toBeCloseTo(50, 1);
    expect(conns[0]!.y).toBeCloseTo(100, 1);
    expect(conns[1]!.x).toBeCloseTo(250, 1);
    expect(conns[1]!.y).toBeCloseTo(100, 1);

    const tj = genericStreetsCatalog.entries.find((e) => e.id === "ST-TJ-90")!;
    const tjConns = computeWorldConnectors(tj, { x: 0, y: 0, rotation: 0 });
    expect(tjConns.length).toBe(3);
    // Main road endpoints
    expect(tjConns[0]!.x).toBeCloseTo(0, 1);
    expect(tjConns[0]!.y).toBeCloseTo(0, 1);
    expect(tjConns[1]!.x).toBeCloseTo(200, 1);
    expect(tjConns[1]!.y).toBeCloseTo(0, 1);
    // 90° Branch endpoint
    expect(tjConns[2]!.x).toBeCloseTo(100, 1);
    expect(tjConns[2]!.y).toBeCloseTo(100, 1);
  });

  it("ensures all street pieces are identified with isRoad in catalog lookup", () => {
    const ids = [
      "ST-S200", "ST-S100", "ST-S050", "ST-FLEX",
      "ST-C200-90", "ST-C200-45", "ST-C300-45",
      "ST-TJ-90", "ST-X-90", "ST-Y-45",
      "ST-LOOP", "ST-BUS", "ST-CROSS"
    ];
    for (const id of ids) {
      const entry = catalogRegistry.findEntry(id);
      expect(entry, `Street entry ${id} should be found`).toBeDefined();
      expect(entry?.isRoad, `Street entry ${id} must have isRoad: true`).toBe(true);
      expect(entry?.roadWidth, `Street entry ${id} must have roadWidth 60`).toBe(60);
    }
  });

  it("provides flex streets across H0, TT, N, Z, 0, 1, G scales", () => {
    const scales = [
      { id: "ST-FLEX", scale: "H0", len: 800, width: 60 },
      { id: "ST-TT-FLEX", scale: "TT", len: 600, width: 44 },
      { id: "ST-N-FLEX", scale: "N", len: 500, width: 35 },
      { id: "ST-Z-FLEX", scale: "Z", len: 350, width: 24 },
      { id: "ST-0-FLEX", scale: "0", len: 1200, width: 115 },
      { id: "ST-1-FLEX", scale: "1", len: 1500, width: 160 },
      { id: "ST-G-FLEX", scale: "G", len: 1500, width: 230 },
    ];
    for (const s of scales) {
      const entry = catalogRegistry.findEntry(s.id);
      expect(entry, `Flex street ${s.id} should exist`).toBeDefined();
      expect(entry?.kind).toBe("flex");
      expect(entry?.isRoad).toBe(true);
      expect(entry?.length).toBe(s.len);
      expect(entry?.roadWidth).toBe(s.width);
      expect(entry?.category).toBe("Geraden");
      expect(entry?.localConnectors?.length).toBe(2);
    }
  });

  it("separates parts list into distinct tracks, roads and buildings", () => {
    const layout: Layout = {
      name: "Mixed Layout",
      scale: "H0",
      pieces: [
        { id: "p1", catalogId: "5106", transform: { x: 0, y: 0, rotation: 0 }, layerId: "layer-1", connectors: [] },
        { id: "p2", catalogId: "5106", transform: { x: 180, y: 0, rotation: 0 }, layerId: "layer-1", connectors: [] },
        { id: "p3", catalogId: "ST-S200", transform: { x: 0, y: 200, rotation: 0 }, layerId: "layer-1", connectors: [] },
        { id: "p4", catalogId: "ST-S200", transform: { x: 200, y: 200, rotation: 0 }, layerId: "layer-1", connectors: [] },
        { id: "p5", catalogId: "ST-C200-90", transform: { x: 400, y: 200, rotation: 0 }, layerId: "layer-1", connectors: [] },
      ],
      models: [
        {
          id: "m1",
          name: "Bahnhof Waldbrunn",
          category: "buildings",
          transform: { x: 100, y: 100, rotation: 0 },
          width: 300,
          depth: 140,
          height: 120,
          modelType: "builtin",
        },
      ],
    };

    const summary = getPieceListSummary(layout, maerklinMCatalog);

    // Total pieces and total objects
    expect(summary.totalPieces).toBe(5);
    expect(summary.totalObjects).toBe(6);

    // Tracks
    expect(summary.tracks.totalPieces).toBe(2);
    expect(summary.tracks.items.length).toBe(1);
    expect(summary.tracks.items[0]!.catalogId).toBe("5106");
    expect(summary.tracks.items[0]!.count).toBe(2);
    expect(summary.tracks.items[0]!.itemType).toBe("track");

    // Roads
    expect(summary.roads.totalPieces).toBe(3);
    expect(summary.roads.items.length).toBe(2);
    expect(summary.roads.items.some((it) => it.catalogId === "ST-S200")).toBe(true);
    expect(summary.roads.items.some((it) => it.catalogId === "ST-C200-90")).toBe(true);
    expect(summary.roads.items[0]!.itemType).toBe("road");

    // Buildings
    expect(summary.buildings.totalPieces).toBe(1);
    expect(summary.buildings.items.length).toBe(1);
    expect(summary.buildings.items[0]!.name).toBe("Bahnhof Waldbrunn");
    expect(summary.buildings.items[0]!.count).toBe(1);

    // CSV Export includes separate sections
    const csv = exportPieceListCSV(layout, maerklinMCatalog);
    expect(csv).toContain("Railway Tracks");
    expect(csv).toContain("Roads & Streets");
    expect(csv).toContain("Buildings & 3D Objects");
  });

  it("registers generic streets N catalog and verifies N scale geometry", () => {
    const nStreetCat = catalogRegistry.get("generic-streets-n");
    expect(nStreetCat).toBeDefined();
    expect(nStreetCat?.scale).toBe("N");
    expect(nStreetCat?.gauge).toBe(9);
    expect(nStreetCat?.isRoadSystem).toBe(true);
    expect(nStreetCat?.entries.length).toBeGreaterThanOrEqual(10);

    const s110 = nStreetCat?.entries.find((e) => e.id === "ST-N-S110");
    expect(s110).toBeDefined();
    expect(s110?.length).toBe(110);
    expect(s110?.roadWidth).toBe(35);
    expect(s110?.isRoad).toBe(true);

    const c110 = nStreetCat?.entries.find((e) => e.id === "ST-N-C110-90");
    expect(c110).toBeDefined();
    expect(c110?.radius).toBe(110);
    expect(c110?.angle).toBe(90);
    expect(c110?.roadWidth).toBe(35);
  });

  it("verifies that every entry in every catalog has a valid track gauge", () => {
    const all = catalogRegistry.getAll();
    expect(all.length).toBeGreaterThanOrEqual(20);

    for (const cat of all) {
      expect(cat.gauge).toBeGreaterThan(0);
      for (const entry of cat.entries) {
        expect(entry.gauge).toBeDefined();
        expect(entry.gauge).toBe(cat.gauge);
      }
    }
  });

  it("imports a custom track catalog from JSON and verifies rendering parameters and connectors", () => {
    const customJson = JSON.stringify({
      id: "custom-garden-g",
      name: "Gartenbahn G Custom",
      manufacturer: "Eigenbau",
      system: "Spur G",
      scale: "G",
      gauge: 45.0,
      hasRoadbed: false,
      entries: [
        {
          id: "G-S300",
          name: "Gerade 300 mm",
          kind: "straight",
          length: 300,
          gauge: 45.0,
        },
        {
          id: "G-R600",
          name: "Bogen R600 30°",
          kind: "curve",
          radius: 600,
          angle: 30,
        },
        {
          id: "G-WL600",
          name: "Weiche links R600 30°",
          kind: "turnout-left",
          length: 300,
          branchRadius: 600,
          branchAngle: 30,
        },
      ],
    });

    const imported = catalogRegistry.loadFromJSON(customJson);
    expect(imported).toBeDefined();
    expect(imported.id).toBe("custom-garden-g");
    expect(imported.gauge).toBe(45.0);
    expect(imported.isCustom).toBe(true);
    expect(imported.entries.length).toBe(3);

    // Verify gauge was propagated to all items
    for (const item of imported.entries) {
      expect(item.gauge).toBe(45.0);
    }

    // Verify findEntry works
    const foundStraight = catalogRegistry.findEntry("G-S300");
    expect(foundStraight).toBeDefined();
    expect(foundStraight?.gauge).toBe(45.0);
    expect(foundStraight?.length).toBe(300);

    // Verify connectors calculation for custom turnout
    const foundTurnout = catalogRegistry.findEntry("G-WL600")!;
    expect(foundTurnout).toBeDefined();
    const connectors = computeWorldConnectors(foundTurnout, { x: 0, y: 0, rotation: 0 });
    expect(connectors.length).toBe(3);
    expect(connectors[0]?.angle).toBe(180);
    expect(connectors[1]?.x).toBe(300);
    expect(connectors[1]?.angle).toBe(0);
    expect(connectors[2]?.angle).toBe(30);
  });

  it("imports a flat list of tracks as JSON and auto-wraps it into a valid catalog", () => {
    const flatListJson = JSON.stringify([
      { id: "FLAT-1", name: "Short piece", kind: "straight", length: 50 },
      { id: "FLAT-2", name: "Curve piece", kind: "curve", radius: 400, angle: 45 },
    ]);

    const imported = catalogRegistry.loadFromJSON(flatListJson);
    expect(imported.entries.length).toBe(2);
    expect(imported.gauge).toBe(16.5);
    expect(imported.entries[0]?.gauge).toBe(16.5);
    expect(imported.entries[1]?.gauge).toBe(16.5);
    expect(imported.isCustom).toBe(true);
  });

  it("handles German aliases, string numbers with commas, and UTF-8 BOM", () => {
    const germanJson = `\uFEFF{
      "hersteller": "Musterbahn",
      "name": "Musterbahn Gleise",
      "spurweite": "16,5",
      "gleise": [
        { "artnr": "M-101", "bezeichnung": "Gerade 200", "typ": "gerade", "laenge": "200,5" },
        { "artnr": "M-102", "bezeichnung": "Kurve R400", "typ": "kurve", "radius": "400", "winkel": "30" },
        { "artnr": "M-103", "bezeichnung": "Weiche links", "typ": "weiche links", "laenge": "200", "abzweigradius": "400", "abzweigwinkel": "30" }
      ]
    }`;

    const imported = catalogRegistry.loadFromJSON(germanJson);
    expect(imported.manufacturer).toBe("Musterbahn");
    expect(imported.gauge).toBe(16.5);
    expect(imported.entries.length).toBe(3);

    const s = imported.entries[0]!;
    expect(s.id).toBe("M-101");
    expect(s.kind).toBe("straight");
    expect(s.length).toBe(200.5);
    expect(s.gauge).toBe(16.5);

    const c = imported.entries[1]!;
    expect(c.id).toBe("M-102");
    expect(c.kind).toBe("curve");
    expect(c.radius).toBe(400);
    expect(c.angle).toBe(30);

    const w = imported.entries[2]!;
    expect(w.id).toBe("M-103");
    expect(w.kind).toBe("turnout-left");
    expect(w.branchRadius).toBe(400);
    expect(w.branchAngle).toBe(30);
  });

  it("detects layout plans and throws a descriptive error", () => {
    const layoutJson = JSON.stringify({
      name: "Mein Layout",
      version: 1,
      pieces: [
        { id: "p1", catalogId: "24188" }
      ]
    });

    expect(() => catalogRegistry.loadFromJSON(layoutJson)).toThrow(/Gleisplan/);
  });
});

describe("Scale 0, 1 & Scale G Tracks, Streets, and Buildings", () => {
  it("registers Lenz Spur 0 track system with 32 mm gauge", () => {
    const l0Cat = catalogRegistry.get("lenz-0");
    expect(l0Cat).toBeDefined();
    expect(l0Cat?.manufacturer).toBe("Lenz");
    expect(l0Cat?.scale).toBe("0");
    expect(l0Cat?.gauge).toBe(32.0);
    expect(l0Cat?.entries.length).toBeGreaterThanOrEqual(15);

    const g1 = catalogRegistry.findEntry("45010");
    expect(g1).toBeDefined();
    expect(g1?.length).toBeCloseTo(440.74, 2);
    expect(g1?.gauge).toBe(32.0);
    expect(g1?.kind).toBe("straight");

    const r1 = catalogRegistry.findEntry("45021");
    expect(r1).toBeDefined();
    expect(r1?.radius).toBe(914.4);
    expect(r1?.angle).toBe(22.5);

    const wl = catalogRegistry.findEntry("45030");
    expect(wl).toBeDefined();
    expect(wl?.kind).toBe("turnout-left");
    expect(wl?.branchRadius).toBe(1649.39);
    expect(wl?.branchAngle).toBe(11.25);

    const dkw = catalogRegistry.findEntry("45040");
    expect(dkw).toBeDefined();
    expect(dkw?.kind).toBe("double-slip");
  });

  it("registers Märklin Spur 1 track system with 45 mm gauge", () => {
    const m1Cat = catalogRegistry.get("maerklin-1");
    expect(m1Cat).toBeDefined();
    expect(m1Cat?.manufacturer).toBe("Märklin");
    expect(m1Cat?.scale).toBe("1");
    expect(m1Cat?.gauge).toBe(45.0);
    expect(m1Cat?.entries.length).toBeGreaterThanOrEqual(20);

    const s300 = catalogRegistry.findEntry("5900");
    expect(s300).toBeDefined();
    expect(s300?.length).toBe(300);
    expect(s300?.gauge).toBe(45.0);
    expect(s300?.kind).toBe("straight");

    const r1020 = catalogRegistry.findEntry("5922");
    expect(r1020).toBeDefined();
    expect(r1020?.radius).toBe(1020);
    expect(r1020?.angle).toBe(30);

    const w1020 = catalogRegistry.findEntry("5972");
    expect(w1020).toBeDefined();
    expect(w1020?.kind).toBe("turnout-left");
    expect(w1020?.branchRadius).toBe(1020);
    expect(w1020?.branchAngle).toBe(22.5);

    const dkw = catalogRegistry.findEntry("5974");
    expect(dkw).toBeDefined();
    expect(dkw?.kind).toBe("double-slip");
  });

  it("registers LGB Spur G track system with 45 mm gauge", () => {
    const lgbCat = catalogRegistry.get("lgb-g");
    expect(lgbCat).toBeDefined();
    expect(lgbCat?.manufacturer).toBe("LGB");
    expect(lgbCat?.scale).toBe("G");
    expect(lgbCat?.gauge).toBe(45.0);
    expect(lgbCat?.entries.length).toBeGreaterThanOrEqual(20);

    const s300 = catalogRegistry.findEntry("10000");
    expect(s300).toBeDefined();
    expect(s300?.length).toBe(300);
    expect(s300?.gauge).toBe(45.0);
    expect(s300?.kind).toBe("straight");

    const r600 = catalogRegistry.findEntry("11000");
    expect(r600).toBeDefined();
    expect(r600?.radius).toBe(600);
    expect(r600?.angle).toBe(30);

    const w600 = catalogRegistry.findEntry("12000");
    expect(w600).toBeDefined();
    expect(w600?.kind).toBe("turnout-left");
    expect(w600?.branchRadius).toBe(600);
    expect(w600?.branchAngle).toBe(30);

    const r3w = catalogRegistry.findEntry("16050");
    expect(r3w).toBeDefined();
    expect(r3w?.branchRadius).toBe(1175);
    expect(r3w?.branchAngle).toBe(22.5);
  });

  it("registers generic streets catalogs for Scale 0 (1:45), Scale 1 (1:32) and Scale G (1:22.5)", () => {
    const st0Cat = catalogRegistry.get("generic-streets-0");
    expect(st0Cat).toBeDefined();
    expect(st0Cat?.scale).toBe("0");
    expect(st0Cat?.gauge).toBe(32);
    expect(st0Cat?.isRoadSystem).toBe(true);
    for (const e of st0Cat!.entries) {
      expect(e.isRoad).toBe(true);
      expect(e.roadWidth).toBe(115);
      expect(e.gauge).toBe(32);
    }

    const st1Cat = catalogRegistry.get("generic-streets-1");
    expect(st1Cat).toBeDefined();
    expect(st1Cat?.scale).toBe("1");
    expect(st1Cat?.gauge).toBe(45);
    expect(st1Cat?.isRoadSystem).toBe(true);
    for (const e of st1Cat!.entries) {
      expect(e.isRoad).toBe(true);
      expect(e.roadWidth).toBe(160);
      expect(e.gauge).toBe(45);
    }

    const stGCat = catalogRegistry.get("generic-streets-g");
    expect(stGCat).toBeDefined();
    expect(stGCat?.scale).toBe("G");
    expect(stGCat?.gauge).toBe(45);
    expect(stGCat?.isRoadSystem).toBe(true);
    for (const e of stGCat!.entries) {
      expect(e.isRoad).toBe(true);
      expect(e.roadWidth).toBe(230);
      expect(e.gauge).toBe(45);
    }
  });
});

