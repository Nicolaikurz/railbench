/**
 * The height panel template (templates/height-panel.html) is authored in German.
 * For the English UI, its static texts are translated with this dictionary.
 */
const DE_TO_EN: Record<string, string> = {
  "Ziehen zum Verschieben (Doppelklick zum Zurücksetzen)": "Drag to move (double-click to reset)",
  "Höhe & Rampe": "Height & Ramp",
  "Schließen (Esc)": "Close (Esc)",
  "Höhe / Ebene": "Height / Level",
  "Steigung": "Slope",
  "Automatisch": "Automatic",
  "0 Gleise": "0 tracks",
  "Zielebene": "Target level",
  "Höhen-Modus": "Height mode",
  "Ebenenhöhe": "Level height",
  "Eigener Wert": "Custom value",
  "Absolute Höhe (mm)": "Absolute height (mm)",
  "Gleise werden auf Ebenenhöhe gesetzt.": "Tracks will be set to level height.",
  "Start": "Start",
  "Ende": "End",
  "Gleis im Gleisplan anklicken": "Click a track in the layout",
  "Gleis wählen...": "Pick track...",
  "Start und Ende vertauschen": "Swap start and end",
  "Verbunden": "Continuous",
  "Maximale Steigung": "Maximum slope",
  "Steigungsprofil": "Slope profile",
  "Linear (Gleichmäßig)": "Linear (uniform)",
  "S-Kurve (Ausgerundet)": "S-curve (eased)",
  "Zielebene für Gleise": "Target level for tracks",
  "Steigung: 0.0%": "Slope: 0.0%",
  "Berechnung aktiv...": "Calculating...",
  "▸ Gleis-Details anzeigen": "▸ Show track details",
  "Start-Gleis & Anschluss": "Start track & connector",
  "Anschluss im Gleisplan anklicken": "Click a connector in the layout",
  "Im Plan wählen": "Pick in layout",
  "Zielhöhe": "Target height",
  "Steigung nach oben": "Slope up",
  "↑ Steigung": "↑ Slope up",
  "Gefälle nach unten": "Slope down",
  "↓ Gefälle": "↓ Slope down",
  "Form der Rampe": "Ramp shape",
  "Gerade": "Straight",
  "Bogen": "Curve",
  "Wendel": "Helix",
  "Kurvenradius / Gleis": "Curve radius / track",
  "Bogenrichtung": "Curve direction",
  "↰ Links": "↰ Left",
  "↱ Rechts": "↱ Right",
  "Winkel": "Angle",
  "Ziel-Steigung": "Target slope",
  "Vorschau wird auf dem Plan dargestellt.": "Preview is shown on the layout.",
  "Abbrechen": "Cancel",
  "Übernehmen": "Apply",
};

/** Translates all static German texts, titles and placeholders inside `root` to English. */
export function translateHeightPanelToEn(root: HTMLElement): void {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  for (let n = walker.nextNode(); n; n = walker.nextNode()) nodes.push(n as Text);
  for (const node of nodes) {
    const raw = node.nodeValue ?? "";
    const key = raw.trim();
    const tr = DE_TO_EN[key];
    if (tr) node.nodeValue = raw.replace(key, tr);
  }
  root.querySelectorAll<HTMLElement>("[title],[placeholder]").forEach((el) => {
    for (const attr of ["title", "placeholder"]) {
      const v = el.getAttribute(attr);
      if (v && DE_TO_EN[v]) el.setAttribute(attr, DE_TO_EN[v]!);
    }
  });
}
