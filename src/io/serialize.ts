import type { Layout, Layer, LayerPlate, TrackPiece, BaseboardConfig, Model3D, ModelCategory } from "@/core/types";
import { maerklinCCatalog, catalogRegistry, computeWorldConnectors } from "@/catalog/catalog";
import { ensureDefaultLayers } from "@/core/layout";

export const toJSON = (l: Layout) => JSON.stringify(ensureDefaultLayers(l), null, 2);

export const fromJSON = (s: string): Layout => {
  const l = JSON.parse(s) as Layout;
  if (!l.pieces) l.pieces = [];
  return ensureDefaultLayers(l);
};

/** Serialisiert das Layout in ein sauberes XML-Format. */
export function toXML(layout: Layout): string {
  const readyLayout = ensureDefaultLayers(layout);
  let xml = `<?xml version="1.0" encoding="UTF-8"?>\n`;
  const locoMaAttr = readyLayout.locoCurrentMa !== undefined ? ` locoCurrentMa="${readyLayout.locoCurrentMa}"` : "";
  const locoCountAttr = readyLayout.locoCount !== undefined ? ` locoCount="${readyLayout.locoCount}"` : "";
  const catVoltAttr = readyLayout.catenaryFeedVoltage !== undefined ? ` catenaryFeedVoltage="${readyLayout.catenaryFeedVoltage}"` : "";
  const catModeAttr = readyLayout.catenaryCircuitMode !== undefined ? ` catenaryCircuitMode="${readyLayout.catenaryCircuitMode}"` : "";
  const lanternMaAttr = readyLayout.turnoutLanternCurrentMa !== undefined ? ` turnoutLanternCurrentMa="${readyLayout.turnoutLanternCurrentMa}"` : "";
  const decoderMaAttr = readyLayout.turnoutDecoderCurrentMa !== undefined ? ` turnoutDecoderCurrentMa="${readyLayout.turnoutDecoderCurrentMa}"` : "";
  xml += `<layout name="${escapeXml(readyLayout.name)}" scale="${readyLayout.scale}" feedVoltage="${readyLayout.feedVoltage ?? 16}"${catVoltAttr}${catModeAttr}${locoMaAttr}${locoCountAttr}${lanternMaAttr}${decoderMaAttr}>\n`;
  
  const boards = readyLayout.baseboards && readyLayout.baseboards.length > 0
    ? readyLayout.baseboards
    : (readyLayout.baseboard ? [readyLayout.baseboard] : []);
  if (boards.length > 0) {
    xml += `  <baseboards>\n`;
    for (const bb of boards) {
      xml += `    <baseboard enabled="${bb.enabled !== false}" id="${escapeXml(bb.id ?? '')}" name="${escapeXml(bb.name ?? 'Table')}" shape="${bb.shape || 'rect'}" triangleType="${escapeXml(bb.triangleType || 'corner-tl')}" rotation="${bb.rotation ?? 0}" width="${bb.width}" depth="${bb.depth}" positionMode="${bb.positionMode}" style="${bb.style}" cornerRadius="${bb.cornerRadius ?? 12}" showDimensions="${bb.showDimensions !== false}" customX="${bb.customX ?? 0}" customY="${bb.customY ?? 0}" lWingWidth="${bb.lWingWidth ?? 800}" lWingDepth="${bb.lWingDepth ?? 800}" lSide="${bb.lSide ?? 'left'}" uLeftWingWidth="${bb.uLeftWingWidth ?? 700}" uRightWingWidth="${bb.uRightWingWidth ?? 700}" uBackDepth="${bb.uBackDepth ?? 600}" tBarDepth="${bb.tBarDepth ?? 600}" tStemWidth="${bb.tStemWidth ?? 800}"/>\n`;
    }
    xml += `  </baseboards>\n`;
  }

  if (readyLayout.layers && readyLayout.layers.length > 0) {
    const firstLayerId = readyLayout.layers[0]?.id ?? "layer-1";
    xml += `  <layers activeLayerId="${escapeXml(readyLayout.activeLayerId ?? firstLayerId)}">\n`;
    for (const lay of readyLayout.layers) {
      const modeAttr = lay.woodPlateMode ? ` woodPlateMode="${escapeXml(lay.woodPlateMode)}"` : "";
      const thAttr = lay.woodPlateThicknessMm !== undefined ? ` woodPlateThicknessMm="${lay.woodPlateThicknessMm}"` : "";
      if (lay.plates && lay.plates.length > 0) {
        xml += `    <layer id="${escapeXml(lay.id)}" name="${escapeXml(lay.name)}" color="${escapeXml(lay.color || '#38bdf8')}" visible="${lay.visible !== false}" locked="${lay.locked === true}" heightMm="${lay.heightMm ?? 0}"${modeAttr}${thAttr}>\n`;
        for (const pl of lay.plates) {
          const nameAttr = pl.name ? ` name="${escapeXml(pl.name)}"` : "";
          const boardAttr = pl.baseboardId ? ` baseboardId="${escapeXml(pl.baseboardId)}"` : "";
          const posAttr = pl.positionMode ? ` positionMode="${escapeXml(pl.positionMode)}"` : "";
          const shapeAttr = pl.shape ? ` shape="${escapeXml(pl.shape)}"` : "";
          const triAttr = pl.triangleType ? ` triangleType="${escapeXml(pl.triangleType)}"` : "";
          const lWingWAttr = pl.lWingWidth !== undefined ? ` lWingWidth="${pl.lWingWidth}"` : "";
          const lWingDAttr = pl.lWingDepth !== undefined ? ` lWingDepth="${pl.lWingDepth}"` : "";
          const lSideAttr = pl.lSide ? ` lSide="${escapeXml(pl.lSide)}"` : "";
          const uLWAttr = pl.uLeftWingWidth !== undefined ? ` uLeftWingWidth="${pl.uLeftWingWidth}"` : "";
          const uRWAttr = pl.uRightWingWidth !== undefined ? ` uRightWingWidth="${pl.uRightWingWidth}"` : "";
          const uBDAttr = pl.uBackDepth !== undefined ? ` uBackDepth="${pl.uBackDepth}"` : "";
          const tBDAttr = pl.tBarDepth !== undefined ? ` tBarDepth="${pl.tBarDepth}"` : "";
          const tSWAttr = pl.tStemWidth !== undefined ? ` tStemWidth="${pl.tStemWidth}"` : "";
          const rotAttr = pl.rotation !== undefined ? ` rotation="${pl.rotation}"` : "";
          xml += `      <plate id="${escapeXml(pl.id)}"${nameAttr}${boardAttr} width="${pl.width}" depth="${pl.depth}" x="${pl.x ?? 0}" y="${pl.y ?? 0}" thicknessMm="${pl.thicknessMm ?? 10}"${posAttr}${shapeAttr}${triAttr}${lWingWAttr}${lWingDAttr}${lSideAttr}${uLWAttr}${uRWAttr}${uBDAttr}${tBDAttr}${tSWAttr}${rotAttr}/>\n`;
        }
        xml += `    </layer>\n`;
      } else {
        xml += `    <layer id="${escapeXml(lay.id)}" name="${escapeXml(lay.name)}" color="${escapeXml(lay.color || '#38bdf8')}" visible="${lay.visible !== false}" locked="${lay.locked === true}" heightMm="${lay.heightMm ?? 0}"${modeAttr}${thAttr}/>\n`;
      }
    }
    xml += `  </layers>\n`;
  }

  xml += `  <pieces>\n`;
  for (const p of readyLayout.pieces) {
    const pf = p.powerFeed ? ` powerFeed="true"` : "";
    const iso = p.isIsolator ? ` isIsolator="true"` : "";
    const cat = p.hasCatenary ? ` hasCatenary="true"` : "";
    const catPf = p.catenaryPowerFeed ? ` catenaryPowerFeed="true"` : "";
    const catIso = p.catenaryIsolator ? ` catenaryIsolator="true"` : "";
    const catIsoIdx = p.catenaryIsolator && p.catenaryIsolatedConnectorIndex !== undefined ? ` catenaryIsolatedConnectorIndex="${p.catenaryIsolatedConnectorIndex}"` : "";
    const isoIdx = p.isIsolator && p.isolatedConnectorIndex !== undefined ? ` isolatedConnectorIndex="${p.isolatedConnectorIndex}"` : "";
    const name = p.customName ? ` customName="${escapeXml(p.customName)}"` : "";
    const lay = p.layerId ? ` layerId="${escapeXml(p.layerId)}"` : "";
    const elev = p.elevation
      ? ` elevStart="${p.elevation.start.toFixed(2)}" elevEnd="${p.elevation.end.toFixed(2)}"`
      : "";
    const lant = p.hasLantern ? ` hasLantern="true"` : "";
    const dec = p.hasDecoder ? ` hasDecoder="true"` : "";
    const tMa = p.turnoutCurrentMa !== undefined ? ` turnoutCurrentMa="${p.turnoutCurrentMa}"` : "";
    const cLen = p.customLength !== undefined ? ` customLength="${p.customLength.toFixed(1)}"` : "";
    const cRad = p.customRadius !== undefined ? ` customRadius="${p.customRadius.toFixed(1)}"` : "";
    const cAng = p.customAngle !== undefined ? ` customAngle="${p.customAngle.toFixed(2)}"` : "";
    xml += `    <piece id="${p.id}" catalogId="${p.catalogId}" x="${p.transform.x.toFixed(2)}" y="${p.transform.y.toFixed(2)}" rotation="${p.transform.rotation.toFixed(2)}"${lay}${elev}${cLen}${cRad}${cAng}${pf}${iso}${isoIdx}${cat}${catPf}${catIso}${catIsoIdx}${lant}${dec}${tMa}${name}/>\n`;
  }
  xml += `  </pieces>\n`;

  if (readyLayout.models && readyLayout.models.length > 0) {
    xml += `  <models>\n`;
    for (const m of readyLayout.models) {
      const type = escapeXml(m.modelType || "builtin");
      const name = escapeXml(m.name || "Model");
      const cat = escapeXml(m.category || "custom");
      const bId = m.builtinId ? ` builtinId="${escapeXml(m.builtinId)}"` : "";
      const fName = m.fileName ? ` fileName="${escapeXml(m.fileName)}"` : "";
      const lay = m.layerId ? ` layerId="${escapeXml(m.layerId)}"` : "";
      const elev = m.elevation !== undefined ? ` elevation="${m.elevation.toFixed(2)}"` : "";
      const scale = m.scale !== undefined ? ` scale="${m.scale.toFixed(3)}"` : "";
      xml += `    <model id="${m.id}" name="${name}" category="${cat}" modelType="${type}" x="${m.transform.x.toFixed(2)}" y="${m.transform.y.toFixed(2)}" rotation="${m.transform.rotation.toFixed(2)}" width="${m.width.toFixed(1)}" depth="${m.depth.toFixed(1)}" height="${m.height.toFixed(1)}"${bId}${fName}${lay}${elev}${scale}>\n`;
      if (m.fileData) {
        xml += `      <fileData>${escapeXml(m.fileData)}</fileData>\n`;
      }
      xml += `    </model>\n`;
    }
    xml += `  </models>\n`;
  }

  xml += `</layout>`;
  return xml;
}

/** Deserialisiert das Layout aus einem XML-String. */
export function fromXML(xmlStr: string): Layout {
  const parser = new DOMParser();
  const doc = parser.parseFromString(xmlStr, "application/xml");

  const parserError = doc.querySelector("parsererror");
  if (parserError) {
    throw new Error("Ungültiges XML-Format: " + parserError.textContent);
  }

  const root = doc.querySelector("layout");
  if (!root) {
    throw new Error("Ungültiges XML: Kein <layout>-Wurzelelement gefunden.");
  }

  const name = root.getAttribute("name") ?? "Geladene Anlage";
  const scale = (root.getAttribute("scale") as "H0" | "TT" | "N" | "Z" | "1" | "G" | "0") ?? "H0";
  const feedVoltageAttr = root.getAttribute("feedVoltage");
  const feedVoltage = feedVoltageAttr ? parseFloat(feedVoltageAttr) : 16;
  const catVoltAttr = root.getAttribute("catenaryFeedVoltage");
  const catenaryFeedVoltage = catVoltAttr ? parseFloat(catVoltAttr) : undefined;
  const catModeAttr = root.getAttribute("catenaryCircuitMode");
  const catenaryCircuitMode = (catModeAttr as "separate" | "parallel") || undefined;
  const locoCurrentMaAttr = root.getAttribute("locoCurrentMa");
  const locoCurrentMa = locoCurrentMaAttr ? Math.min(1000, Math.max(50, parseFloat(locoCurrentMaAttr))) : undefined;
  const locoCountAttr = root.getAttribute("locoCount");
  const locoCount = locoCountAttr ? Math.max(1, parseInt(locoCountAttr, 10)) : undefined;
  const lanternMaAttr = root.getAttribute("turnoutLanternCurrentMa");
  const turnoutLanternCurrentMa = lanternMaAttr ? parseFloat(lanternMaAttr) : undefined;
  const decoderMaAttr = root.getAttribute("turnoutDecoderCurrentMa");
  const turnoutDecoderCurrentMa = decoderMaAttr ? parseFloat(decoderMaAttr) : undefined;

  const bbNodes = root.querySelectorAll("baseboard");
  const parsedBaseboards: BaseboardConfig[] = [];
  bbNodes.forEach((bbNode, index) => {
    parsedBaseboards.push({
      id: bbNode.getAttribute("id") || `board-${index + 1}`,
      name: bbNode.getAttribute("name") || `Tisch ${index + 1}`,
      enabled: bbNode.getAttribute("enabled") !== "false",
      shape: (bbNode.getAttribute("shape") as any) || "rect",
      triangleType: (bbNode.getAttribute("triangleType") as any) || undefined,
      rotation: parseFloat(bbNode.getAttribute("rotation") || "0"),
      width: parseFloat(bbNode.getAttribute("width") || "2400"),
      depth: parseFloat(bbNode.getAttribute("depth") || "1200"),
      positionMode: (bbNode.getAttribute("positionMode") as any) || "center",
      style: (bbNode.getAttribute("style") as any) || "wood",
      cornerRadius: parseFloat(bbNode.getAttribute("cornerRadius") || "12"),
      showDimensions: bbNode.getAttribute("showDimensions") !== "false",
      customX: parseFloat(bbNode.getAttribute("customX") || "0"),
      customY: parseFloat(bbNode.getAttribute("customY") || "0"),
      lWingWidth: parseFloat(bbNode.getAttribute("lWingWidth") || "800"),
      lWingDepth: parseFloat(bbNode.getAttribute("lWingDepth") || "800"),
      lSide: (bbNode.getAttribute("lSide") as any) || "left",
      uLeftWingWidth: parseFloat(bbNode.getAttribute("uLeftWingWidth") || "700"),
      uRightWingWidth: parseFloat(bbNode.getAttribute("uRightWingWidth") || "700"),
      uBackDepth: parseFloat(bbNode.getAttribute("uBackDepth") || "600"),
      tBarDepth: parseFloat(bbNode.getAttribute("tBarDepth") || "600"),
      tStemWidth: parseFloat(bbNode.getAttribute("tStemWidth") || "800"),
    });
  });
  const parsedBaseboard = parsedBaseboards.length > 0 ? parsedBaseboards[0] : undefined;

  const layersNode = root.querySelector("layers");
  const parsedLayers: Layer[] = [];
  let activeLayerId: string | undefined = undefined;

  if (layersNode) {
    activeLayerId = layersNode.getAttribute("activeLayerId") || undefined;
    const layerElements = layersNode.querySelectorAll("layer");
    layerElements.forEach((el) => {
      const modeAttr = el.getAttribute("woodPlateMode") as any;
      const thAttr = el.getAttribute("woodPlateThicknessMm");
      const plateNodes = el.querySelectorAll("plate");
      const plates: LayerPlate[] = [];
      plateNodes.forEach((pl) => {
        plates.push({
          id: pl.getAttribute("id") || `plate-${Date.now()}-${Math.random()}`,
          name: pl.getAttribute("name") || undefined,
          baseboardId: pl.getAttribute("baseboardId") || undefined,
          shape: (pl.getAttribute("shape") as any) || undefined,
          triangleType: (pl.getAttribute("triangleType") as any) || undefined,
          lWingWidth: pl.getAttribute("lWingWidth") ? parseFloat(pl.getAttribute("lWingWidth")!) : undefined,
          lWingDepth: pl.getAttribute("lWingDepth") ? parseFloat(pl.getAttribute("lWingDepth")!) : undefined,
          lSide: (pl.getAttribute("lSide") as any) || undefined,
          uLeftWingWidth: pl.getAttribute("uLeftWingWidth") ? parseFloat(pl.getAttribute("uLeftWingWidth")!) : undefined,
          uRightWingWidth: pl.getAttribute("uRightWingWidth") ? parseFloat(pl.getAttribute("uRightWingWidth")!) : undefined,
          uBackDepth: pl.getAttribute("uBackDepth") ? parseFloat(pl.getAttribute("uBackDepth")!) : undefined,
          tBarDepth: pl.getAttribute("tBarDepth") ? parseFloat(pl.getAttribute("tBarDepth")!) : undefined,
          tStemWidth: pl.getAttribute("tStemWidth") ? parseFloat(pl.getAttribute("tStemWidth")!) : undefined,
          rotation: pl.getAttribute("rotation") ? parseFloat(pl.getAttribute("rotation")!) : undefined,
          width: parseFloat(pl.getAttribute("width") || "1200"),
          depth: parseFloat(pl.getAttribute("depth") || "600"),
          x: parseFloat(pl.getAttribute("x") || "0"),
          y: parseFloat(pl.getAttribute("y") || "0"),
          thicknessMm: parseFloat(pl.getAttribute("thicknessMm") || "10"),
          positionMode: (pl.getAttribute("positionMode") as any) || "center",
        });
      });

      parsedLayers.push({
        id: el.getAttribute("id") || `layer-${Date.now()}`,
        name: el.getAttribute("name") || "Layer",
        color: el.getAttribute("color") || "#38bdf8",
        visible: el.getAttribute("visible") !== "false",
        locked: el.getAttribute("locked") === "true",
        heightMm: parseFloat(el.getAttribute("heightMm") || "0"),
        woodPlateMode: modeAttr || undefined,
        woodPlateThicknessMm: thAttr ? parseFloat(thAttr) : undefined,
        plates: plates.length > 0 ? plates : undefined,
      });
    });
  }

  const pieces: TrackPiece[] = [];
  const pieceNodes = doc.querySelectorAll("piece");

  const entryMap = new Map();
  for (const e of maerklinCCatalog.entries) {
    entryMap.set(e.id, e);
  }

  pieceNodes.forEach((node) => {
    const id = node.getAttribute("id") || `piece-${Date.now()}-${Math.random()}`;
    const catalogId = node.getAttribute("catalogId") || "24188";
    const x = parseFloat(node.getAttribute("x") || "0");
    const y = parseFloat(node.getAttribute("y") || "0");
    const rotation = parseFloat(node.getAttribute("rotation") || "0");
    const powerFeed = node.getAttribute("powerFeed") === "true";
    const isIsolator = node.getAttribute("isIsolator") === "true";
    const hasCatenary = node.getAttribute("hasCatenary") === "true";
    const catenaryPowerFeed = node.getAttribute("catenaryPowerFeed") === "true";
    const catenaryIsolator = node.getAttribute("catenaryIsolator") === "true";
    const catIsoIdxAttr = node.getAttribute("catenaryIsolatedConnectorIndex");
    const catenaryIsolatedConnectorIndex = catIsoIdxAttr !== null ? parseInt(catIsoIdxAttr, 10) : (catenaryIsolator ? 0 : undefined);
    const isoIdxAttr = node.getAttribute("isolatedConnectorIndex");
    const isolatedConnectorIndex = isoIdxAttr !== null ? parseInt(isoIdxAttr, 10) : (isIsolator ? 0 : undefined);
    const hasLantern = node.getAttribute("hasLantern") === "true";
    const hasDecoder = node.getAttribute("hasDecoder") === "true";
    const tMaAttr = node.getAttribute("turnoutCurrentMa");
    const turnoutCurrentMa = tMaAttr !== null ? parseFloat(tMaAttr) : undefined;
    const customName = node.getAttribute("customName") || undefined;
    const layerId = node.getAttribute("layerId") || undefined;
    const elevStart = node.getAttribute("elevStart");
    const elevEnd = node.getAttribute("elevEnd");
    const elevation =
      elevStart !== null && elevEnd !== null
        ? { start: parseFloat(elevStart), end: parseFloat(elevEnd) }
        : undefined;
    const customLength = node.hasAttribute("customLength") ? parseFloat(node.getAttribute("customLength")!) : undefined;
    const customRadius = node.hasAttribute("customRadius") ? parseFloat(node.getAttribute("customRadius")!) : undefined;
    const customAngle = node.hasAttribute("customAngle") ? parseFloat(node.getAttribute("customAngle")!) : undefined;

    const transform = { x, y, rotation };
    const entry =
      entryMap.get(catalogId) ||
      catalogRegistry.findEntry(catalogId) || { id: catalogId, name: catalogId, kind: "straight", length: 188.3 };
    const connectors = computeWorldConnectors(entry, transform, { customLength, customRadius, customAngle });

    pieces.push({
      id,
      catalogId,
      transform,
      connectors,
      elevation,
      customLength,
      customRadius,
      customAngle,
      powerFeed,
      isIsolator,
      isolatedConnectorIndex,
      hasCatenary: (hasCatenary || catenaryPowerFeed || catenaryIsolator) || undefined,
      catenaryPowerFeed: catenaryPowerFeed || undefined,
      catenaryIsolator: catenaryIsolator || undefined,
      catenaryIsolatedConnectorIndex,
      hasLantern: hasLantern || undefined,
      hasDecoder: hasDecoder || undefined,
      turnoutCurrentMa,
      customName,
      layerId,
    });
  });

  const modelsNode = root.querySelector("models");
  const parsedModels: Model3D[] = [];
  if (modelsNode) {
    const modelElements = modelsNode.querySelectorAll("model");
    modelElements.forEach((el) => {
      const id = el.getAttribute("id") || `model-${Date.now()}`;
      const name = el.getAttribute("name") || "Model";
      const category = (el.getAttribute("category") as ModelCategory) || "custom";
      const modelType = (el.getAttribute("modelType") as "builtin" | "glb" | "gltf") || "builtin";
      const builtinId = el.getAttribute("builtinId") || undefined;
      const fileName = el.getAttribute("fileName") || undefined;
      const layerId = el.getAttribute("layerId") || undefined;
      const x = parseFloat(el.getAttribute("x") || "0");
      const y = parseFloat(el.getAttribute("y") || "0");
      const rotation = parseFloat(el.getAttribute("rotation") || "0");
      const width = parseFloat(el.getAttribute("width") || "50");
      const depth = parseFloat(el.getAttribute("depth") || "50");
      const height = parseFloat(el.getAttribute("height") || "50");
      const elevation = el.hasAttribute("elevation") ? parseFloat(el.getAttribute("elevation")!) : undefined;
      const scale = el.hasAttribute("scale") ? parseFloat(el.getAttribute("scale")!) : undefined;
      const fileData = el.querySelector("fileData")?.textContent || undefined;

      parsedModels.push({
        id,
        name,
        category,
        modelType,
        builtinId,
        fileName,
        layerId,
        transform: { x, y, rotation },
        width,
        depth,
        height,
        elevation,
        scale,
        fileData,
      });
    });
  }

  const layout: Layout = {
    name,
    scale,
    feedVoltage,
    catenaryFeedVoltage,
    catenaryCircuitMode,
    locoCurrentMa,
    locoCount,
    turnoutLanternCurrentMa,
    turnoutDecoderCurrentMa,
    pieces,
    models: parsedModels.length > 0 ? parsedModels : undefined,
    layers: parsedLayers.length > 0 ? parsedLayers : undefined,
    activeLayerId,
    baseboard: parsedBaseboard,
    baseboards: parsedBaseboards.length > 0 ? parsedBaseboards : undefined,
  };

  return ensureDefaultLayers(layout);
}

function escapeXml(unsafe: string): string {
  return unsafe.replace(/[<>&'"]/g, (c) => {
    switch (c) {
      case "<": return "&lt;";
      case ">": return "&gt;";
      case "&": return "&amp;";
      case "'": return "&apos;";
      case '"': return "&quot;";
      default: return c;
    }
  });
}
