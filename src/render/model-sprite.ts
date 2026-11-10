import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { Model3D } from "@/core/types";
import { deg2rad, getUprightTextRotation } from "@/core/geometry";
import { findBuiltinModelDef } from "@/core/models-3d";

export function createModel2DGraphics(model: Model3D, isSelected: boolean): Container {
  const container = new Container();
  container.position.set(model.transform.x, model.transform.y);
  container.rotation = deg2rad(model.transform.rotation);

  const scale = model.scale ?? 1.0;
  const w = Math.max(10, model.width * scale);
  const d = Math.max(10, model.depth * scale);

  const halfW = w / 2;
  const halfD = d / 2;

  const g = new Graphics();

  // 1. Kategoriespezifische Farbcodierung
  let fillColor = 0xd97706;  // Gebäude: Terracotta / Amber
  let strokeColor = 0xb45309;
  let icon = "📦";

  if (model.modelType === "builtin" && model.builtinId) {
    const def = findBuiltinModelDef(model.builtinId);
    if (def) icon = def.icon;
  }

  switch (model.category) {
    case "nature":
      fillColor = 0x16a34a;
      strokeColor = 0x15803d;
      if (icon === "📦") icon = "🌲";
      break;
    case "railway":
      fillColor = 0x0284c7;
      strokeColor = 0x0369a1;
      if (icon === "📦") icon = "🚂";
      break;
    case "buildings":
      fillColor = 0xd97706;
      strokeColor = 0xb45309;
      if (icon === "📦") icon = "🏢";
      break;
    case "custom":
    default:
      fillColor = 0x6366f1;
      strokeColor = 0x4f46e5;
      if (icon === "📦") icon = "🧊";
      break;
  }

  // 2. Grundriss (Footprint) zeichnen
  g.roundRect(-halfW, -halfD, w, d, Math.min(6, Math.min(w, d) / 4));
  g.fill({ color: fillColor, alpha: isSelected ? 0.45 : 0.25 });
  g.stroke({
    width: isSelected ? 2.5 : 1.5,
    color: isSelected ? 0x38bdf8 : strokeColor,
    alpha: 0.95,
  });

  // Diagonale Konstruktionslinie / Architekturgitter
  if (w >= 30 && d >= 30) {
    g.moveTo(-halfW + 4, -halfD + 4);
    g.lineTo(halfW - 4, halfD - 4);
    g.stroke({ width: 0.8, color: strokeColor, alpha: 0.25 });

    g.moveTo(halfW - 4, -halfD + 4);
    g.lineTo(-halfW + 4, halfD - 4);
    g.stroke({ width: 0.8, color: strokeColor, alpha: 0.25 });
  }

  // 3. Wenn selektiert: Rotations-Griff zeichnen (oben)
  if (isSelected) {
    const handleDist = halfD + 18;

    // Verbindungslinie
    g.moveTo(0, -halfD);
    g.lineTo(0, -handleDist);
    g.stroke({ width: 1.5, color: 0x38bdf8, alpha: 0.85 });

    // Drehknopf
    g.circle(0, -handleDist, 6);
    g.fill({ color: 0x38bdf8 });
    g.stroke({ width: 1.5, color: 0xffffff });

    // Pfeil-Symbol im Knopf
    g.circle(0, -handleDist, 2);
    g.fill({ color: 0x0f172a });
  }

  container.addChild(g);

  // 4. Text-Beschriftung im Modell-Zentrum (stets horizontal und aufrecht per Gegen-Rotation)
  if (w >= 35 && d >= 22) {
    const rot = model.transform.rotation ?? 0;
    const rad = deg2rad(rot);
    const cosA = Math.abs(Math.cos(rad));
    const sinA = Math.abs(Math.sin(rad));
    // Verfügbare horizontale Spannweite im gedrehten Gebäudekörper
    const maxHorizWidth = Math.min(
      cosA > 0.001 ? w / cosA : Infinity,
      sinA > 0.001 ? d / sinA : Infinity,
    );
    const wrapWidth = Math.max(28, Math.min(maxHorizWidth - 8, Math.max(w, d)));

    const style = new TextStyle({
      fontFamily: "system-ui, -apple-system, sans-serif",
      fontSize: Math.min(11, Math.max(8, Math.min(w / 10, d / 3))),
      fontWeight: "bold",
      fill: isSelected ? "#38bdf8" : "#ffffff",
      dropShadow: {
        color: "#000000",
        blur: 3,
        alpha: 0.8,
        distance: 1,
      },
      align: "center",
      wordWrap: true,
      wordWrapWidth: wrapWidth,
    });

    const labelCont = new Container();
    labelCont.rotation = getUprightTextRotation(rot);

    const labelText = new Text({
      text: `${icon} ${model.name}`,
      style,
    });
    labelText.anchor.set(0.5, 0.5);

    // Dezentes dunkles Hintergrund-Badge für saubere Lesbarkeit auf allen Modellfarben
    const padX = 5;
    const padY = 2.5;
    let textW = 40;
    let textH = 12;
    try {
      if (typeof document !== "undefined") {
        textW = labelText.width || 40;
        textH = labelText.height || 12;
      }
    } catch {
      textW = 40;
      textH = 12;
    }
    const bgW = textW + padX * 2;
    const bgH = textH + padY * 2;
    const bg = new Graphics();
    bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1, bgW, bgH, 3);
    bg.fill({ color: 0x000000, alpha: 0.5 });
    bg.roundRect(-bgW / 2, -bgH / 2, bgW, bgH, 3);
    bg.fill({ color: 0x0f172a, alpha: 0.8 });
    bg.stroke({ width: 1, color: isSelected ? 0x38bdf8 : 0x475569, alpha: 0.85 });

    labelCont.addChild(bg);
    labelCont.addChild(labelText);
    container.addChild(labelCont);
  }

  return container;
}

