import { describe, it, expect } from "vitest";
import { toJSON, fromJSON, toXML, fromXML } from "@/io/serialize";
import type { Layout } from "@/core/types";

describe("Layout Serialization", () => {
  const layout: Layout = {
    name: "Testbahn",
    scale: "H0",
    feedVoltage: 16,
    pieces: [
      {
        id: "p1",
        catalogId: "24188",
        transform: { x: 100, y: 200, rotation: 0 },
        connectors: [{ x: 100, y: 200, angle: 180 }, { x: 288.3, y: 200, angle: 0 }],
        powerFeed: true,
      },
    ],
  };

  it("serialisiert und deserialisiert JSON", () => {
    const json = toJSON(layout);
    const parsed = fromJSON(json);
    expect(parsed.name).toBe("Testbahn");
    expect(parsed.pieces.length).toBe(1);
    expect(parsed.pieces[0]?.catalogId).toBe("24188");
  });

  it("serialisiert und deserialisiert JSON mit Ebenen", () => {
    const layoutWithLayers: Layout = {
      ...layout,
      layers: [
        { id: "layer-1", name: "Schattenbahnhof", heightMm: -100, color: "#94a3b8", visible: true, locked: false },
        { id: "layer-2", name: "Hauptstrecke", heightMm: 0, color: "#38bdf8", visible: true, locked: true },
      ],
      activeLayerId: "layer-2",
      pieces: [
        {
          ...layout.pieces[0]!,
          layerId: "layer-2",
        },
      ],
    };

    const json = toJSON(layoutWithLayers);
    const parsed = fromJSON(json);
    expect(parsed.name).toBe("Testbahn");
    expect(parsed.layers?.length).toBe(2);
    expect(parsed.activeLayerId).toBe("layer-2");
    expect(parsed.pieces[0]?.layerId).toBe("layer-2");
    expect(parsed.layers?.find((l) => l.id === "layer-2")?.locked).toBe(true);
  });

  it("erzeugt wohlgeformtes XML mit Ebenen", () => {
    const layoutWithLayers: Layout = {
      ...layout,
      layers: [
        { id: "layer-1", name: "Schattenbahnhof", heightMm: -100, color: "#94a3b8", visible: false, locked: true },
      ],
      activeLayerId: "layer-1",
      pieces: [
        {
          ...layout.pieces[0]!,
          layerId: "layer-1",
        },
      ],
    };

    const xml = toXML(layoutWithLayers);
    expect(xml).toContain('name="Testbahn"');
    expect(xml).toContain('catalogId="24188"');
    expect(xml).toContain('powerFeed="true"');
    expect(xml).toContain('<layers activeLayerId="layer-1">');
    expect(xml).toContain('name="Schattenbahnhof"');
    expect(xml).toContain('locked="true"');
    expect(xml).toContain('visible="false"');
    expect(xml).toContain('layerId="layer-1"');
  });

  it("serialisiert und deserialisiert Baseboard / Holzplatte in JSON und XML", () => {
    const layoutWithBoard: Layout = {
      ...layout,
      baseboard: {
        enabled: true,
        width: 2500,
        depth: 1250,
        positionMode: "custom",
        customX: -50,
        customY: 100,
        style: "plywood",
        cornerRadius: 15,
        showDimensions: true,
      },
    };

    const json = toJSON(layoutWithBoard);
    const parsedJson = fromJSON(json);
    expect(parsedJson.baseboard).toBeDefined();
    expect(parsedJson.baseboard?.width).toBe(2500);
    expect(parsedJson.baseboard?.depth).toBe(1250);
    expect(parsedJson.baseboard?.style).toBe("plywood");
    expect(parsedJson.baseboard?.customX).toBe(-50);

    const xml = toXML(layoutWithBoard);
    expect(xml).toContain('<baseboard enabled="true"');
    expect(xml).toContain('width="2500"');
    expect(xml).toContain('depth="1250"');
    expect(xml).toContain('style="plywood"');
    expect(xml).toContain('customX="-50"');
  });

  it("unterstützt verschiedene Tischformen (L-Form, U-Form, T-Form, Dreieck) und Drehung", () => {
    const layoutWithTriangle: Layout = {
      ...layout,
      baseboard: {
        enabled: true,
        shape: "triangle",
        triangleType: "corner-tr",
        rotation: 45,
        width: 1800,
        depth: 1400,
        positionMode: "custom",
        customX: 100,
        customY: 200,
        style: "wood",
      },
    };

    const json = toJSON(layoutWithTriangle);
    const parsed = fromJSON(json);
    expect(parsed.baseboard?.shape).toBe("triangle");
    expect(parsed.baseboard?.triangleType).toBe("corner-tr");
    expect(parsed.baseboard?.rotation).toBe(45);

    const xml = toXML(layoutWithTriangle);
    expect(xml).toContain('shape="triangle"');
    expect(xml).toContain('triangleType="corner-tr"');
    expect(xml).toContain('rotation="45"');
  });

  it("unterstützt verschiedene Tischformen (L-Form, U-Form, T-Form)", () => {
    const layoutWithUShape: Layout = {
      ...layout,
      baseboard: {
        enabled: true,
        shape: "u-shape",
        width: 3200,
        depth: 2400,
        uLeftWingWidth: 700,
        uRightWingWidth: 750,
        uBackDepth: 600,
        positionMode: "center",
        style: "wood",
      },
    };

    const json = toJSON(layoutWithUShape);
    const parsed = fromJSON(json);
    expect(parsed.baseboard?.shape).toBe("u-shape");
    expect(parsed.baseboard?.uLeftWingWidth).toBe(700);
    expect(parsed.baseboard?.uRightWingWidth).toBe(750);

    const xml = toXML(layoutWithUShape);
    expect(xml).toContain('shape="u-shape"');
    expect(xml).toContain('uLeftWingWidth="700"');
    expect(xml).toContain('uRightWingWidth="750"');
  });

  it("unterstützt mehrere Baseboards und Tables in JSON und XML", () => {
    const layoutWithMultipleBoards: Layout = {
      ...layout,
      baseboards: [
        {
          id: "tisch-1",
          name: "Hauptplatte",
          enabled: true,
          shape: "rect",
          width: 2400,
          depth: 1200,
          positionMode: "custom",
          customX: 0,
          customY: 0,
          style: "wood",
        },
        {
          id: "tisch-2",
          name: "Schattenbahnhof Anbau",
          enabled: true,
          shape: "rect",
          width: 1600,
          depth: 800,
          positionMode: "custom",
          customX: 2450,
          customY: 0,
          style: "dark-wood",
        },
      ],
    };

    const json = toJSON(layoutWithMultipleBoards);
    const parsedJson = fromJSON(json);
    expect(parsedJson.baseboards?.length).toBe(2);
    expect(parsedJson.baseboards?.[0]?.name).toBe("Hauptplatte");
    expect(parsedJson.baseboards?.[1]?.name).toBe("Schattenbahnhof Anbau");
    expect(parsedJson.baseboards?.[1]?.customX).toBe(2450);
    expect(parsedJson.baseboard?.name).toBe("Hauptplatte");

    const xml = toXML(layoutWithMultipleBoards);
    expect(xml).toContain('<baseboards>');
    expect(xml).toContain('name="Hauptplatte"');
    expect(xml).toContain('name="Schattenbahnhof Anbau"');
    expect(xml).toContain('customX="2450"');
  });

  it("serialisiert und deserialisiert 3D-Modelle in JSON und XML", () => {
    const layoutWithModels: Layout = {
      ...layout,
      models: [
        {
          id: "m1",
          name: "Bahnhof Neustadt",
          category: "buildings",
          modelType: "builtin",
          builtinId: "station",
          transform: { x: 150, y: 250, rotation: 45 },
          width: 320,
          depth: 140,
          height: 120,
          elevation: 10,
          scale: 1.0,
          layerId: "layer-1",
        },
        {
          id: "m2",
          name: "Custom Signal",
          category: "custom",
          modelType: "glb",
          transform: { x: 300, y: 400, rotation: 0 },
          width: 30,
          depth: 30,
          height: 80,
          fileData: "data:model/gltf-binary;base64,ZmFrZQ==",
          fileName: "signal.glb",
        },
      ],
    };

    const json = toJSON(layoutWithModels);
    const parsed = fromJSON(json);
    expect(parsed.models?.length).toBe(2);
    expect(parsed.models?.[0]?.name).toBe("Bahnhof Neustadt");
    expect(parsed.models?.[0]?.builtinId).toBe("station");
    expect(parsed.models?.[1]?.modelType).toBe("glb");

    const xml = toXML(layoutWithModels);
    expect(xml).toContain('<models>');
    expect(xml).toContain('name="Bahnhof Neustadt"');
    expect(xml).toContain('builtinId="station"');
    expect(xml).toContain('modelType="glb"');
    expect(xml).toContain('<fileData>data:model/gltf-binary;base64,ZmFrZQ==</fileData>');
  });

  it("serialisiert und deserialisiert Trennstellen mit isolatedConnectorIndex in XML und JSON", () => {
    const layoutWithIso: Layout = {
      name: "Trennstellen Layout",
      scale: "H0",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 100, y: 100, rotation: 0 },
          connectors: [{ x: 100, y: 100, angle: 180 }, { x: 288.3, y: 100, angle: 0 }],
          isIsolator: true,
          isolatedConnectorIndex: 1,
        },
      ],
    };

    const xml = toXML(layoutWithIso);
    expect(xml).toContain('isIsolator="true"');
    expect(xml).toContain('isolatedConnectorIndex="1"');

    const json = toJSON(layoutWithIso);
    const parsed = fromJSON(json);
    expect(parsed.pieces[0]?.isIsolator).toBe(true);
    expect(parsed.pieces[0]?.isolatedConnectorIndex).toBe(1);
  });

  it("serialisiert und deserialisiert locoCurrentMa und locoCount in XML und JSON", () => {
    const layoutWithPower: Layout = {
      name: "Strom Layout",
      scale: "H0",
      feedVoltage: 14.5,
      locoCurrentMa: 550,
      locoCount: 2,
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
        },
      ],
    };

    const xml = toXML(layoutWithPower);
    expect(xml).toContain('feedVoltage="14.5"');
    expect(xml).toContain('locoCurrentMa="550"');
    expect(xml).toContain('locoCount="2"');

    const json = toJSON(layoutWithPower);
    const parsedJson = fromJSON(json);
    expect(parsedJson.feedVoltage).toBe(14.5);
    expect(parsedJson.locoCurrentMa).toBe(550);
    expect(parsedJson.locoCount).toBe(2);
  });

  it("serialisiert und deserialisiert hasCatenary in XML und JSON", () => {
    const layoutWithCatenary: Layout = {
      name: "Oberleitungs-Layout",
      scale: "H0",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
          hasCatenary: true,
        },
        {
          id: "p2",
          catalogId: "24172",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 359.8, y: 0, angle: 0 }],
          hasCatenary: false,
        },
      ],
    };

    const xml = toXML(layoutWithCatenary);
    expect(xml).toContain('hasCatenary="true"');

    const json = toJSON(layoutWithCatenary);
    const parsedJson = fromJSON(json);
    expect(parsedJson.pieces[0]?.hasCatenary).toBe(true);
    expect(parsedJson.pieces[1]?.hasCatenary).toBe(false);
  });

  it("serialisiert und deserialisiert catenaryPowerFeed und catenaryIsolator in XML und JSON", () => {
    const layoutWithCatenaryPower: Layout = {
      name: "Oberleitung Elektrik",
      scale: "H0",
      catenaryFeedVoltage: 15.5,
      catenaryCircuitMode: "parallel",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
          hasCatenary: true,
          catenaryPowerFeed: true,
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 376.6, y: 0, angle: 0 }],
          hasCatenary: true,
          catenaryIsolator: true,
          catenaryIsolatedConnectorIndex: 1,
        },
      ],
    };

    const xml = toXML(layoutWithCatenaryPower);
    expect(xml).toContain('catenaryFeedVoltage="15.5"');
    expect(xml).toContain('catenaryCircuitMode="parallel"');
    expect(xml).toContain('catenaryPowerFeed="true"');
    expect(xml).toContain('catenaryIsolator="true"');
    expect(xml).toContain('catenaryIsolatedConnectorIndex="1"');

    const json = toJSON(layoutWithCatenaryPower);
    const parsedJson = fromJSON(json);
    expect(parsedJson.catenaryFeedVoltage).toBe(15.5);
    expect(parsedJson.catenaryCircuitMode).toBe("parallel");
    expect(parsedJson.pieces[0]?.catenaryPowerFeed).toBe(true);
    expect(parsedJson.pieces[1]?.catenaryIsolator).toBe(true);
    expect(parsedJson.pieces[1]?.catenaryIsolatedConnectorIndex).toBe(1);
  });

  it("serialisiert und deserialisiert Weichenlaterne, Decoder und Ruhestrom in XML und JSON", () => {
    const layoutWithTurnout: Layout = {
      name: "Weichen Layout",
      scale: "H0",
      turnoutLanternCurrentMa: 25,
      turnoutDecoderCurrentMa: 18,
      pieces: [
        {
          id: "w1",
          catalogId: "24611",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
          hasLantern: true,
          hasDecoder: true,
          turnoutCurrentMa: 43,
        },
      ],
    };

    const xml = toXML(layoutWithTurnout);
    expect(xml).toContain('turnoutLanternCurrentMa="25"');
    expect(xml).toContain('turnoutDecoderCurrentMa="18"');
    expect(xml).toContain('hasLantern="true"');
    expect(xml).toContain('hasDecoder="true"');
    expect(xml).toContain('turnoutCurrentMa="43"');

    const json = toJSON(layoutWithTurnout);
    const parsedJson = fromJSON(json);
    expect(parsedJson.turnoutLanternCurrentMa).toBe(25);
    expect(parsedJson.turnoutDecoderCurrentMa).toBe(18);
    expect(parsedJson.pieces[0]?.hasLantern).toBe(true);
    expect(parsedJson.pieces[0]?.hasDecoder).toBe(true);
    expect(parsedJson.pieces[0]?.turnoutCurrentMa).toBe(43);
  });

  it("serialisiert und deserialisiert Layer-Platten mit Stil/Textur in XML und JSON", () => {
    const layoutWithPlates: Layout = {
      name: "Platten Layout",
      scale: "H0",
      pieces: [],
      layers: [
        {
          id: "l-upper",
          name: "Obere Ebene",
          heightMm: 120,
          visible: true,
          locked: false,
          woodPlateMode: "table",
          plates: [
            {
              id: "pl-1",
              name: "Wiesen-Platte",
              width: 1200,
              depth: 600,
              thicknessMm: 12,
              style: "grass",
            },
            {
              id: "pl-2",
              name: "Schiefer-Platte",
              width: 800,
              depth: 400,
              thicknessMm: 10,
              style: "slate",
            },
          ],
        },
      ],
    };

    const xml = toXML(layoutWithPlates);
    expect(xml).toContain('style="grass"');
    expect(xml).toContain('style="slate"');

    const json = toJSON(layoutWithPlates);
    const parsedJson = fromJSON(json);
    expect(parsedJson.layers?.[0]?.plates?.[0]?.style).toBe("grass");
    expect(parsedJson.layers?.[0]?.plates?.[1]?.style).toBe("slate");
  });
});
