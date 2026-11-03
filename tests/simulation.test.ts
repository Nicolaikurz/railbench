import { describe, it, expect } from "vitest";
import { simulatePowerDrop, isTurnoutEntry, detectShortCircuit, isTwoWireCatalog } from "@/core/simulation";
import { maerklinCCatalog, trixCCatalog, pikoACatalog } from "@/catalog/catalog";
import type { Layout, TrackPiece } from "@/core/types";

describe("Power Drop Simulation", () => {
  it("berechnet Spannungsabfall für Gleise mit Einspeisung", () => {
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      feedVoltage: 16.0,
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [
            { x: 0, y: 0, angle: 180 },
            { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
          ],
          powerFeed: true,
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [
            { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
            { x: 376.6, y: 0, angle: 0 },
          ],
        },
      ],
    };

    const sim = simulatePowerDrop(layout, maerklinCCatalog);
    expect(sim.feederCount).toBe(1);
    expect(sim.results.get("p1")?.voltage).toBeCloseTo(16.0, 1);
    expect(sim.results.get("p2")?.status).toBe("optimal");
  });

  it("erkennt nicht angeschlossene Gleise", () => {
    const layout: Layout = {
      name: "Insel Gleis",
      scale: "H0",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
          powerFeed: true,
        },
        {
          id: "isolated",
          catalogId: "24188",
          transform: { x: 1000, y: 1000, rotation: 0 },
          connectors: [{ x: 1000, y: 1000, angle: 180 }, { x: 1188.3, y: 1000, angle: 0 }],
        },
      ],
    };

    const sim = simulatePowerDrop(layout, maerklinCCatalog);
    expect(sim.disconnectedCount).toBe(1);
    expect(sim.results.get("isolated")?.status).toBe("disconnected");
  });

  it("isoliert analoge Streckenabschnitte über Spannungstrennstellen (isIsolator)", () => {
    // Kette p1 (Feeder) -> p2 (Isolator) -> p3 (Hinter Trennstelle)
    const layout: Layout = {
      name: "Analog Block Layout",
      scale: "H0",
      feedVoltage: 14.0,
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [
            { x: 0, y: 0, angle: 180 },
            { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
          ],
          powerFeed: true,
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [
            { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
            { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "p3", connectorIndex: 0 } },
          ],
          isIsolator: true, // Trennstelle: empfängt keinen Strom aus p1
        },
        {
          id: "p3",
          catalogId: "24188",
          transform: { x: 376.6, y: 0, rotation: 0 },
          connectors: [
            { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "p2", connectorIndex: 1 } },
            { x: 564.9, y: 0, angle: 0 },
          ],
        },
      ],
    };

    const sim = simulatePowerDrop(layout, maerklinCCatalog);
    expect(sim.feederCount).toBe(1);
    expect(sim.isolatorCount).toBe(1);
    expect(sim.results.get("p1")?.voltage).toBeCloseTo(14.0, 1);
    // p2 ist eine Trennstelle und wird von p1 isoliert -> disconnected (0V)
    expect(sim.results.get("p2")?.status).toBe("disconnected");
    expect(sim.results.get("p2")?.voltage).toBe(0);
    // p3 liegt hinter der Trennstelle und hat keinen eigenen Feeder -> disconnected (0V)
    expect(sim.results.get("p3")?.status).toBe("disconnected");
    expect(sim.results.get("p3")?.voltage).toBe(0);
    expect(sim.disconnectedCount).toBe(2);
  });

  it("erlaubt unabhängige Stromkreise mit eigenem Feeder auf isolierten Abschnitten", () => {
    // p1 (Hauptkreis Feeder 16V) | Trennstelle | p2 (Schattenbahnhof Feeder 12V) -> p3
    const layout: Layout = {
      name: "Two Independent Circuits",
      scale: "H0",
      feedVoltage: 16.0,
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [
            { x: 0, y: 0, angle: 180 },
            { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
          ],
          powerFeed: true,
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [
            { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
            { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "p3", connectorIndex: 0 } },
          ],
          isIsolator: true,
          powerFeed: true, // Eigener Feeder für den isolierten Bereich
        },
        {
          id: "p3",
          catalogId: "24188",
          transform: { x: 376.6, y: 0, rotation: 0 },
          connectors: [
            { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "p2", connectorIndex: 1 } },
            { x: 564.9, y: 0, angle: 0 },
          ],
        },
      ],
    };

    const sim = simulatePowerDrop(layout, maerklinCCatalog);
    expect(sim.feederCount).toBe(2);
    expect(sim.isolatorCount).toBe(1);
    expect(sim.results.get("p1")?.status).toBe("optimal");
    expect(sim.results.get("p2")?.status).toBe("optimal");
    expect(sim.results.get("p3")?.status).toBe("optimal");
    expect(sim.disconnectedCount).toBe(0);
  });

  it("isoliert nur an Anschluss 1 wenn isolatedConnectorIndex = 1 gesetzt ist", () => {
    // p1 (Feeder) -> p2 (Anschluss 0 leitet, Anschluss 1 isoliert) -> p3 (getrennt)
    const layout: Layout = {
      name: "Cut at Connector 1",
      scale: "H0",
      feedVoltage: 16.0,
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [
            { x: 0, y: 0, angle: 180 },
            { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
          ],
          powerFeed: true,
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [
            { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
            { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "p3", connectorIndex: 0 } },
          ],
          isIsolator: true,
          isolatedConnectorIndex: 1, // Trennung liegt am Ende zu p3, p2 selbst hat Strom!
        },
        {
          id: "p3",
          catalogId: "24188",
          transform: { x: 376.6, y: 0, rotation: 0 },
          connectors: [
            { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "p2", connectorIndex: 1 } },
            { x: 564.9, y: 0, angle: 0 },
          ],
        },
      ],
    };

    const sim = simulatePowerDrop(layout, maerklinCCatalog);
    expect(sim.results.get("p1")?.status).toBe("optimal");
    // p2 ist über Anschluss 0 versorgt und leitet:
    expect(sim.results.get("p2")?.status).toBe("optimal");
    expect(sim.results.get("p2")?.voltage).toBeCloseTo(16.0, 1);
    // p3 ist über Anschluss 1 abgetrennt:
    expect(sim.results.get("p3")?.status).toBe("disconnected");
    expect(sim.results.get("p3")?.voltage).toBe(0);
    expect(sim.disconnectedCount).toBe(1);
  });

  describe("Einstellbare Lok-Stromstärken & Maßstabs-Presets", () => {
    it("verwendet maßstabsabhängige Richtwerte für Loks (N, Z = 100mA, TT = 300mA, H0 = 450mA)", () => {
      const basePiece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
        powerFeed: true,
      };

      const layoutN: Layout = { name: "N Layout", scale: "N", pieces: [{ ...basePiece }] };
      const simN = simulatePowerDrop(layoutN, maerklinCCatalog);
      expect(simN.locoCurrentMa).toBe(100);
      expect(simN.trainCurrentAmps).toBeCloseTo(0.10, 2);

      const layoutTT: Layout = { name: "TT Layout", scale: "TT", pieces: [{ ...basePiece }] };
      const simTT = simulatePowerDrop(layoutTT, maerklinCCatalog);
      expect(simTT.locoCurrentMa).toBe(300);
      expect(simTT.trainCurrentAmps).toBeCloseTo(0.30, 2);

      const layoutH0: Layout = { name: "H0 Layout", scale: "H0", pieces: [{ ...basePiece }] };
      const simH0 = simulatePowerDrop(layoutH0, maerklinCCatalog);
      expect(simH0.locoCurrentMa).toBe(450);
      expect(simH0.trainCurrentAmps).toBeCloseTo(0.45, 2);
    });

    it("begrenzt den Maximalstrom pro Lok auf 1 A (1000 mA)", () => {
      const layout: Layout = {
        name: "Max Current Test",
        scale: "H0",
        locoCurrentMa: 2500, // Nutzer gibt übertriebenen Wert ein
        pieces: [
          {
            id: "p1",
            catalogId: "24188",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
            powerFeed: true,
          },
        ],
      };

      const sim = simulatePowerDrop(layout, maerklinCCatalog);
      expect(sim.locoCurrentMa).toBe(1000); // Gekappt auf 1A
      expect(sim.trainCurrentAmps).toBe(1.0);
    });

    it("berechnet den Gesamtstrom bei Mehrzugbetrieb (locoCount * locoCurrentMa)", () => {
      const layout: Layout = {
        name: "Multi-Loco Test",
        scale: "H0",
        locoCurrentMa: 400,
        locoCount: 3,
        pieces: [
          {
            id: "p1",
            catalogId: "24188",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
            powerFeed: true,
          },
        ],
      };

      const sim = simulatePowerDrop(layout, maerklinCCatalog);
      expect(sim.locoCurrentMa).toBe(400);
      expect(sim.locoCount).toBe(3);
      expect(sim.trainCurrentAmps).toBeCloseTo(1.20, 2);
    });
  });

  describe("Oberleitungs-Stromkreis (Catenary Circuit)", () => {
    it("simuliert getrennten Oberleitungsstromkreis mit eigener Einspeisung und Trennstelle", () => {
      const layout: Layout = {
        name: "Catenary Test Layout",
        scale: "H0",
        feedVoltage: 16.0,
        catenaryFeedVoltage: 14.0,
        catenaryCircuitMode: "separate",
        pieces: [
          {
            id: "p1",
            catalogId: "24188",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
            ],
            powerFeed: true,
            hasCatenary: true,
            catenaryPowerFeed: true,
          },
          {
            id: "p2",
            catalogId: "24188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "p3", connectorIndex: 0 } },
            ],
            hasCatenary: true,
          },
          {
            id: "p3",
            catalogId: "24188",
            transform: { x: 376.6, y: 0, rotation: 0 },
            connectors: [
              { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "p2", connectorIndex: 1 } },
              { x: 564.9, y: 0, angle: 0, connectedTo: { pieceId: "p4", connectorIndex: 0 } },
            ],
            hasCatenary: true,
            catenaryIsolator: true,
          },
          {
            id: "p4",
            catalogId: "24188",
            transform: { x: 564.9, y: 0, rotation: 0 },
            connectors: [
              { x: 564.9, y: 0, angle: 180, connectedTo: { pieceId: "p3", connectorIndex: 1 } },
              { x: 753.2, y: 0, angle: 0 },
            ],
            hasCatenary: true,
          },
        ],
      };

      const sim = simulatePowerDrop(layout, maerklinCCatalog);
      expect(sim.catenaryPieceCount).toBe(4);
      expect(sim.catenaryFeederCount).toBe(1);
      expect(sim.catenaryIsolatorCount).toBe(1);
      expect(sim.catenaryFeedVoltage).toBe(14.0);
      expect(sim.catenaryCircuitMode).toBe("separate");

      // p1: Oberleitungseinspeisung
      const resP1 = sim.results.get("p1");
      expect(resP1?.catenaryVoltage).toBeCloseTo(14.0, 1);
      expect(resP1?.catenaryStatus).toBe("optimal");

      // p2: verbunden mit p1
      const resP2 = sim.results.get("p2");
      expect(resP2?.catenaryVoltage).toBeGreaterThan(13.8);
      expect(resP2?.catenaryStatus).toBe("optimal");

      // p3: Oberleitungstrennstelle -> kein Strom in Oberleitung
      const resP3 = sim.results.get("p3");
      expect(resP3?.catenaryStatus).toBe("disconnected");

      // p4: hinter Oberleitungstrennstelle
      const resP4 = sim.results.get("p4");
      expect(resP4?.catenaryStatus).toBe("disconnected");

      // Schienenspannung bleibt über alle Gleise verbunden
      expect(resP4?.voltage).toBeGreaterThan(15.0);
      expect(resP4?.status).toBe("optimal");
    });

    it("reduziert Spannungsabfall im Parallelschaltungs-Modus (catenaryCircuitMode: 'parallel')", () => {
      const pieces: TrackPiece[] = [];
      const numPieces = 10;
      for (let i = 0; i < numPieces; i++) {
        const x = i * 188.3;
        pieces.push({
          id: `p${i + 1}`,
          catalogId: "24188",
          transform: { x, y: 0, rotation: 0 },
          connectors: [
            {
              x,
              y: 0,
              angle: 180,
              connectedTo: i > 0 ? { pieceId: `p${i}`, connectorIndex: 1 } : undefined,
            },
            {
              x: x + 188.3,
              y: 0,
              angle: 0,
              connectedTo: i < numPieces - 1 ? { pieceId: `p${i + 2}`, connectorIndex: 0 } : undefined,
            },
          ],
          powerFeed: i === 0,
          hasCatenary: true,
          catenaryPowerFeed: i === 0,
        });
      }

      const layoutSeparate: Layout = {
        name: "Separate Mode Layout",
        scale: "H0",
        feedVoltage: 16.0,
        locoCurrentMa: 800,
        locoCount: 2,
        catenaryCircuitMode: "separate",
        pieces,
      };

      const layoutParallel: Layout = {
        ...layoutSeparate,
        catenaryCircuitMode: "parallel",
      };

      const simSeparate = simulatePowerDrop(layoutSeparate, maerklinCCatalog);
      const simParallel = simulatePowerDrop(layoutParallel, maerklinCCatalog);

      const lastSeparate = simSeparate.results.get(`p${numPieces}`)!;
      const lastParallel = simParallel.results.get(`p${numPieces}`)!;

      expect(lastParallel.resistanceOhm).toBeLessThan(lastSeparate.resistanceOhm);
      expect(lastParallel.voltageDrop).toBeLessThan(lastSeparate.voltageDrop);
      expect(lastParallel.voltage).toBeGreaterThan(lastSeparate.voltage);
    });
  });

  describe("Weichen-Zubehör & Digitaldecoder (Turnout Accessories)", () => {
    it("erkennt Weichen zuverlässig über isTurnoutEntry", () => {
      const wLinks = maerklinCCatalog.entries.find((e) => e.id === "24611");
      const wRechts = maerklinCCatalog.entries.find((e) => e.id === "24612");
      const straight = maerklinCCatalog.entries.find((e) => e.id === "24188");

      expect(isTurnoutEntry(wLinks)).toBe(true);
      expect(isTurnoutEntry(wRechts)).toBe(true);
      expect(isTurnoutEntry(straight)).toBe(false);
    });

    it("berechnet Weichenlaternen- und Decoder-Ruhestrom sowie Spannungsabfall", () => {
      const numPieces = 5;
      const pieces: TrackPiece[] = [];
      for (let i = 0; i < numPieces; i++) {
        const x = i * 188.3;
        const isTurnout = i === numPieces - 1;
        pieces.push({
          id: `p${i + 1}`,
          catalogId: isTurnout ? "24611" : "24188",
          transform: { x, y: 0, rotation: 0 },
          connectors: [
            { x, y: 0, angle: 180, connectedTo: i > 0 ? { pieceId: `p${i}`, connectorIndex: 1 } : undefined },
            { x: x + 188.3, y: 0, angle: 0, connectedTo: i < numPieces - 1 ? { pieceId: `p${i + 2}`, connectorIndex: 0 } : undefined },
          ],
          powerFeed: i === 0,
          hasLantern: isTurnout,
          hasDecoder: isTurnout,
        });
      }

      const layout: Layout = {
        name: "Turnout Power Test",
        scale: "H0",
        feedVoltage: 16.0,
        turnoutLanternCurrentMa: 30,
        turnoutDecoderCurrentMa: 20, // 50 mA = 0.05 A
        pieces,
      };

      const simWithTurnout = simulatePowerDrop(layout, maerklinCCatalog, { trainCurrentAmps: 0.5 });
      expect(simWithTurnout.turnoutCount).toBe(1);
      expect(simWithTurnout.turnoutLanternCount).toBe(1);
      expect(simWithTurnout.turnoutDecoderCount).toBe(1);
      expect(simWithTurnout.turnoutAccessoryCurrentAmps).toBeCloseTo(0.05, 3);
      expect(simWithTurnout.totalCurrentAmps).toBeCloseTo(0.55, 3);

      const simWithoutTurnout = simulatePowerDrop(
        {
          ...layout,
          pieces: pieces.map((p) => ({ ...p, hasLantern: false, hasDecoder: false })),
        },
        maerklinCCatalog,
        { trainCurrentAmps: 0.5 },
      );

      const lastWith = simWithTurnout.results.get(`p${numPieces}`)!;
      const lastWithout = simWithoutTurnout.results.get(`p${numPieces}`)!;

      expect(lastWith.accessoryDrop).toBeGreaterThan(0);
      expect(lastWith.voltageDrop).toBeGreaterThanOrEqual(lastWithout.voltageDrop);
      expect(lastWith.voltage).toBeLessThanOrEqual(lastWithout.voltage);
    });

    it("berücksichtigt benutzerdefinierte Stromaufnahmen pro Weiche (turnoutCurrentMa)", () => {
      const layout: Layout = {
        name: "Custom Turnout Load",
        scale: "H0",
        feedVoltage: 16.0,
        pieces: [
          {
            id: "p1",
            catalogId: "24188",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
            ],
            powerFeed: true,
          },
          {
            id: "p2",
            catalogId: "24611",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0 },
            ],
            hasLantern: true,
            turnoutCurrentMa: 80, // Benutzerdefiniert 80 mA
          },
        ],
      };

      const sim = simulatePowerDrop(layout, maerklinCCatalog);
      expect(sim.turnoutAccessoryCurrentAmps).toBeCloseTo(0.08, 3);
      expect(sim.results.get("p2")?.accessoryCurrentMa).toBe(80);
    });
  });

  describe("2-Wire Short Circuit Detection (Kurzschluss-Erkennung)", () => {
    it("erkennt 2-Leiter-Systeme korrekt", () => {
      expect(isTwoWireCatalog(trixCCatalog)).toBe(true);
      expect(isTwoWireCatalog(pikoACatalog)).toBe(true);
      expect(isTwoWireCatalog(maerklinCCatalog)).toBe(false);
    });

    it("meldet keinen Kurzschluss bei einer normalen 2-Leiter-Ovalstrecke", () => {
      const layout: Layout = {
        name: "Normal 2-Rail Oval",
        scale: "H0",
        pieces: [
          {
            id: "p1",
            catalogId: "62188",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
            ],
          },
          {
            id: "p2",
            catalogId: "62188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0 },
            ],
          },
        ],
      };

      const res = detectShortCircuit(layout, trixCCatalog);
      expect(res.isTwoWireSystem).toBe(true);
      expect(res.hasShortCircuit).toBe(false);
      expect(res.shortCircuitPieces.size).toBe(0);
      expect(res.shortCircuitPoints.length).toBe(0);
    });

    it("erkennt Kurzschluss bei einer Kehrschleife im 2-Leiter-System (Verbindung von linker und rechter Schiene)", () => {
      const layout: Layout = {
        name: "Reverse Loop Shortcut",
        scale: "H0",
        pieces: [
          {
            id: "t1",
            catalogId: "62611",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "c1", connectorIndex: 0 } },
              { x: 188.3, y: 40, angle: 24.3, connectedTo: { pieceId: "c2", connectorIndex: 1 } },
            ],
          },
          {
            id: "c1",
            catalogId: "62188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "t1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "c2", connectorIndex: 0 } },
            ],
          },
          {
            id: "c2",
            catalogId: "62188",
            transform: { x: 376.6, y: 0, rotation: 0 },
            connectors: [
              { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "c1", connectorIndex: 1 } },
              { x: 188.3, y: 40, angle: 204.3, connectedTo: { pieceId: "t1", connectorIndex: 2 } },
            ],
          },
        ],
      };

      const res = detectShortCircuit(layout, trixCCatalog);
      expect(res.isTwoWireSystem).toBe(true);
      expect(res.hasShortCircuit).toBe(true);
      expect(res.shortCircuitPieces.has("t1")).toBe(true);
      expect(res.shortCircuitPieces.has("c1")).toBe(true);
      expect(res.shortCircuitPieces.has("c2")).toBe(true);
      expect(res.shortCircuitPoints.length).toBeGreaterThan(0);
    });

    it("bricht Kurzschluss ab, wenn eine Trennstelle (isIsolator) eingebaut wird", () => {
      const layout: Layout = {
        name: "Reverse Loop with Isolator",
        scale: "H0",
        pieces: [
          {
            id: "t1",
            catalogId: "62611",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "c1", connectorIndex: 0 } },
              { x: 188.3, y: 40, angle: 24.3, connectedTo: { pieceId: "c2", connectorIndex: 1 } },
            ],
          },
          {
            id: "c1",
            catalogId: "62188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "t1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "c2", connectorIndex: 0 } },
            ],
          },
          {
            id: "c2",
            catalogId: "62188",
            transform: { x: 376.6, y: 0, rotation: 0 },
            connectors: [
              { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "c1", connectorIndex: 1 } },
              { x: 188.3, y: 40, angle: 204.3, connectedTo: { pieceId: "t1", connectorIndex: 2 } },
            ],
            isIsolator: true,
            isolatedConnectorIndex: 1,
          },
        ],
      };

      const res = detectShortCircuit(layout, trixCCatalog);
      expect(res.isTwoWireSystem).toBe(true);
      expect(res.hasShortCircuit).toBe(false);
    });

    it("meldet keinen Kurzschluss in einem 3-Leiter-System (Märklin C, conductors=3)", () => {
      const layout: Layout = {
        name: "3-Rail Reverse Loop",
        scale: "H0",
        pieces: [
          {
            id: "t1",
            catalogId: "24611",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "c1", connectorIndex: 0 } },
              { x: 188.3, y: 40, angle: 24.3, connectedTo: { pieceId: "c2", connectorIndex: 1 } },
            ],
          },
          {
            id: "c1",
            catalogId: "24188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "t1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "c2", connectorIndex: 0 } },
            ],
          },
          {
            id: "c2",
            catalogId: "24188",
            transform: { x: 376.6, y: 0, rotation: 0 },
            connectors: [
              { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "c1", connectorIndex: 1 } },
              { x: 188.3, y: 40, angle: 204.3, connectedTo: { pieceId: "t1", connectorIndex: 2 } },
            ],
          },
        ],
      };

      const res = detectShortCircuit(layout, maerklinCCatalog);
      expect(res.isTwoWireSystem).toBe(false);
      expect(res.hasShortCircuit).toBe(false);
    });

    it("integriert Kurzschluss in simulatePowerDrop und warnt in der Empfehlung", () => {
      const layout: Layout = {
        name: "Simulation Reverse Loop",
        scale: "H0",
        feedVoltage: 14.0,
        pieces: [
          {
            id: "t1",
            catalogId: "62611",
            transform: { x: 0, y: 0, rotation: 0 },
            connectors: [
              { x: 0, y: 0, angle: 180 },
              { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "c1", connectorIndex: 0 } },
              { x: 188.3, y: 40, angle: 24.3, connectedTo: { pieceId: "c2", connectorIndex: 1 } },
            ],
            powerFeed: true,
          },
          {
            id: "c1",
            catalogId: "62188",
            transform: { x: 188.3, y: 0, rotation: 0 },
            connectors: [
              { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "t1", connectorIndex: 1 } },
              { x: 376.6, y: 0, angle: 0, connectedTo: { pieceId: "c2", connectorIndex: 0 } },
            ],
          },
          {
            id: "c2",
            catalogId: "62188",
            transform: { x: 376.6, y: 0, rotation: 0 },
            connectors: [
              { x: 376.6, y: 0, angle: 180, connectedTo: { pieceId: "c1", connectorIndex: 1 } },
              { x: 188.3, y: 40, angle: 204.3, connectedTo: { pieceId: "t1", connectorIndex: 2 } },
            ],
          },
        ],
      };

      const sim = simulatePowerDrop(layout, trixCCatalog);
      expect(sim.hasShortCircuit).toBe(true);
      expect(sim.shortCircuitPieces?.length).toBe(3);
      expect(sim.recommendation).toContain("Kurzschluss");
      expect(sim.results.get("t1")?.isShortCircuit).toBe(true);
    });
  });
});
