import { describe, it, expect } from "vitest";
import { Container, Text } from "pixi.js";
import { drawVoltageBadge } from "@/render/track-badges";
import type { CatalogEntry } from "@/catalog/catalog";
import type { PiecePowerResult } from "@/core/simulation";

describe("track-badges: drawVoltageBadge", () => {
  const dummyEntry: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188,3 mm",
    kind: "straight",
    category: "Geraden",
    description: "",
    length: 188.3,
  };

  it("zeigt Spannung auf dem Gleis ohne Spannungsabfall in Klammern", () => {
    const container = new Container();
    const powerResult: PiecePowerResult = {
      pieceId: "p1",
      voltage: 15.82,
      voltageDrop: 0.18,
      voltageDropPercent: 1.1,
      status: "optimal",
      resistanceOhm: 0.4,
      distanceFromFeederMeters: 2.5,
      isFeeder: false,
    };

    drawVoltageBadge(container, dummyEntry, 0, powerResult, false);

    // Finde das Text-Objekt im Badge
    const badgeCont = container.children[0] as Container;
    const textChild = badgeCont.children.find((c) => c instanceof Text) as Text;
    expect(textChild).toBeDefined();
    expect(textChild.text).toBe("15.82 V");
    expect(textChild.text).not.toContain("(-");
  });

  it("zeigt Feeder-Spannung mit Blitz-Icon ohne Spannungsabfall", () => {
    const container = new Container();
    const powerResult: PiecePowerResult = {
      pieceId: "feeder-1",
      voltage: 16.0,
      voltageDrop: 0.0,
      voltageDropPercent: 0.0,
      status: "optimal",
      resistanceOhm: 0.01,
      distanceFromFeederMeters: 0,
      isFeeder: true,
    };

    drawVoltageBadge(container, dummyEntry, 0, powerResult, false);

    const badgeCont = container.children[0] as Container;
    const textChild = badgeCont.children.find((c) => c instanceof Text) as Text;
    expect(textChild.text).toBe("⚡ 16.00 V");
    expect(textChild.text).not.toContain("(-");
  });

  it("zeigt Oberleitung mit Spannung ohne Abfall in Klammern", () => {
    const container = new Container();
    const powerResult: PiecePowerResult = {
      pieceId: "p2",
      voltage: 15.90,
      voltageDrop: 0.10,
      voltageDropPercent: 0.6,
      status: "optimal",
      resistanceOhm: 0.2,
      distanceFromFeederMeters: 1.5,
      isFeeder: false,
      hasCatenary: true,
      catenaryVoltage: 15.85,
      catenaryDrop: 0.15,
      catenaryDropPercent: 0.9,
      catenaryStatus: "optimal",
    };

    drawVoltageBadge(container, dummyEntry, 0, powerResult, false);

    const badgeCont = container.children[0] as Container;
    const textChild = badgeCont.children.find((c) => c instanceof Text) as Text;
    expect(textChild.text).toBe("15.90 V • OL: 15.85 V");
    expect(textChild.text).not.toContain("(-");
  });
});
