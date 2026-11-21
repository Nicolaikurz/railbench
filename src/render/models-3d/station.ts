import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedMaterial,
} from "./primitive-cache";

let _sharedMainRoofGeo: THREE.ExtrudeGeometry | null = null;
let _sharedWingRoofGeo: THREE.ExtrudeGeometry | null = null;
let _sharedSpireGeo: THREE.ConeGeometry | null = null;

function getSharedMainRoofGeo(): THREE.ExtrudeGeometry {
  if (!_sharedMainRoofGeo) {
    const s = new THREE.Shape();
    s.moveTo(-48, 0);
    s.lineTo(0, 32);
    s.lineTo(48, 0);
    s.closePath();
    _sharedMainRoofGeo = new THREE.ExtrudeGeometry(s, { depth: 150, bevelEnabled: false });
    _sharedMainRoofGeo.rotateY(Math.PI / 2);
    _sharedMainRoofGeo.translate(-75, 76, -12);
    _sharedMainRoofGeo.userData.shared = true;
  }
  return _sharedMainRoofGeo;
}

function getSharedWingRoofGeo(): THREE.ExtrudeGeometry {
  if (!_sharedWingRoofGeo) {
    const s = new THREE.Shape();
    s.moveTo(-42, 0);
    s.lineTo(0, 24);
    s.lineTo(42, 0);
    s.closePath();
    _sharedWingRoofGeo = new THREE.ExtrudeGeometry(s, { depth: 92, bevelEnabled: false });
    _sharedWingRoofGeo.rotateY(Math.PI / 2);
    _sharedWingRoofGeo.translate(-46, 54, -12);
    _sharedWingRoofGeo.userData.shared = true;
  }
  return _sharedWingRoofGeo;
}

function getSharedSpireGeo(): THREE.ConeGeometry {
  if (!_sharedSpireGeo) {
    _sharedSpireGeo = new THREE.ConeGeometry(28, 36, 4);
    _sharedSpireGeo.rotateY(Math.PI / 4);
    _sharedSpireGeo.userData.shared = true;
  }
  return _sharedSpireGeo;
}

/**
 * 1. BAHNHOF NEUSTADT — Überarbeitetes Gründerzeit-Empfangsgebäude (320 × 140 × 150 mm)
 * Symmetrischer 3-Flügel-Bau (Mittelbau + 2 Seitenflügel), 4-seitiger Uhrturm mit Zifferblättern,
 * verglastes Bahnsteigdach mit Gusseisensäulen, Bänken, Zugziel-/Bahnhofsschildern & Beleuchtung.
 */
export function buildStation3D(group: THREE.Group): void {
  // Materialien (alle aus dem Shared-Cache)
  const stoneBaseMat = getSharedMaterial(0x64748b, 0.88);
  const platformMat = getSharedMaterial(0x94a3b8, 0.85);
  const forecourtMat = getSharedMaterial(0xcbd5e1, 0.82);
  const tactileMat = getSharedMaterial(0xfef08a, 0.7);
  const wallMat = getSharedMaterial(0xe5d6be, 0.78); // Warmer Sandstein / Putz
  const brickTrimMat = getSharedMaterial(0x9a3412, 0.76); // Klinker-Ecken & Fensterstürze
  const corniceMat = getSharedMaterial(0xd6d3d1, 0.72); // Helles Sandstein-Gesims
  const roofMat = getSharedMaterial(0x991b1b, 0.68); // Biberschwanz-Ziegelrot
  const ridgeMat = getSharedMaterial(0x7c2d12, 0.7);
  const spireMat = getSharedMaterial(0x2d5a4c, 0.45, 0.35); // Kupferpatina-Turmhelm
  const ironMat = getSharedMaterial(0x1e293b, 0.4, 0.72); // Dunkles Gusseisen
  const canopyRoofMat = getSharedMaterial(0x334155, 0.52, 0.45);
  const skylightMat = getSharedMaterial(0x93c5fd, 0.15, 0.75);
  const glassDarkMat = getSharedMaterial(0x1e3a5f, 0.12, 0.85);
  const glassWarmMat = getSharedMaterial(0xfde68a, 0.2, 0.25, 0xf59e0b, 0.45);
  const frameMat = getSharedMaterial(0xf8fafc, 0.65);
  const woodDoorMat = getSharedMaterial(0x5c2e14, 0.72);
  const benchWoodMat = getSharedMaterial(0x92400e, 0.78);
  const signBlueMat = getSharedMaterial(0x1d4ed8, 0.45, 0.15);
  const signWhiteMat = getSharedMaterial(0xf8fafc, 0.55);
  const brassMat = getSharedMaterial(0xd97706, 0.3, 0.82);
  const lampGlowMat = getSharedMaterial(0xfef08a, 0.2, 0.1, 0xfacc15, 0.9);
  const planterMat = getSharedMaterial(0x57534e, 0.88);
  const foliageMat = getSharedMaterial(0x15803d, 0.9);

  // =========================================================================
  // 1. SOCKEL, BAHNSTEIG (+Z) & VORPLATZ (-Z) innerhalb 320 × 140 mm
  // =========================================================================
  const plinth = new THREE.Mesh(getSharedBoxGeometry(320, 4, 140), stoneBaseMat);
  plinth.position.set(0, 2, 0);
  group.add(plinth);

  // Gepflasterter Hausbahnsteig (Gleisseite +Z)
  const platform = new THREE.Mesh(getSharedBoxGeometry(316, 2.2, 42), platformMat);
  platform.position.set(0, 5.1, 48);
  group.add(platform);

  // Bahnsteigkante + Sicherheitsstreifen (gelb/weiß)
  const edgeCurb = new THREE.Mesh(getSharedBoxGeometry(316, 2.5, 3), corniceMat);
  edgeCurb.position.set(0, 5.25, 68.5);
  group.add(edgeCurb);

  const tactileLine = new THREE.Mesh(getSharedBoxGeometry(312, 0.4, 2.2), tactileMat);
  tactileLine.position.set(0, 6.3, 63.5);
  group.add(tactileLine);

  // Gepflasterter Vorplatz (Stadtseite -Z)
  const forecourt = new THREE.Mesh(getSharedBoxGeometry(316, 1.2, 20), forecourtMat);
  forecourt.position.set(0, 4.6, -60);
  group.add(forecourt);

  // =========================================================================
  // 2. HAUPTGEBÄUDE (MITTELBAU) + 2 SEITENFLÜGEL
  // =========================================================================
  // Gebäudesockel (Naturstein)
  const mainSockel = new THREE.Mesh(getSharedBoxGeometry(142, 8, 88), stoneBaseMat);
  mainSockel.position.set(0, 8, -12);
  group.add(mainSockel);

  // Mittelbau (2-geschossig)
  const mainBody = new THREE.Mesh(getSharedBoxGeometry(140, 68, 86), wallMat);
  mainBody.position.set(0, 42, -12);
  group.add(mainBody);

  // Geschossgesims & Traufgesims am Mittelbau
  const midBelt = new THREE.Mesh(getSharedBoxGeometry(143, 2.6, 89), corniceMat);
  midBelt.position.set(0, 41, -12);
  group.add(midBelt);

  const topCornice = new THREE.Mesh(getSharedBoxGeometry(145, 3, 91), corniceMat);
  topCornice.position.set(0, 75, -12);
  group.add(topCornice);

  // Klinker-Ecklisenen am Mittelbau
  const mainPilasterGeo = getSharedBoxGeometry(4.5, 68, 4.5);
  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      const pil = new THREE.Mesh(mainPilasterGeo, brickTrimMat);
      pil.position.set(sx * 68.5, 42, -12 + sz * 41.5);
      group.add(pil);
    }
  }

  // Steiles Satteldach Mittelbau + Firstkappe
  const mainRoof = new THREE.Mesh(getSharedMainRoofGeo(), roofMat);
  group.add(mainRoof);

  const mainRidge = new THREE.Mesh(getSharedBoxGeometry(151, 2.5, 3.5), ridgeMat);
  mainRidge.position.set(0, 108.2, -12);
  group.add(mainRidge);

  // Dachgauben (2 Stück auf Vorplatz- & Gleisseite neben dem Turm)
  const dormerWallGeo = getSharedBoxGeometry(16, 12, 16);
  const dormerRoofGeo = getSharedBoxGeometry(19, 3, 19);
  const dormerWinGeo = getSharedBoxGeometry(8, 7, 1);
  for (const dx of [-42, 42]) {
    for (const side of [-1, 1]) {
      const dz = -12 + side * 26;
      const dw = new THREE.Mesh(dormerWallGeo, wallMat);
      dw.position.set(dx, 87, dz);
      group.add(dw);
      const dr = new THREE.Mesh(dormerRoofGeo, roofMat);
      dr.position.set(dx, 94, dz);
      group.add(dr);
      const dwin = new THREE.Mesh(dormerWinGeo, glassWarmMat);
      dwin.position.set(dx, 87.5, dz + side * 8.2);
      group.add(dwin);
    }
  }

  // Backstein-Schornstein
  const chimney = new THREE.Mesh(getSharedBoxGeometry(10, 26, 10), brickTrimMat);
  chimney.position.set(52, 100, -24);
  group.add(chimney);
  const chimneyCap = new THREE.Mesh(getSharedBoxGeometry(12.5, 2.5, 12.5), corniceMat);
  chimneyCap.position.set(52, 113.5, -24);
  group.add(chimneyCap);

  // Linker & Rechter Seitenflügel (symmetrisch bei x = ±112)
  const wingSockelGeo = getSharedBoxGeometry(88, 7, 76);
  const wingBodyGeo = getSharedBoxGeometry(86, 48, 74);
  const wingCorniceGeo = getSharedBoxGeometry(90, 2.6, 78);
  const wingPilasterGeo = getSharedBoxGeometry(4, 48, 4);
  const wingRidgeGeo = getSharedBoxGeometry(93, 2.2, 3);

  for (const sideX of [-1, 1]) {
    const wx = sideX * 112;
    const ws = new THREE.Mesh(wingSockelGeo, stoneBaseMat);
    ws.position.set(wx, 7.5, -12);
    group.add(ws);

    const wb = new THREE.Mesh(wingBodyGeo, wallMat);
    wb.position.set(wx, 31, -12);
    group.add(wb);

    const wc = new THREE.Mesh(wingCorniceGeo, corniceMat);
    wc.position.set(wx, 54, -12);
    group.add(wc);

    // Ecklisenen an Außenkanten der Flügel
    for (const sz of [-1, 1]) {
      const wp = new THREE.Mesh(wingPilasterGeo, brickTrimMat);
      wp.position.set(wx + sideX * 41.5, 31, -12 + sz * 35.5);
      group.add(wp);
    }

    const wr = new THREE.Mesh(getSharedWingRoofGeo(), roofMat);
    wr.position.set(wx, 0, 0);
    group.add(wr);

    const wridge = new THREE.Mesh(wingRidgeGeo, ridgeMat);
    wridge.position.set(wx, 78.2, -12);
    group.add(wridge);

    // Giebel-Fenster an den Außenwänden (x = ±155) — sauber gestaffelt ohne Z-Fighting
    const gableFrame = new THREE.Mesh(getSharedBoxGeometry(1.0, 16.5, 12.5), frameMat);
    gableFrame.position.set(wx + sideX * 43.2, 34, -12);
    group.add(gableFrame);
    const gableWin = new THREE.Mesh(getSharedBoxGeometry(0.8, 14, 10), glassWarmMat);
    gableWin.position.set(wx + sideX * 43.8, 34, -12);
    group.add(gableWin);
    const gableSill = new THREE.Mesh(getSharedBoxGeometry(2.6, 1.6, 14.5), corniceMat);
    gableSill.position.set(wx + sideX * 44.0, 25, -12);
    group.add(gableSill);
    const gableLintel = new THREE.Mesh(getSharedBoxGeometry(1.8, 2.2, 14), brickTrimMat);
    gableLintel.position.set(wx + sideX * 43.6, 43.2, -12);
    group.add(gableLintel);
  }

  // =========================================================================
  // 3. ZENTRALER UHRTURM MIT 4-SEITIGEN BAHNHOFSUHREN
  // =========================================================================
  const towerBody = new THREE.Mesh(getSharedBoxGeometry(36, 114, 36), wallMat);
  towerBody.position.set(0, 61, -12);
  group.add(towerBody);

  // Klinker-Eckstreifen am Turm
  const towerCornerGeo = getSharedBoxGeometry(3.2, 42, 3.2);
  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      const tc = new THREE.Mesh(towerCornerGeo, brickTrimMat);
      tc.position.set(sx * 16.8, 98, -12 + sz * 16.8);
      group.add(tc);
    }
  }

  // Turmgesimse
  const towerBelt1 = new THREE.Mesh(getSharedBoxGeometry(39, 2.5, 39), corniceMat);
  towerBelt1.position.set(0, 92, -12);
  group.add(towerBelt1);

  const towerBelt2 = new THREE.Mesh(getSharedBoxGeometry(41, 3, 41), corniceMat);
  towerBelt2.position.set(0, 118, -12);
  group.add(towerBelt2);

  // 4-seitige Bahnhofsuhren am Uhrturm (Vorder-/Rückseite & beide Flanken)
  const clockRimGeo = getSharedCylinderGeometry(7.8, 7.8, 1.2, 16);
  const clockDialGeo = getSharedCylinderGeometry(6.6, 6.6, 1.5, 16);
  const clockHandMinZGeo = getSharedBoxGeometry(0.9, 5.0, 0.4);
  const clockHandHrZGeo = getSharedBoxGeometry(3.8, 1.0, 0.4);
  const clockHandMinXGeo = getSharedBoxGeometry(0.4, 5.0, 0.9);
  const clockHandHrXGeo = getSharedBoxGeometry(0.4, 1.0, 3.8);

  for (const side of [-1, 1] as const) {
    const cz = -12 + side * 18.2;
    const rimZ = new THREE.Mesh(clockRimGeo, ironMat);
    rimZ.rotation.x = Math.PI / 2;
    rimZ.position.set(0, 104, cz);
    group.add(rimZ);
    const dialZ = new THREE.Mesh(clockDialGeo, signWhiteMat);
    dialZ.rotation.x = Math.PI / 2;
    dialZ.position.set(0, 104, cz + side * 0.15);
    group.add(dialZ);
    const minZ = new THREE.Mesh(clockHandMinZGeo, ironMat);
    minZ.position.set(0, 106.2, cz + side * 0.95);
    group.add(minZ);
    const hrZ = new THREE.Mesh(clockHandHrZGeo, ironMat);
    hrZ.position.set(1.6, 104, cz + side * 0.95);
    group.add(hrZ);

    const cx = side * 18.2;
    const rimX = new THREE.Mesh(clockRimGeo, ironMat);
    rimX.rotation.z = Math.PI / 2;
    rimX.position.set(cx, 104, -12);
    group.add(rimX);
    const dialX = new THREE.Mesh(clockDialGeo, signWhiteMat);
    dialX.rotation.z = Math.PI / 2;
    dialX.position.set(cx + side * 0.15, 104, -12);
    group.add(dialX);
    const minX = new THREE.Mesh(clockHandMinXGeo, ironMat);
    minX.position.set(cx + side * 0.95, 106.2, -12);
    group.add(minX);
    const hrX = new THREE.Mesh(clockHandHrXGeo, ironMat);
    hrX.position.set(cx + side * 0.95, 104, -10.4);
    group.add(hrX);
  }

  // Kupferpatina-Spitzhelm + Turmspitze
  const spire = new THREE.Mesh(getSharedSpireGeo(), spireMat);
  spire.position.set(0, 137.5, -12);
  group.add(spire);

  const finial = new THREE.Mesh(getSharedCylinderGeometry(0.7, 1.2, 14, 8), brassMat);
  finial.position.set(0, 160, -12);
  group.add(finial);

  const finialBall = new THREE.Mesh(getSharedCylinderGeometry(2.2, 2.2, 2.2, 10), brassMat);
  finialBall.position.set(0, 156, -12);
  group.add(finialBall);

  // =========================================================================
  // 4. FENSTER, PORTALE & BAHNHOFSSCHILDER (GLEIS- & STADTSEITE)
  // =========================================================================
  const winFrameGeo = getSharedBoxGeometry(13, 19, 1.2);
  const winGlassGeo = getSharedBoxGeometry(10.5, 16.5, 1.4);
  const winSillGeo = getSharedBoxGeometry(15.5, 1.8, 3.2);
  const winLintelGeo = getSharedBoxGeometry(14.5, 2.4, 2.0);
  const mullionVGeo = getSharedBoxGeometry(0.9, 16.5, 1.7);
  const mullionHGeo = getSharedBoxGeometry(10.5, 0.9, 1.7);

  const addWindow = (wx: number, wy: number, wallZ: number, side: 1 | -1, warm: boolean) => {
    const fz = wallZ + side * 0.5;
    const frame = new THREE.Mesh(winFrameGeo, frameMat);
    frame.position.set(wx, wy, fz);
    group.add(frame);

    const glass = new THREE.Mesh(winGlassGeo, warm ? glassWarmMat : glassDarkMat);
    glass.position.set(wx, wy, wallZ + side * 0.7);
    group.add(glass);

    const mv = new THREE.Mesh(mullionVGeo, frameMat);
    mv.position.set(wx, wy, wallZ + side * 0.85);
    group.add(mv);

    const mh = new THREE.Mesh(mullionHGeo, frameMat);
    mh.position.set(wx, wy + 2, wallZ + side * 0.85);
    group.add(mh);

    const sill = new THREE.Mesh(winSillGeo, corniceMat);
    sill.position.set(wx, wy - 10.2, wallZ + side * 1.4);
    group.add(sill);

    const lintel = new THREE.Mesh(winLintelGeo, brickTrimMat);
    lintel.position.set(wx, wy + 10.6, wallZ + side * 0.9);
    group.add(lintel);
  };

  // Mittelbau: Erdgeschoss & Obergeschoss Fenster (beide Seiten)
  for (const side of [1, -1] as const) {
    const mainWallZ = -12 + side * 43;
    // EG (links & rechts vom Portal)
    for (const dx of [-46, -26, 26, 46]) {
      addWindow(dx, 24, mainWallZ, side, Math.abs(dx) === 26);
    }
    // OG (5 Fenster symmetrisch)
    for (const dx of [-46, -23, 0, 23, 46]) {
      addWindow(dx, 56, mainWallZ, side, dx === 0 || Math.abs(dx) === 46);
    }

    // Haupteingangs-Portal (mittig bei x = 0)
    const portalPillarGeo = getSharedBoxGeometry(4, 30, 5);
    for (const px of [-12, 12]) {
      const pp = new THREE.Mesh(portalPillarGeo, corniceMat);
      pp.position.set(px, 21, mainWallZ + side * 2);
      group.add(pp);
    }
    const portalArch = new THREE.Mesh(getSharedBoxGeometry(30, 4.5, 6), corniceMat);
    portalArch.position.set(0, 36.5, mainWallZ + side * 2.5);
    group.add(portalArch);

    // Doppelflügel-Holztür + beleuchtetes Oberlicht
    const doorFrame = new THREE.Mesh(getSharedBoxGeometry(20, 29, 1.4), ironMat);
    doorFrame.position.set(0, 20.5, mainWallZ + side * 0.6);
    group.add(doorFrame);

    for (const dx of [-4.5, 4.5]) {
      const leaf = new THREE.Mesh(getSharedBoxGeometry(8.2, 22, 1.2), woodDoorMat);
      leaf.position.set(dx, 17, mainWallZ + side * 1.2);
      group.add(leaf);
    }
    const transom = new THREE.Mesh(getSharedBoxGeometry(17, 5, 1.2), glassWarmMat);
    transom.position.set(0, 31.5, mainWallZ + side * 1.2);
    group.add(transom);

    // Eingangsstufen
    const step1 = new THREE.Mesh(getSharedBoxGeometry(28, 2, 7), corniceMat);
    step1.position.set(0, 5.2, mainWallZ + side * 3.5);
    group.add(step1);

    // Seitenflügel-Fenster (je 3 Fenster pro Flügel & Seite)
    const wingWallZ = -12 + side * 37;
    for (const sideX of [-1, 1]) {
      const wx = sideX * 112;
      for (const ox of [-24, 0, 24]) {
        addWindow(wx + ox, 27, wingWallZ, side, ox === 0);
      }
    }
  }

  // Bahnhofsschild über dem Haupteingang (Vorplatzseite -Z)
  const frontSignBoard = new THREE.Mesh(getSharedBoxGeometry(56, 7.5, 1.4), signBlueMat);
  frontSignBoard.position.set(0, 44.5, -56.5);
  group.add(frontSignBoard);
  const frontSignText = new THREE.Mesh(getSharedBoxGeometry(44, 2.6, 1.6), signWhiteMat);
  frontSignText.position.set(0, 44.5, -56.6);
  group.add(frontSignText);

  // Dekorative Pflanzkübel am Vorplatz-Eingang
  const planterGeo = getSharedBoxGeometry(10, 6, 10);
  const shrubGeo = getSharedCylinderGeometry(4.5, 3.2, 8, 8);
  for (const px of [-24, 24]) {
    const pot = new THREE.Mesh(planterGeo, planterMat);
    pot.position.set(px, 8, -60);
    group.add(pot);
    const shrub = new THREE.Mesh(shrubGeo, foliageMat);
    shrub.position.set(px, 14, -60);
    group.add(shrub);
  }

  // =========================================================================
  // 5. VERGLASTES BAHNSTEIG-VORDACH, GUSSEISENSÄULEN & BAHNSTEIGDETAILS (+Z)
  // =========================================================================
  // Schräges Bahnsteigdach mit zentralem Lichtband (Glas) & Flügelanschluss ohne Luftspalt
  const wingCanopyTieGeo = getSharedBoxGeometry(80, 2.2, 6.4);
  for (const sideX of [-1, 1]) {
    const tie = new THREE.Mesh(wingCanopyTieGeo, canopyRoofMat);
    tie.position.set(sideX * 109, 42.0, 27.8);
    tie.rotation.x = 0.08;
    group.add(tie);
  }

  const canopyBack = new THREE.Mesh(getSharedBoxGeometry(298, 2.2, 13), canopyRoofMat);
  canopyBack.position.set(0, 41.2, 36);
  canopyBack.rotation.x = 0.08;
  group.add(canopyBack);

  const canopyGlass = new THREE.Mesh(getSharedBoxGeometry(294, 1.2, 11), skylightMat);
  canopyGlass.position.set(0, 40.3, 47.5);
  canopyGlass.rotation.x = 0.08;
  group.add(canopyGlass);

  const canopyFront = new THREE.Mesh(getSharedBoxGeometry(298, 2.2, 13), canopyRoofMat);
  canopyFront.position.set(0, 39.3, 59);
  canopyFront.rotation.x = 0.08;
  group.add(canopyFront);

  // Vordach-Stirnblende & Regenrinne
  const fascia = new THREE.Mesh(getSharedBoxGeometry(298, 3.5, 1.6), ironMat);
  fascia.position.set(0, 38.2, 65.5);
  group.add(fascia);

  // Gusseisensäulen mit Steinsockel & Kopfband-Streben
  const colPlinthGeo = getSharedBoxGeometry(6, 5, 6);
  const colShaftGeo = getSharedCylinderGeometry(1.7, 2.1, 31, 10);
  const colCapitalGeo = getSharedBoxGeometry(5, 2.2, 5);
  const rafterInnerGeo = getSharedBoxGeometry(2.2, 2.4, 36);
  const rafterOuterGeo = getSharedBoxGeometry(2.2, 2.4, 41);

  for (let x = -130; x <= 130; x += 52) {
    const cp = new THREE.Mesh(colPlinthGeo, stoneBaseMat);
    cp.position.set(x, 8.5, 58);
    group.add(cp);

    const shaft = new THREE.Mesh(colShaftGeo, ironMat);
    shaft.position.set(x, 25, 58);
    group.add(shaft);

    const cap = new THREE.Mesh(colCapitalGeo, ironMat);
    cap.position.set(x, 38.5, 58);
    group.add(cap);

    // Dachsparren vom Gebäude zur Säule (Außenflügel bis Z = 25)
    const isWing = Math.abs(x) > 70;
    const rafter = new THREE.Mesh(isWing ? rafterOuterGeo : rafterInnerGeo, ironMat);
    rafter.position.set(x, isWing ? 39.0 : 38.8, isWing ? 43 : 46);
    rafter.rotation.x = 0.08;
    group.add(rafter);
  }

  // Hängende Bahnhofsschilder (DB-Blau/Weiß) & Bahnsteiglampen unter dem Dach
  const hangSignGeo = getSharedBoxGeometry(30, 5.5, 1.2);
  const hangSignStripeGeo = getSharedBoxGeometry(24, 1.8, 1.5);
  const lampShadeGeo = getSharedCylinderGeometry(2.8, 3.8, 2.2, 10);
  const lampBulbGeo = getSharedCylinderGeometry(1.6, 1.6, 1.5, 8);

  for (const sx of [-78, 0, 78]) {
    const sb = new THREE.Mesh(hangSignGeo, signBlueMat);
    sb.position.set(sx, 34, 56);
    group.add(sb);
    const st = new THREE.Mesh(hangSignStripeGeo, signWhiteMat);
    st.position.set(sx, 34, 56);
    group.add(st);
  }

  for (const lx of [-104, -52, 52, 104]) {
    const shade = new THREE.Mesh(lampShadeGeo, ironMat);
    shade.position.set(lx, 36.8, 49);
    group.add(shade);
    const bulb = new THREE.Mesh(lampBulbGeo, lampGlowMat);
    bulb.position.set(lx, 35.4, 49);
    group.add(bulb);
  }

  // Bahnsteig-Sitzbänke (Holzlatten + Eisengestell)
  const benchSeatGeo = getSharedBoxGeometry(22, 1.2, 5.5);
  const benchBackGeo = getSharedBoxGeometry(22, 4.5, 1.0);
  const benchLegGeo = getSharedBoxGeometry(1.2, 4.5, 5.0);

  for (const bx of [-95, -40, 40, 95]) {
    const seat = new THREE.Mesh(benchSeatGeo, benchWoodMat);
    seat.position.set(bx, 10.2, 42);
    group.add(seat);
    const back = new THREE.Mesh(benchBackGeo, benchWoodMat);
    back.position.set(bx, 13.0, 39.5);
    group.add(back);
    for (const lx of [-8.5, 8.5]) {
      const leg = new THREE.Mesh(benchLegGeo, ironMat);
      leg.position.set(bx + lx, 8.4, 42);
      group.add(leg);
    }
  }

  // Fahrplan-Vitrine bündig an der Hauptwand (+Z = 31) neben dem Bahnsteig-Eingang
  const vitrineFrame = new THREE.Mesh(getSharedBoxGeometry(14, 16, 2.2), ironMat);
  vitrineFrame.position.set(22, 18, 32.0);
  group.add(vitrineFrame);
  const vitrineSheet = new THREE.Mesh(getSharedBoxGeometry(11.5, 13, 1.8), signWhiteMat);
  vitrineSheet.position.set(22, 18, 32.35);
  group.add(vitrineSheet);
}

/** Vereinfachte Low-Poly Variante des Bahnhofs (gleiche 3-Flügel-Silhouette, LOD >= 3500 mm) */
export function buildStationLowPoly3D(group: THREE.Group): void {
  const stoneBaseMat = getSharedMaterial(0x64748b, 0.88);
  const wallMat = getSharedMaterial(0xe5d6be, 0.78);
  const roofMat = getSharedMaterial(0x991b1b, 0.68);
  const spireMat = getSharedMaterial(0x2d5a4c, 0.45, 0.35);
  const canopyMat = getSharedMaterial(0x334155, 0.52, 0.45);

  const plinth = new THREE.Mesh(getSharedBoxGeometry(320, 5, 140), stoneBaseMat);
  plinth.position.set(0, 2.5, 0);
  group.add(plinth);

  const mainBody = new THREE.Mesh(getSharedBoxGeometry(140, 71, 86), wallMat);
  mainBody.position.set(0, 40.5, -12);
  group.add(mainBody);

  const mainRoof = new THREE.Mesh(getSharedMainRoofGeo(), roofMat);
  group.add(mainRoof);

  for (const sideX of [-1, 1]) {
    const wb = new THREE.Mesh(getSharedBoxGeometry(86, 50, 74), wallMat);
    wb.position.set(sideX * 112, 30, -12);
    group.add(wb);

    const wr = new THREE.Mesh(getSharedWingRoofGeo(), roofMat);
    wr.position.set(sideX * 112, 0, 0);
    group.add(wr);
  }

  const tower = new THREE.Mesh(getSharedBoxGeometry(36, 114, 36), wallMat);
  tower.position.set(0, 61, -12);
  group.add(tower);

  const spire = new THREE.Mesh(getSharedSpireGeo(), spireMat);
  spire.position.set(0, 137.5, -12);
  group.add(spire);

  const canopy = new THREE.Mesh(getSharedBoxGeometry(298, 2.5, 38), canopyMat);
  canopy.position.set(0, 40, 48);
  group.add(canopy);
}
