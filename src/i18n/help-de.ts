import type { HelpDialogTranslations } from "./types";

export const helpDe: HelpDialogTranslations = {
  modalTitle: "Railbench – Benutzerhandbuch & Funktionsübersicht",
  closeTitle: "Hilfe schließen (Esc)",
  closeBtnText: "Handbuch schließen",
  footerTip:
    "Tipp: Drücken Sie jederzeit <strong>F1</strong>, um diese Hilfe zu öffnen, oder <strong>M</strong> bei ausgewählten Gleisen für Höhen- und Rampenanpassung.",
  copiedText: "✓ In Zwischenablage kopiert!",
  tabs: {
    overview: "Erste Schritte",
    layers: "Ebenen & Höhen",
    print: "Ebenen-Druck",
    power: "Spannungs-Simulation",
    baseboard: "Tisch & Platte",
    view3d: "3D-Ansicht",
    server: "Multi-User & Server",
    shortcuts: "Tastaturkürzel",
    catalog: "Gleiskatalog (JSON)",
    about: "Über Railbench & Gleise",
  },
  overview: {
    title: "Erste Schritte mit Railbench",
    intro:
      "<strong>Railbench</strong> ist eine moderne, hochpräzise CAD-Gleisplanungssoftware für den Modellbahn-Bereich. Sie verbindet millimetergenaue CAD-Geometrien mit elektrischer Traktionssimulation, flexibler Mehrebenen-Verwaltung und druckfertigen Bauplänen.",
    cards: [
      {
        title: "1. Gleise hinzufügen",
        body: "Wählen Sie Gleise aus dem <strong>linken Katalog</strong> (Taste <code>B</code> zum Ein-/Ausblenden). Klicken Sie auf ein Gleis oder ziehen Sie es per <strong>Drag & Drop</strong> direkt auf die Arbeitsfläche.",
      },
      {
        title: "2. Intelligentes Einrasten (Snapping)",
        body: "Das automatische magnetische Einrasten (Taste <code>S</code>) verbindet Gleisenden nahtlos, berechnet exakte Winkel und schließt Verbindungen im Millimeterbereich.",
      },
      {
        title: "3. Drehen & Verschieben",
        body: "Gleise mit der Maus ziehen zum Verschieben. Drehen in 7,5°-Schritten mit Taste <code>R</code> (bzw. <code>Umschalt+R</code> gegen den Uhrzeigersinn). Halten Sie <code>Umschalt + Mausrad</code> für feines Drehen oder nutzen Sie das direkte Winkelfeld in der unteren Aktionsleiste!",
      },
      {
        title: "4. Mehrfachauswahl (Aufziehrahmen)",
        body: "Klicken Sie direkt auf ein Gleis, um es auszuwählen. Klicken Sie mit der <strong>linken Maustaste in einen freien Bereich</strong>, halten Sie gedrückt und ziehen Sie ein Rechteck auf (Marquee Box). Alle darin liegenden Gleise werden markiert. Halten Sie <code>Strg</code> oder <code>Umschalt</code> gedrückt, um bestehende Auswahlen zu erweitern oder Gleise einzeln hinzuzufügen/abzuwählen.",
      },
    ],
    callout:
      "<strong>Navigationstipp:</strong> Halten Sie die <strong>rechte Maustaste</strong> oder das <strong>Mausrad (Mittelklick)</strong> gedrückt, um die Arbeitsfläche frei zu verschieben (Pan). Drehen Sie das <strong>Mausrad</strong> zum stufenlosen Zoomen. Drücken Sie jederzeit <code>Strg+0</code>, um den Plan zu zentrieren.",
  },
  layers: {
    title: "Mehrebenen-Verwaltung & Höhenplanung",
    intro:
      "Mit Railbench können Sie mehrstöckige Modellbahnanlagen (z. B. Grundebene, Bergetappen, Rampen und verdeckte Schattenbahnhöfe) mit echten vertikalen Millimeterhöhen konstruieren.",
    cards: [
      {
        title: "Schwebendes Ebenen-Menü",
        body: "Drücken Sie <code>Umschalt+B</code> oder klicken Sie auf das <strong>Ebenen</strong>-Symbol in der Leiste. Die Ebenen sind nach ihrer Höhe sortiert (höchste Ebene oben).",
      },
      {
        title: "Sichtbarkeit & Sperrschutz",
        body: "Klicken Sie auf das <strong>Auge-Symbol</strong>, um eine Ebene auszublenden, wenn Sie darunterliegende Trassen bearbeiten. Klicken Sie auf das <strong>Schloss-Symbol</strong>, um eine Ebene gegen versehentliches Verschieben zu sperren.",
      },
      {
        title: "Ebenen direkt umbenennen",
        body: "Klicken Sie auf das Stift-Symbol oder doppelklicken Sie auf den Namen, um eine Ebene direkt umzubenennen. Klicken Sie auf das Höhen-Badge (z. B. <code>+80 mm</code>), um die Basishöhe anzupassen.",
      },
      {
        title: "Gleise Ebene zuweisen (M)",
        body: 'Gleise markieren und Taste <code>M</code> drücken oder auf <strong>"↳ Verschieben & Höhe anpassen"</strong> klicken, um Gleise einer Ebene zuzuordnen und ihre Basishöhe automatisch anzugleichen.',
      },
      {
        title: "Rampen- & Steigungsrechner",
        body: "Erstellen Sie gleichmäßige Steigungen (z. B. 0 mm → 80 mm). Der Rechner teilt die Höhendifferenz durch die Gleislänge und zeigt die Steigung in Prozent an:<br>• <strong>&le; 3,0%</strong>: Empfohlen (Sanfte Steigung)<br>• <strong>3,0% – 4,5%</strong>: Mäßig (Bei schweren/langen Zügen beachten)<br>• <strong>&gt; 4,5%</strong>: Warnung (Schleudergefahr)",
      },
    ],
    callout:
      "<strong>Schwebende Auswahlleiste:</strong> Sobald Gleise markiert sind, erscheint am unteren Bildschirmrand eine Leiste, um Ebene, Höhe und Winkel mit einem Klick zu steuern!",
  },
  print: {
    title: "Ebenengetrennter PDF-Export & Plan-Druck (Strg+P)",
    intro:
      "Herkömmliche Programme exportieren oder drucken alle Ebenen unübersichtlich übereinander. Railbench bietet echten ebenengetrennten Export für den schrittweisen Anlagenbau.",
    cards: [
      {
        title: "1. Eine Seite je Ebene",
        body: "Jede Ebene wird sauber auf einem eigenen Blatt ausgegeben – ideal als maßstäbliche Bau- und Trassenunterlage.",
      },
      {
        title: "2. Exakt identischer Ausschnitt & Maßstab",
        body: "Alle exportierten Ebenenseiten teilen sich exakt denselben Ursprung, Maßstab und Begrenzungsrahmen. So können Seiten deckungsgleich übereinandergelegt werden!",
      },
      {
        title: "3. Abgeblendete Referenzkonturen",
        body: "Auf Wunsch werden über- oder unterliegende Ebenen zart angedeutet, um Kreuzungen und Pfeilerstandorte sofort im Blick zu haben.",
      },
      {
        title: "4. Integrierte Ebenen-Stückliste",
        body: "Jede Seite enthält ein Teileverzeichnis mit den für diesen Bauabschnitt benötigten Gleisnummern und Stückzahlen (z. B. <code>24188: 8×, 24172: 4×</code>).",
      },
    ],
    callout:
      '<strong>So exportieren Sie:</strong> Drücken Sie <code>Strg+P</code>, wählen Sie die Ebenen in der Vorschau aus und klicken Sie auf <strong>"System-Druck..."</strong> oder <strong>"Als PDF exportieren"</strong>.',
  },
  power: {
    title: "Spannungsabfall & Traktionsstrom-Simulation",
    intro:
      "Gewährleisten Sie einen zuverlässigen Betrieb Ihrer Lokomotiven durch die integrierte elektrische Netzanalyse von Railbench.",
    cards: [
      {
        title: "Farb-Heatmap (Taste P)",
        body: 'Taste <code>P</code> blendet die Spannungs-Heatmap ein:<br>• <strong style="color: #10b981;">Grün</strong>: Optimale Spannung (&gt; 92% der Einspeisung)<br>• <strong style="color: #f59e0b;">Gelb/Orange</strong>: Mäßiger Abfall (&gt; 80% der Einspeisung)<br>• <strong style="color: #ef4444;">Rot</strong>: Kritischer Abfall (&lt; 80% — zusätzliche Einspeisung erforderlich!)<br>• <strong style="color: #dc2626;">Gestrichelt Rot</strong>: Stromlos / vom Netz isolierter Abschnitt',
      },
      {
        title: "Einspeisepunkte (Taste F)",
        body: "Gleis markieren und <code>F</code> drücken, um einen Stromeinspeisepunkt (Feeder) zu setzen. Railbench berechnet die Spannungsverteilung sofort neu.",
      },
      {
        title: "Trennstellen & Isolierverbinder (Taste I)",
        body: "Gleis markieren und <code>I</code> drücken, um das Gleis isoliert vom Netz zu trennen. Ideal zur Planung separater Boosterkreise, Blockstrecken, Signalabschnitte oder abschaltbarer Schattenbahnhofsgleise.",
      },
      {
        title: "Spannung & Fahrstrom konfigurieren",
        body: "Klicken Sie auf das Menü am Strom-Schalter in der Toolbar und wählen Sie <strong>Spannung & Strom anpassen...</strong>:<br>• <strong>16V AC / Digital</strong>: Märklin H0 Standard<br>• <strong>14V DC / DCC</strong>: Zweileiter H0 (Piko, Roco, Trix)<br>• <strong>12V DC</strong>: Spur N & TT<br>• <strong>18V DC</strong>: Spur 1 & Gartenbahn<br>• <strong>Lokomotiv-Verbrauch (Presets)</strong>:<br>&nbsp;&nbsp;• <strong>N / Z</strong>: 50–150 mA (typ. 100 mA)<br>&nbsp;&nbsp;• <strong>TT</strong>: 200–400 mA (typ. 300 mA)<br>&nbsp;&nbsp;• <strong>H0</strong>: 300–600 mA (typ. 450 mA)<br>&nbsp;&nbsp;• <strong>Maximal pro Lok</strong>: 1 A (1000 mA)<br>• <strong>Mehrzugbetrieb</strong>: Gesamtfahrstrom wird mit der Anzahl aktiver Lokomotiven multipliziert.",
      },
    ],
  },
  baseboard: {
    title: "Grundplatte & Tischkonstruktion",
    intro:
      "Planen Sie Ihre Modellbahn passgenau für den Raum und Ihre handwerklichen Rahmenbedingungen.",
    cards: [
      {
        title: "Modulare Tischformen",
        body: "Wählen Sie zwischen vier gängigen Modellbahn-Tischformen:<br>• <strong>Rechteckig</strong>: Klassischer Anlagentisch<br>• <strong>L-Form</strong>: Eckanlage mit linkem oder rechtem Schenkel<br>• <strong>U-Form</strong>: Anlage mit Bedienertrog<br>• <strong>T-Form</strong>: Zungenanlage oder Nebenbahnausläufer",
      },
      {
        title: "Holzarten & Oberflächen",
        body: "Wählen Sie realistische Texturen wie <strong>Birke hell</strong>, <strong>Nussbaum dunkel</strong>, <strong>Multiplex</strong>, <strong>Schiefer</strong> oder neutrales <strong>CAD-Weiß</strong>.",
      },
      {
        title: "Maßketten & Lineale",
        body: "Zeigt Millimeter-Bemaßungen direkt an den Kanten an. Taste <code>Umschalt+L</code> schaltet zusätzliche Randlineale ein.",
      },
      {
        title: "Platte an Gleise anpassen",
        body: 'Ein Klick auf <strong>"Platte an Gleise anpassen"</strong> berechnet automatisch die Mindestmaße des Tisches mit Sicherheitsabstand.',
      },
    ],
  },
  view3d: {
    title: "3D-Perspektive & Ansicht",
    intro:
      "Der 3D-Viewer rendert Ihre gesamte Modellbahnanlage in Echtzeit dreidimensional inklusive Grundplatte, Brückenpfeilern, Ebenen und 3D-Ausschmückung.",
    cards: [
      {
        title: "Kamera-Navigation",
        body: '• <strong>Drehen:</strong> Linksklick und über die 3D-Szene ziehen.<br>• <strong>Verschieben:</strong> Rechtsklick und ziehen (oder Umschalt + Linksklick).<br>• <strong>Zoomen:</strong> Mausrad oder Trackpad-Geste.<br>• <strong>Zurücksetzen:</strong> "⌖ Ansicht zurücksetzen" zentriert die Kamera auf die Grundplatte.',
      },
      {
        title: "Brücken & Pfeiler",
        body: "Gleise in Höhenlagen werden vertikal akkurat dargestellt. Ab 15 mm Höhe werden automatisch Brückenpfeiler bis zur Grundplatte generiert!",
      },
      {
        title: "3D-Tisch & Unterbau",
        body: "Der Holztisch wird mit Plattenstärke und soliden Stahlfüßen gerendert.",
      },
      {
        title: "3D-Modelle & Ausschmückung (.glb / .gltf)",
        body: "Platzieren Sie Gebäude (Bahnhof, Stellwerk, Lokschuppen, Wasserturm, Fachwerkhaus, Signale, Prellböcke, Bäume, Lampen) aus dem Katalog oder importieren Sie eigene GLB/GLTF-Dateien.",
      },
      {
        title: "3D-Modell-Lizenzen & Herkunft",
        body: "• <strong>Integrierte 3D-Objekte:</strong> Alle 10 mitgelieferten 3D-Modelle sind zu <strong>100 % prozeduraler Open-Source-Code</strong> (Three.js Geometrien in <code>src/render/builtin-models-3d.ts</code>). Sie enthalten keine externen Binärmodelle oder proprietären Texturen, sind frei von Urheberrechtsansprüchen Dritter und unterliegen der freien Open-Source-Lizenz (GPLv3 / Royalty-Free).<br>• <strong>Eigene Modelle (.glb / .gltf):</strong> Selbst importierte 3D-Modelle unterliegen den jeweiligen Original-Lizenzen ihrer Autoren (z. B. Creative Commons CC0, CC-BY).",
      },
      {
        title: "Rückkehr zur 2D-Zeichnung",
        body: "Mit Taste <code>3</code>, <code>V</code> oder <code>Esc</code> schalten Sie jederzeit nahtlos zurück in die 2D-Planung.",
      },
    ],
    callout:
      "<strong>Schnellwechsel:</strong> Drücken Sie jederzeit <code>3</code> oder <code>V</code>, um zwischen der 2D-CAD-Zeichenebene und der realistischen 3D-Visualisierung umzuschalten.",
  },
  server: {
    title: "Multi-User Zusammenarbeit & Server-Autostart",
    intro:
      "Mit Railbench können mehrere Personen gleichzeitig und in Echtzeit über das lokale Netzwerk (LAN) oder das Internet an einem gemeinsamen Gleisplan bauen.",
    cards: [
      {
        title: "1. Echtzeit-Synchronisation",
        body: "Gleise platzieren, verschieben, drehen, Rampensteigungen anpassen und Grundplatten verändern – alle Aktionen werden ohne Verzögerung an alle verbundenen Planer übertragen.",
      },
      {
        title: "2. Live-Cursor & Benutzerfarben",
        body: "Jeder Teilnehmer besitzt eine eigene Farbe und einen Benutzernamen. Mausbewegungen und Auswahlen sind live auf dem Plan sichtbar.",
      },
      {
        title: "3. Passwort-Schutz & Räume",
        body: "Räume können mit einem Passwort geschützt werden. Unbefugte können dem Raum ohne Kennwort nicht beitreten.",
      },
      {
        title: "4. Automatisches Speichern",
        body: "Der Server sichert den Baufortschritt automatisch im Ordner <code>./saved_plans/&lt;raum&gt;.gleisplan</code> ab.",
      },
    ],
    cliTitle: "Startvariablen für Server-Autostart (Kommandozeile & Umgebung)",
    cliIntro:
      "Sie können den Server beim Start der Anwendung automatisch im Hintergrund aktivieren, ohne ihn manuell in den Menüs einschalten zu müssen.",
    cliHeaders: ["Startvariable / Flag", "Standard", "Bedeutung / Funktion"],
    cliRows: [
      [
        "<code>--server</code> oder <code>--collab-server</code>",
        "aus",
        "Startet den integrierten WebSocket-Server automatisch beim App-Start.",
      ],
      [
        "<code>--port &lt;Port&gt;</code> oder <code>--server-port</code>",
        "3000",
        "Legt den Netzwerk-Port fest (z. B. <code>--port 8080</code>).",
      ],
      [
        "<code>--password &lt;PW&gt;</code>",
        "keines",
        "Richtet ein Passwort für den Standardraum ein.",
      ],
      [
        "<code>--room &lt;Name&gt;</code>",
        "default",
        "Bestimmt den Namen des aktiven Raums (z. B. <code>--room hauptbahnhof</code>).",
      ],
      [
        "<code>--connect</code> oder <code>--client</code>",
        "aus",
        "Verbindet das lokale Fenster direkt nach dem Start mit dem eigenen Server.",
      ],
    ],
    envTitle: "Umgebungsvariablen (Environment Variables):",
    envHeaders: ["Variable", "Beispiel", "Wirkung"],
    envRows: [
      [
        "<code>RAILBENCH_SERVER</code>",
        "<code>1</code> oder <code>true</code>",
        "Aktiviert den Server-Autostart beim Starten.",
      ],
      [
        "<code>RAILBENCH_SERVER_PORT</code>",
        "<code>3000</code>",
        "Server-Port konfigurieren (alternativ <code>COLLAB_PORT</code> / <code>PORT</code>).",
      ],
      [
        "<code>RAILBENCH_SERVER_PASSWORD</code>",
        "<code>geheim123</code>",
        "Passwort für den Raumschutz vorgeben.",
      ],
      [
        "<code>RAILBENCH_SERVER_ROOM</code>",
        "<code>bergstrecke</code>",
        "Standard-Raumnamen festlegen.",
      ],
      [
        "<code>RAILBENCH_CONNECT</code>",
        "<code>1</code>",
        "Automatische Client-Verbindung beim Start ausführen.",
      ],
    ],
    callout:
      "<strong>Start-Beispiele:</strong><br>• Desktop-App mit Server & Passwort: <code>railbench --server --port 3000 --password geheim --connect</code><br>• Standalone Node.js Server: <code>node server.mjs --port 8080 --password geheim --room anlage1</code><br>• Web-Direktlink für Mitbauer: <code>http://&lt;IP-Adresse&gt;:3000/?collab=1&room=default&pass=geheim</code>",
  },
  shortcuts: {
    title: "⌨️ Tastaturkürzel-Referenz",
    intro: "Nutzen Sie diese Kurzbefehle für schnelles und flüssiges Arbeiten:",
    headers: ["Kürzel", "Aktion", "Kategorie"],
    rows: [
      ["<kbd>Strg</kbd> + <kbd>N</kbd>", "Neuer Gleisplan", "Datei"],
      ["<kbd>Strg</kbd> + <kbd>O</kbd>", "Plan öffnen (.railbench / .gleisplan / XML)", "Datei"],
      ["<kbd>Strg</kbd> + <kbd>S</kbd>", "Plan speichern", "Datei"],
      ["<kbd>Strg</kbd> + <kbd>Umschalt</kbd> + <kbd>S</kbd>", "Plan speichern unter...", "Datei"],
      ["<kbd>Strg</kbd> + <kbd>W</kbd>", "<strong>Plan / Dialog / Fenster schließen</strong>", "Datei / Navigation"],
      ["<kbd>Strg</kbd> + <kbd>P</kbd>", "<strong>Gleisplan drucken (Ebenengetrennt)</strong>", "Druck / Export"],
      ["<kbd>Strg</kbd> + <kbd>E</kbd>", "Stückliste exportieren (CSV)", "Druck / Export"],
      ["<kbd>Strg</kbd> + <kbd>Z</kbd>", "Rückgängig", "Bearbeiten"],
      ["<kbd>Strg</kbd> + <kbd>Y</kbd>", "Wiederholen", "Bearbeiten"],
      ["<kbd>Strg</kbd> + <kbd>C</kbd>", "Markierte Gleise kopieren", "Bearbeiten"],
      ["<kbd>Strg</kbd> + <kbd>V</kbd>", "Gleise aus Zwischenablage einfügen", "Bearbeiten"],
      ["<kbd>Strg</kbd> + <kbd>D</kbd>", "<strong>Markierte Gleise duplizieren</strong>", "Bearbeiten"],
      ["<kbd>Strg</kbd> + <kbd>A</kbd>", "Alle Gleise auswählen", "Auswahl"],
      ["<kbd>R</kbd>", "Markierte Gleise +7,5° im Uhrzeigersinn drehen", "Bearbeiten"],
      ["<kbd>Umschalt</kbd> + <kbd>R</kbd>", "Markierte Gleise -7,5° gegen den Uhrzeigersinn drehen", "Bearbeiten"],
      ["<kbd>Umschalt</kbd> + <kbd>Mausrad</kbd>", "Gleis unter dem Mauszeiger stufenlos drehen", "Bearbeiten"],
      ["<kbd>Linksklick</kbd> + Ziehen (freier Bereich)", "<strong>Aufziehrahmen / Mehrfachauswahl (Marquee)</strong>", "Auswahl"],
      ["<kbd>Rechtsklick</kbd> / <kbd>Mittelklick</kbd> + Ziehen", "Arbeitsfläche verschieben (Pan)", "Navigation"],
      ["<kbd>Strg</kbd> / <kbd>Umschalt</kbd> + <kbd>Linksklick</kbd>", "Gleis zur Auswahl hinzufügen / entfernen (Mehrfachauswahl)", "Auswahl"],
      ["<kbd>Alt</kbd> + <kbd>L</kbd>", "<strong>Auswahlsperre aufheben (Gesperrte & gelöschte Ebenen auswählen)</strong>", "Arbeitsbereich / Ebenen"],
      ["<kbd>M</kbd>", "<strong>Gleishöhe & Ebene anpassen (Dialog)</strong>", "Ebenen"],
      ["<kbd>Umschalt</kbd> + <kbd>M</kbd>", "<strong>Auto-Rampe berechnen</strong> (Konstante Steigung zwischen Ebenen/Höhen)", "Ebenen / Rampen"],
      ["<kbd>Entf</kbd> / <kbd>Rückschritt</kbd>", "Markierte Gleise löschen", "Bearbeiten"],
      ["<kbd>F</kbd>", "Stromeinspeisepunkt setzen / umschalten", "Elektrik"],
      ["<kbd>Umschalt</kbd> + <kbd>F</kbd>", "Oberleitungs-Einspeisung setzen / umschalten", "Elektrik / Oberleitung"],
      ["<kbd>I</kbd>", "Trennstelle / Isolierverbinder setzen", "Elektrik"],
      ["<kbd>Umschalt</kbd> + <kbd>I</kbd>", "Oberleitungs-Trennstelle / Streckentrenner setzen", "Elektrik / Oberleitung"],
      ["<kbd>O</kbd>", "Oberleitung (Fahrdraht & Masten) umschalten", "Elektrik / 3D"],
      ["<kbd>Umschalt</kbd> + <kbd>D</kbd>", "Digitalen Weichendecoder einbauen / entfernen", "Elektrik / Weichen"],
      ["<kbd>Umschalt</kbd> + <kbd>L</kbd>", "Beleuchtete Weichenlaterne montieren / ausschalten", "Elektrik / Weichen"],
      ["<kbd>P</kbd>", "Spannungsabfall-Heatmap ein/aus", "Elektrik"],
      ["<kbd>3</kbd> oder <kbd>V</kbd>", "<strong>3D-Perspektive & Ansicht umschalten</strong>", "Ansicht / 3D"],
      ["<kbd>Esc</kbd>", "3D-Modus verlassen / Dialoge schließen", "Ansicht / Navigation"],
      ["<kbd>S</kbd>", "Automatisches Einrasten umschalten", "Ansicht / Fangmodus"],
      ["<kbd>C</kbd>", "Gleistyp-Farben umschalten", "Ansicht"],
      ["<kbd>N</kbd>", "Artikelnummern an Gleisen ein/aus", "Ansicht"],
      ["<kbd>Umschalt</kbd> + <kbd>B</kbd>", "Ebenen-Manager ein/ausblenden", "Arbeitsbereich"],
      ["<kbd>Umschalt</kbd> + <kbd>L</kbd>", "Millimeter-Lineale ein/aus", "Ansicht"],
      ["<kbd>Strg</kbd> + <kbd>0</kbd>", "Ansicht zentrieren", "Ansicht"],
      ["<kbd>F1</kbd> oder <kbd>?</kbd>", "Dieses Benutzerhandbuch öffnen", "Hilfe"],
    ],
  },
  catalog: {
    title: "Gleiskatalog-Format & JSON-Beispiel (Spur H0)",
    intro:
      "Railbench unterstützt das Importieren eigener Gleisbibliotheken über das Menü <strong>Datei ➔ Gleiskatalog importieren (JSON)...</strong>. Gleiskataloge werden im schlanken, universellen JSON-Format gespeichert.",
    cards: [
      {
        title: "Katalog-Kopfdaten",
        body: '• <code>scale</code>: Nenngröße (z. B. <code>"H0"</code>, <code>"TT"</code>, <code>"N"</code>, <code>"Z"</code>, <code>"1"</code>, <code>"G"</code>, <code>"0"</code>)<br>• <code>gauge</code>: Modell-Spurweite in mm (H0: <code>16.5</code>, TT: <code>12.0</code>, N: <code>9.0</code>, Z: <code>6.5</code>, 1/G: <code>45.0</code>, 0: <code>32.0</code>)<br>• <code>hasRoadbed</code>: <code>true</code> für Schotterbettung, <code>false</code> für Schwellengleis<br>• <code>conductors</code>: <code>2</code> (Zweileiter) oder <code>3</code> (Dreileiter / Mittelleiter)',
      },
      {
        title: "Unterstützte Gleistypen (17 Typen)",
        body: "Das untenstehende Muster enthält <strong>exakt ein Exemplar jedes Gleistyps</strong>:<br><code>straight</code>, <code>curve</code>, <code>turnout-left</code>, <code>turnout-right</code>, <code>curved-turnout-left</code>, <code>curved-turnout-right</code>, <code>three-way</code>, <code>double-slip</code>, <code>crossing</code>, <code>uncoupler</code>, <code>buffer-stop</code>, <code>feeder</code>, <code>isolating</code>, <code>flex</code>, <code>turntable</code>, <code>transfer-table</code>, <code>accessory</code>.",
      },
    ],
    copyBtn: "📋 JSON kopieren",
    downloadBtn: "💾 JSON herunterladen (railbench-catalog-h0-example.json)",
    metaInfo: "Spur H0 · 16,5 mm · 17 Gleistypen",
  },
  about: {
    title: "Über Railbench & Gleissysteme",
    intro:
      "<strong>Railbench {version}</strong> ist eine Open-Source-Modellbahn-CAD-Anwendung für die Planung und elektrische Netzanalyse von Modellbahnanlagen. Sie basiert auf moderner Web-Technologie (TypeScript, PixiJS WebGL-Rendering, Vite und Electron).",
    systemsHeading: "Unterstützte Nenngrößen & Gleisbibliotheken:",
    systems: [
      "<strong>H0 (1:87)</strong>: Märklin C-Gleis (Dreileiter-Bettungsgleis mit allen Radien R1–R5, Weichen, schlanken Weichen und Kreuzungen)",
      "<strong>H0 (1:87)</strong>: Märklin K-Gleis (Vorbildgetreues Dreileiter-Schwellengleis ohne Bettung, R1–R5, schlanke Weichen 14° 26' und Flexgleis)",
      "<strong>H0 (1:87)</strong>: Märklin M-Gleis (Klassisches Metallgleis mit Blechbettung und Punktkontakten, R1, R2, Industriekreis und DKW)",
      "<strong>H0 (1:87)</strong>: Trix C-Gleis (Zweileiter-Gleichstrom mit C-Gleis-Bettungsgeometrie)",
      "<strong>H0 (1:87)</strong>: Piko A-Gleis (Zweileiter-Gleichstrom Schwellengleis)",
      "<strong>H0 (1:87)</strong>: Roco Line (Zweileiter-Gleichstrom Gleis mit Bettung)",
      "<strong>H0 (1:87)</strong>: Peco Streamline Code 100 (Universelles Zweileiter-Schwellengleissystem)",
      "<strong>H0 (1:87)</strong>: Tillig Elite (Vorbildgetreues Zweileiter-Gleissystem mit feinem Schienenprofil)",
      "<strong>TT (1:120)</strong>: Tillig TT Modellgleis",
      "<strong>TT (1:120)</strong>: Tillig TT Bettungsgleis (Gleissystem mit integrierter Schotterbettung)",
      "<strong>TT (1:120)</strong>: Kuehn TT Gleis (Zweileiter-Gleissystem für Spur TT)",
      "<strong>N (1:160)</strong>: Fleischmann Piccolo (Spur N Bettungsgleis)",
      "<strong>N (1:160)</strong>: Kato Unitrack (N-Gleis mit integriertem Schotterbett und Unijoiner-Steckverbindung)",
      "<strong>N (1:160)</strong>: Minitrix (Klassisches Zweileiter-Schwellengleis für Spur N)",
      "<strong>Z (1:220)</strong>: Märklin mini-club (Klassisches Spur-Z-Zweileitergleis)",
      "<strong>Z (1:220)</strong>: Rokuhan (Spur-Z-Gleissystem mit integriertem Schotterbett)",
    ],
    callout:
      "<strong>Plattform-Unterstützung:</strong><br>• Getestet & Verifiziert auf: <strong>Windows x86_64</strong> und <strong>Linux (x86_64 AppImage / Flatpak)</strong>.<br>• Verfügbar für: <strong>macOS</strong> (Apple Silicon arm64 & Intel x64), <strong>Windows ARM64</strong> und <strong>Linux ARM64</strong>.",
    licenseNote:
      "Lizenziert unter der GNU General Public License v3 (GPLv3). Alle integrierten prozeduralen 3D-Modelle sind quelloffen und frei nutzbar. Alle Gleisproduktnamen, Artikelnummern und Herstellermarken (Märklin, Trix, Piko, Roco, Fleischmann, Tillig) sind Warenzeichen der jeweiligen Eigentümer.",
  },
};
