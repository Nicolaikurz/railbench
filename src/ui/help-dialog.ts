import { getLanguage } from "@/i18n";

export type HelpTab =
  | "overview"
  | "layers"
  | "print"
  | "power"
  | "baseboard"
  | "view3d"
  | "server"
  | "shortcuts"
  | "about";

export function openHelpDialog(initialTab: HelpTab = "overview"): void {
  const existing = document.getElementById("help-modal");
  if (existing) existing.remove();

  const isDe = getLanguage() === "de";

  const modal = document.createElement("div");
  modal.id = "help-modal";
  modal.className = "rb-modal-overlay";

  const modalTitle = isDe
    ? "Railbench – Benutzerhandbuch & Funktionsübersicht"
    : "Railbench – User & Feature Guide";
  const closeTitle = isDe ? "Hilfe schließen (Esc)" : "Close Help (Esc)";
  const closeBtnText = isDe ? "Handbuch schließen" : "Close Guide";
  const footerTip = isDe
    ? "Tipp: Drücken Sie jederzeit <strong>F1</strong>, um diese Hilfe zu öffnen, oder <strong>M</strong> bei ausgewählten Gleisen für Höhen- und Rampenanpassung."
    : "Tip: Press <strong>F1</strong> anywhere to open this guide, or <strong>M</strong> when tracks are selected to change layer elevation.";

  const tabs: Array<{ id: HelpTab; label: string }> = isDe
    ? [
        { id: "overview", label: "Erste Schritte" },
        { id: "layers", label: "Ebenen & Höhen" },
        { id: "print", label: "Ebenen-Druck" },
        { id: "power", label: "Spannungs-Simulation" },
        { id: "baseboard", label: "Tisch & Platte" },
        { id: "view3d", label: "3D-Ansicht (Alpha)" },
        { id: "server", label: "Multi-User & Server" },
        { id: "shortcuts", label: "Tastaturkürzel" },
        { id: "about", label: "Über Railbench & Gleise" },
      ]
    : [
        { id: "overview", label: "Getting Started" },
        { id: "layers", label: "Layers & Heights" },
        { id: "print", label: "Layer Printing" },
        { id: "power", label: "Voltage Simulation" },
        { id: "baseboard", label: "Table & Baseboard" },
        { id: "view3d", label: "3D View (Alpha)" },
        { id: "server", label: "Multi-User & Server" },
        { id: "shortcuts", label: "Keyboard Shortcuts" },
        { id: "about", label: "About & Systems" },
      ];

  modal.innerHTML = `
    <div class="rb-modal-card help-modal-card">
      <div class="rb-modal-header">
        <div class="rb-modal-title">
          <span>${modalTitle}</span>
          <span class="brand-badge">v0.4.0</span>
        </div>
        <button class="rb-modal-close-btn" id="help-modal-close" title="${closeTitle}">✕</button>
      </div>

      <div class="help-modal-layout">
        <!-- Linke Navigationsleiste -->
        <div class="help-nav-sidebar">
          ${tabs
            .map(
              (tb) => `
            <button class="help-nav-btn ${initialTab === tb.id ? "active" : ""}" data-tab="${tb.id}">
              <span>${tb.label}</span>
            </button>
          `
            )
            .join("")}
        </div>

        <!-- Rechter Inhaltsbereich -->
        <div class="help-content-area" id="help-content-area"></div>
      </div>

      <div class="rb-modal-footer">
        <span class="help-footer-tip">${footerTip}</span>
        <button class="rb-btn rb-btn-primary" id="help-btn-close">${closeBtnText}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#help-modal-close") as HTMLButtonElement;
  const bottomCloseBtn = modal.querySelector("#help-btn-close") as HTMLButtonElement;
  const contentArea = modal.querySelector("#help-content-area") as HTMLElement;
  const navButtons = modal.querySelectorAll<HTMLButtonElement>(".help-nav-btn");

  const closeModal = () => modal.remove();
  closeBtn.addEventListener("click", closeModal);
  bottomCloseBtn.addEventListener("click", closeModal);

  // ESC schließt Dialog
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
      window.removeEventListener("keydown", handleKeyDown);
    }
  };
  window.addEventListener("keydown", handleKeyDown);

  const renderTabContent = (tab: HelpTab) => {
    if (isDe) {
      renderGermanTab(tab, contentArea);
    } else {
      renderEnglishTab(tab, contentArea);
    }
  };

  // Navigations-Tabs schalten
  navButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      navButtons.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const tab = btn.getAttribute("data-tab") as HelpTab;
      renderTabContent(tab);
    });
  });

  // Initialen Tab rendern
  renderTabContent(initialTab);
}

function renderGermanTab(tab: HelpTab, contentArea: HTMLElement) {
  switch (tab) {
    case "overview":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Erste Schritte mit Railbench</h2>
          <p>
            <strong>Railbench</strong> ist eine moderne, hochpräzise CAD-Gleisplanungssoftware für den Modellbahn-Bereich.
            Sie verbindet millimetergenaue CAD-Geometrien mit elektrischer Traktionssimulation, flexibler Mehrebenen-Verwaltung und druckfertigen Bauplänen.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Gleise hinzufügen</h3>
              <p>
                Wählen Sie Gleise aus dem <strong>linken Katalog</strong> (Taste <code>B</code> zum Ein-/Ausblenden).
                Klicken Sie auf ein Gleis oder ziehen Sie es per <strong>Drag & Drop</strong> direkt auf die Arbeitsfläche.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Intelligentes Einrasten (Snapping)</h3>
              <p>
                Das automatische magnetische Einrasten (Taste <code>S</code>) verbindet Gleisenden nahtlos, berechnet exakte
                Winkel und schließt Verbindungen im Millimeterbereich.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Drehen & Verschieben</h3>
              <p>
                Gleise mit der Maus ziehen zum Verschieben. Drehen in 7,5°-Schritten mit Taste <code>R</code> (bzw. <code>Umschalt+R</code> gegen den Uhrzeigersinn).
                Halten Sie <code>Umschalt + Mausrad</code> für feines Drehen oder nutzen Sie das direkte Winkelfeld in der unteren Aktionsleiste!
              </p>
            </div>

            <div class="help-card">
              <h3>4. Mehrfachauswahl (Aufziehrahmen)</h3>
              <p>
                Klicken Sie direkt auf ein Gleis, um es auszuwählen. Klicken Sie mit der <strong>linken Maustaste in einen freien Bereich</strong>, halten Sie gedrückt und ziehen Sie ein Rechteck auf (Marquee Box).
                Alle darin liegenden Gleise werden markiert. Halten Sie <code>Strg</code> oder <code>Umschalt</code> gedrückt, um bestehende Auswahlen zu erweitern oder Gleise einzeln hinzuzufügen/abzuwählen.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Navigationstipp:</strong>
            Halten Sie die <strong>rechte Maustaste</strong> oder das <strong>Mausrad (Mittelklick)</strong> gedrückt, um die Arbeitsfläche frei zu verschieben (Pan). Drehen Sie das <strong>Mausrad</strong> zum stufenlosen Zoomen. Drücken Sie jederzeit <code>Strg+0</code>, um den Plan zu zentrieren.
          </div>
        </div>
      `;
      break;

    case "layers":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Mehrebenen-Verwaltung & Höhenplanung</h2>
          <p>
            Mit Railbench können Sie mehrstöckige Modellbahnanlagen (z. B. Grundebene, Bergetappen, Rampen und verdeckte Schattenbahnhöfe) mit echten vertikalen Millimeterhöhen konstruieren.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Schwebendes Ebenen-Menü</h3>
              <p>
                Drücken Sie <code>Umschalt+B</code> oder klicken Sie auf das <strong>Ebenen</strong>-Symbol in der Leiste.
                Die Ebenen sind nach ihrer Höhe sortiert (höchste Ebene oben).
              </p>
            </div>

            <div class="help-card">
              <h3>Sichtbarkeit & Sperrschutz</h3>
              <p>
                Klicken Sie auf das <strong>Auge-Symbol</strong>, um eine Ebene auszublenden, wenn Sie darunterliegende Trassen bearbeiten.
                Klicken Sie auf das <strong>Schloss-Symbol</strong>, um eine Ebene gegen versehentliches Verschieben zu sperren.
              </p>
            </div>

            <div class="help-card">
              <h3>Ebenen direkt umbenennen</h3>
              <p>
                Klicken Sie auf das Stift-Symbol oder doppelklicken Sie auf den Namen, um eine Ebene direkt umzubenennen. Klicken Sie auf das Höhen-Badge (z. B. <code>+80 mm</code>), um die Basishöhe anzupassen.
              </p>
            </div>

            <div class="help-card">
              <h3>Gleise Ebene zuweisen (M)</h3>
              <p>
                Gleise markieren und Taste <code>M</code> drücken oder auf <strong>"↳ Verschieben & Höhe anpassen"</strong> klicken, um Gleise einer Ebene zuzuordnen und ihre Basishöhe automatisch anzugleichen.
              </p>
            </div>

            <div class="help-card">
              <h3>Rampen- & Steigungsrechner</h3>
              <p>
                Erstellen Sie gleichmäßige Steigungen (z. B. 0 mm → 80 mm). Der Rechner teilt die Höhendifferenz durch die Gleislänge und zeigt die Steigung in Prozent an:
                <br>• <strong>&le; 3,0%</strong>: Empfohlen (Sanfte Steigung)
                <br>• <strong>3,0% – 4,5%</strong>: Mäßig (Bei schweren/langen Zügen beachten)
                <br>• <strong>&gt; 4,5%</strong>: Warnung (Schleudergefahr)
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Schwebende Auswahlleiste:</strong> Sobald Gleise markiert sind, erscheint am unteren Bildschirmrand eine Leiste, um Ebene, Höhe und Winkel mit einem Klick zu steuern!
          </div>
        </div>
      `;
      break;

    case "print":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Ebenengetrennter Plan-Druck & PDF-Export (Strg+P)</h2>
          <p>
            Herkömmliche Programme drucken alle Ebenen unübersichtlich übereinander. Railbench bietet echten ebenengetrennten Ausdruck für den schrittweisen Anlagenbau.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Eine Seite je Ebene</h3>
              <p>
                Jede Ebene wird sauber auf einem eigenen Blatt gedruckt – ideal als 1:1 oder maßstäbliche Bau- und Trassenunterlage.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Exakt identischer Ausschnitt & Maßstab</h3>
              <p>
                Alle Ebenenseiten teilen sich exakt denselben Ursprung, Maßstab und Begrenzungsrahmen. So können transparente Folien oder Ausdrucke deckungsgleich übereinandergelegt werden!
              </p>
            </div>

            <div class="help-card">
              <h3>3. Abgeblendete Referenzkonturen</h3>
              <p>
                Auf Wunsch werden über- oder unterliegende Ebenen zart angedeutet, um Kreuzungen und Pfeilerstandorte sofort im Blick zu haben.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Integrierte Ebenen-Stückliste</h3>
              <p>
                Jede Seite enthält ein Teileverzeichnis mit den für diesen Bauabschnitt benötigten Gleisnummern und Stückzahlen (z. B. <code>24188: 8×, 24172: 4×</code>).
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>So drucken Sie:</strong> Drücken Sie <code>Strg+P</code>, wählen Sie die Ebenen in der Vorschau aus und klicken Sie auf <strong>"System-Druck..."</strong> oder <strong>"Direkt als PDF exportieren"</strong>.
          </div>
        </div>
      `;
      break;

    case "power":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Spannungsabfall & Traktionsstrom-Simulation</h2>
          <p>
            Gewährleisten Sie einen zuverlässigen Betrieb Ihrer Lokomotiven durch die integrierte elektrische Netzanalyse von Railbench.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Farb-Heatmap (Taste P)</h3>
              <p>
                Taste <code>P</code> blendet die Spannungs-Heatmap ein:
                <br>• <strong style="color: #10b981;">Grün</strong>: Optimale Spannung (&gt; 92% der Einspeisung)
                <br>• <strong style="color: #f59e0b;">Gelb/Orange</strong>: Mäßiger Abfall (&gt; 80% der Einspeisung)
                <br>• <strong style="color: #ef4444;">Rot</strong>: Kritischer Abfall (&lt; 80% — zusätzliche Einspeisung erforderlich!)
                <br>• <strong style="color: #dc2626;">Gestrichelt Rot</strong>: Stromlos / vom Netz isolierter Abschnitt
              </p>
            </div>

            <div class="help-card">
              <h3>Einspeisepunkte (Taste F)</h3>
              <p>
                Gleis markieren und <code>F</code> drücken, um einen Stromeinspeisepunkt (Feeder) zu setzen. Railbench berechnet die Spannungsverteilung sofort neu.
              </p>
            </div>

            <div class="help-card">
              <h3>Trennstellen & Isolierverbinder (Taste I)</h3>
              <p>
                Gleis markieren und <code>I</code> drücken, um das Gleis isoliert vom Netz zu trennen. Ideal zur Planung separater Boosterkreise, Blockstrecken, Signalabschnitte oder abschaltbarer Schattenbahnhofsgleise.
              </p>
            </div>

            <div class="help-card">
              <h3>Spannung & Fahrstrom konfigurieren</h3>
              <p>
                Klicken Sie auf das Menü am Strom-Schalter in der Toolbar und wählen Sie <strong>Spannung & Strom anpassen...</strong>:
                <br>• <strong>16V AC / Digital</strong>: Märklin H0 Standard
                <br>• <strong>14V DC / DCC</strong>: Zweileiter H0 (Piko, Roco, Trix)
                <br>• <strong>12V DC</strong>: Spur N & TT
                <br>• <strong>18V DC</strong>: Spur 1 & Gartenbahn
                <br>• <strong>Lokomotiv-Verbrauch (Presets)</strong>:
                <br>&nbsp;&nbsp;• <strong>N / Z</strong>: 50–150 mA (typ. 100 mA)
                <br>&nbsp;&nbsp;• <strong>TT</strong>: 200–400 mA (typ. 300 mA)
                <br>&nbsp;&nbsp;• <strong>H0</strong>: 300–600 mA (typ. 450 mA)
                <br>&nbsp;&nbsp;• <strong>Maximal pro Lok</strong>: 1 A (1000 mA)
                <br>• <strong>Mehrzugbetrieb</strong>: Gesamtfahrstrom wird mit der Anzahl aktiver Lokomotiven multipliziert.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "baseboard":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Grundplatte & Tischkonstruktion</h2>
          <p>
            Planen Sie Ihre Modellbahn passgenau für den Raum und Ihre handwerklichen Rahmenbedingungen.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Modulare Tischformen</h3>
              <p>
                Wählen Sie zwischen vier gängigen Modellbahn-Tischformen:
                <br>• <strong>Rechteckig</strong>: Klassischer Anlagentisch
                <br>• <strong>L-Form</strong>: Eckanlage mit linkem oder rechtem Schenkel
                <br>• <strong>U-Form</strong>: Anlage mit Bedienertrog
                <br>• <strong>T-Form</strong>: Zungenanlage oder Nebenbahnausläufer
              </p>
            </div>

            <div class="help-card">
              <h3>Holzarten & Oberflächen</h3>
              <p>
                Wählen Sie realistische Texturen wie <strong>Birke hell</strong>, <strong>Nussbaum dunkel</strong>,
                <strong>Multiplex</strong>, <strong>Schiefer</strong> oder neutrales <strong>CAD-Weiß</strong>.
              </p>
            </div>

            <div class="help-card">
              <h3>Maßketten & Lineale</h3>
              <p>
                Zeigt Millimeter-Bemaßungen direkt an den Kanten an. Taste <code>Umschalt+L</code> schaltet zusätzliche Randlineale ein.
              </p>
            </div>

            <div class="help-card">
              <h3>Platte an Gleise anpassen</h3>
              <p>
                Ein Klick auf <strong>"Platte an Gleise anpassen"</strong> berechnet automatisch die Mindestmaße des Tisches mit Sicherheitsabstand.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "view3d":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>3D-Perspektive & Ansicht <span class="v3d-alpha-tag">ALPHA</span></h2>
          <p>
            Der 3D-Viewer rendert Ihre gesamte Modellbahnanlage in Echtzeit dreidimensional inklusive Grundplatte, Brückenpfeilern, Ebenen und 3D-Ausschmückung.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Kamera-Navigation</h3>
              <p>
                • <strong>Drehen:</strong> Linksklick und über die 3D-Szene ziehen.<br>
                • <strong>Verschieben:</strong> Rechtsklick und ziehen (oder Umschalt + Linksklick).<br>
                • <strong>Zoomen:</strong> Mausrad oder Trackpad-Geste.<br>
                • <strong>Zurücksetzen:</strong> "⌖ Ansicht zurücksetzen" zentriert die Kamera auf die Grundplatte.
              </p>
            </div>

            <div class="help-card">
              <h3>Brücken & Pfeiler</h3>
              <p>
                Gleise in Höhenlagen werden vertikal akkurat dargestellt. Ab 15 mm Höhe werden automatisch Brückenpfeiler bis zur Grundplatte generiert!
              </p>
            </div>

            <div class="help-card">
              <h3>3D-Tisch & Unterbau</h3>
              <p>
                Der Holztisch wird mit Plattenstärke und soliden Stahlfüßen gerendert.
              </p>
            </div>

            <div class="help-card">
              <h3>3D-Modelle & Ausschmückung (.glb / .gltf)</h3>
              <p>
                Platzieren Sie Gebäude (Bahnhof, Stellwerk, Lokschuppen, Wasserturm, Fachwerkhaus, Signale, Prellböcke, Bäume, Lampen) aus dem Katalog oder importieren Sie eigene GLB/GLTF-Dateien.
              </p>
            </div>

            <div class="help-card">
              <h3>3D-Modell-Lizenzen & Herkunft</h3>
              <p>
                • <strong>Integrierte 3D-Objekte:</strong> Alle 10 mitgelieferten 3D-Modelle sind zu <strong>100 % prozeduraler Open-Source-Code</strong> (Three.js Geometrien in <code>src/render/builtin-models-3d.ts</code>). Sie enthalten keine externen Binärmodelle oder proprietären Texturen, sind frei von Urheberrechtsansprüchen Dritter und unterliegen der freien Open-Source-Lizenz (GPLv3 / Royalty-Free).<br>
                • <strong>Eigene Modelle (.glb / .gltf):</strong> Selbst importierte 3D-Modelle unterliegen den jeweiligen Original-Lizenzen ihrer Autoren (z. B. Creative Commons CC0, CC-BY).
              </p>
            </div>

            <div class="help-card">
              <h3>Hinweis zum Alpha-Status</h3>
              <p>
                Der 3D-Modus befindet sich in stetiger Weiterentwicklung. Taste <code>3</code>, <code>V</code> oder <code>Esc</code> schaltet jederzeit zurück in die 2D-Planung.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Schnellwechsel:</strong> Drücken Sie jederzeit <code>3</code> oder <code>V</code>, um zwischen der 2D-CAD-Zeichenebene und der realistischen 3D-Visualisierung umzuschalten.
          </div>
        </div>
      `;
      break;

    case "server":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Multi-User Zusammenarbeit & Server-Autostart</h2>
          <p>
            Mit Railbench können mehrere Personen gleichzeitig und in Echtzeit über das lokale Netzwerk (LAN) oder das Internet an einem gemeinsamen Gleisplan bauen.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Echtzeit-Synchronisation</h3>
              <p>
                Gleise platzieren, verschieben, drehen, Rampensteigungen anpassen und Grundplatten verändern – alle Aktionen werden ohne Verzögerung an alle verbundenen Planer übertragen.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Live-Cursor & Benutzerfarben</h3>
              <p>
                Jeder Teilnehmer besitzt eine eigene Farbe und einen Benutzernamen. Mausbewegungen und Auswahlen sind live auf dem Plan sichtbar.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Passwort-Schutz & Räume</h3>
              <p>
                Räume können mit einem Passwort geschützt werden. Unbefugte können dem Raum ohne Kennwort nicht beitreten.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Automatisches Speichern</h3>
              <p>
                Der Server sichert den Baufortschritt automatisch im Ordner <code>./saved_plans/&lt;raum&gt;.gleisplan</code> ab.
              </p>
            </div>
          </div>

          <div class="help-topic-section" style="margin-top: 20px;">
            <h3>Startvariablen für Server-Autostart (Kommandozeile & Umgebung)</h3>
            <p>
              Sie können den Server beim Start der Anwendung automatisch im Hintergrund aktivieren, ohne ihn manuell in den Menüs einschalten zu müssen.
            </p>

            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Startvariable / Flag</th>
                  <th style="width: 25%;">Standard</th>
                  <th style="width: 40%;">Bedeutung / Funktion</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>--server</code> oder <code>--collab-server</code></td><td>aus</td><td>Startet den integrierten WebSocket-Server automatisch beim App-Start.</td></tr>
                <tr><td><code>--port &lt;Port&gt;</code> oder <code>--server-port</code></td><td>3000</td><td>Legt den Netzwerk-Port fest (z. B. <code>--port 8080</code>).</td></tr>
                <tr><td><code>--password &lt;PW&gt;</code></td><td>keines</td><td>Richtet ein Passwort für den Standardraum ein.</td></tr>
                <tr><td><code>--room &lt;Name&gt;</code></td><td>default</td><td>Bestimmt den Namen des aktiven Raums (z. B. <code>--room hauptbahnhof</code>).</td></tr>
                <tr><td><code>--connect</code> oder <code>--client</code></td><td>aus</td><td>Verbindet das lokale Fenster direkt nach dem Start mit dem eigenen Server.</td></tr>
              </tbody>
            </table>

            <h4 style="margin-top: 14px; margin-bottom: 6px;">Umgebungsvariablen (Environment Variables):</h4>
            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Variable</th>
                  <th style="width: 25%;">Beispiel</th>
                  <th style="width: 40%;">Wirkung</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>RAILBENCH_SERVER</code></td><td><code>1</code> oder <code>true</code></td><td>Aktiviert den Server-Autostart beim Starten.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PORT</code></td><td><code>3000</code></td><td>Server-Port konfigurieren (alternativ <code>COLLAB_PORT</code> / <code>PORT</code>).</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PASSWORD</code></td><td><code>geheim123</code></td><td>Passwort für den Raumschutz vorgeben.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_ROOM</code></td><td><code>bergstrecke</code></td><td>Standard-Raumnamen festlegen.</td></tr>
                <tr><td><code>RAILBENCH_CONNECT</code></td><td><code>1</code></td><td>Automatische Client-Verbindung beim Start ausführen.</td></tr>
              </tbody>
            </table>
          </div>

          <div class="help-callout" style="margin-top: 16px;">
            <strong>Start-Beispiele:</strong><br>
            • Desktop-App mit Server & Passwort: <code>railbench --server --port 3000 --password geheim --connect</code><br>
            • Standalone Node.js Server: <code>node server.mjs --port 8080 --password geheim --room anlage1</code><br>
            • Web-Direktlink für Mitbauer: <code>http://&lt;IP-Adresse&gt;:3000/?collab=1&room=default&pass=geheim</code>
          </div>
        </div>
      `;
      break;

    case "shortcuts":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>⌨️ Tastaturkürzel-Referenz</h2>
          <p>Nutzen Sie diese Kurzbefehle für schnelles und flüssiges Arbeiten:</p>

          <table class="help-shortcuts-table">
            <thead>
              <tr>
                <th style="width: 25%;">Kürzel</th>
                <th style="width: 40%;">Aktion</th>
                <th style="width: 35%;">Kategorie</th>
              </tr>
            </thead>
            <tbody>
              <tr><td><kbd>Strg</kbd> + <kbd>N</kbd></td><td>Neuer Gleisplan</td><td>Datei</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>O</kbd></td><td>Plan öffnen (.gleisplan / XML)</td><td>Datei</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>S</kbd></td><td>Plan speichern</td><td>Datei</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>Umschalt</kbd> + <kbd>S</kbd></td><td>Plan speichern unter...</td><td>Datei</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>W</kbd></td><td><strong>Plan / Dialog / Fenster schließen</strong></td><td>Datei / Navigation</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>P</kbd></td><td><strong>Gleisplan drucken (Ebenengetrennt)</strong></td><td>Druck / Export</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>E</kbd></td><td>Stückliste exportieren (CSV)</td><td>Druck / Export</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>Z</kbd></td><td>Rückgängig</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>Y</kbd></td><td>Wiederholen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>C</kbd></td><td>Markierte Gleise kopieren</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>V</kbd></td><td>Gleise aus Zwischenablage einfügen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>D</kbd></td><td><strong>Markierte Gleise duplizieren</strong></td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>A</kbd></td><td>Alle Gleise auswählen</td><td>Auswahl</td></tr>
              <tr><td><kbd>R</kbd></td><td>Markierte Gleise +7,5° im Uhrzeigersinn drehen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>R</kbd></td><td>Markierte Gleise -7,5° gegen den Uhrzeigersinn drehen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>Mausrad</kbd></td><td>Gleis unter dem Mauszeiger stufenlos drehen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>Linksklick</kbd> + Ziehen (freier Bereich)</td><td><strong>Aufziehrahmen / Mehrfachauswahl (Marquee)</strong></td><td>Auswahl</td></tr>
              <tr><td><kbd>Rechtsklick</kbd> / <kbd>Mittelklick</kbd> + Ziehen</td><td>Arbeitsfläche verschieben (Pan)</td><td>Navigation</td></tr>
              <tr><td><kbd>Strg</kbd> / <kbd>Umschalt</kbd> + <kbd>Linksklick</kbd></td><td>Gleis zur Auswahl hinzufügen / entfernen (Mehrfachauswahl)</td><td>Auswahl</td></tr>
              <tr><td><kbd>M</kbd></td><td><strong>Gleishöhe & Ebene anpassen</strong></td><td>Ebenen</td></tr>
              <tr><td><kbd>Entf</kbd> / <kbd>Rückschritt</kbd></td><td>Markierte Gleise löschen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>F</kbd></td><td>Stromeinspeisepunkt setzen / umschalten</td><td>Elektrik</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>F</kbd></td><td>Oberleitungs-Einspeisung setzen / umschalten</td><td>Elektrik / Oberleitung</td></tr>
              <tr><td><kbd>I</kbd></td><td>Trennstelle / Isolierverbinder setzen</td><td>Elektrik</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>I</kbd></td><td>Oberleitungs-Trennstelle / Streckentrenner setzen</td><td>Elektrik / Oberleitung</td></tr>
              <tr><td><kbd>O</kbd></td><td>Oberleitung (Fahrdraht & Masten) umschalten</td><td>Elektrik / 3D</td></tr>
              <tr><td><kbd>W</kbd> / <kbd>Umschalt</kbd> + <kbd>W</kbd></td><td>Weichenlaterne / Weichendecoder umschalten</td><td>Elektrik / Weichen</td></tr>
              <tr><td><kbd>P</kbd></td><td>Spannungsabfall-Heatmap ein/aus</td><td>Elektrik</td></tr>
              <tr><td><kbd>3</kbd> oder <kbd>V</kbd></td><td><strong>3D-Perspektive & Ansicht umschalten (Alpha)</strong></td><td>Ansicht / 3D</td></tr>
              <tr><td><kbd>Esc</kbd></td><td>3D-Modus verlassen / Dialoge schließen</td><td>Ansicht / Navigation</td></tr>
              <tr><td><kbd>S</kbd></td><td>Automatisches Einrasten umschalten</td><td>Ansicht / Fangmodus</td></tr>
              <tr><td><kbd>C</kbd></td><td>Gleistyp-Farben umschalten</td><td>Ansicht</td></tr>
              <tr><td><kbd>N</kbd></td><td>Artikelnummern an Gleisen ein/aus</td><td>Ansicht</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>B</kbd></td><td>Ebenen-Manager ein/ausblenden</td><td>Arbeitsbereich</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>L</kbd></td><td>Millimeter-Lineale ein/aus</td><td>Ansicht</td></tr>
              <tr><td><kbd>Strg</kbd> + <kbd>0</kbd></td><td>Ansicht zentrieren</td><td>Ansicht</td></tr>
              <tr><td><kbd>F1</kbd> oder <kbd>?</kbd></td><td>Dieses Benutzerhandbuch öffnen</td><td>Hilfe</td></tr>
            </tbody>
          </table>
        </div>
      `;
      break;

    case "about":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Über Railbench & Gleissysteme</h2>
          <p>
            <strong>Railbench v0.5.0</strong> ist eine Open-Source-Modellbahn-CAD-Anwendung für die Planung und elektrische Netzanalyse von Modellbahnanlagen.
            Sie basiert auf moderner Web-Technologie (TypeScript, PixiJS WebGL-Rendering, Vite und Electron).
          </p>

          <h3>Unterstützte Nenngrößen & Gleisbibliotheken:</h3>
          <ul class="help-systems-list">
            <li><strong>H0 (1:87)</strong>: Märklin C-Gleis (Dreileiter-Bettungsgleis mit allen Radien R1–R5, Weichen, schlanken Weichen und Kreuzungen)</li>
            <li><strong>H0 (1:87)</strong>: Märklin K-Gleis (Vorbildgetreues Dreileiter-Schwellengleis ohne Bettung, R1–R5, schlanke Weichen 14° 26' und Flexgleis)</li>
            <li><strong>H0 (1:87)</strong>: Märklin M-Gleis (Klassisches Metallgleis mit Blechbettung und Punktkontakten, R1, R2, Industriekreis und DKW)</li>
            <li><strong>H0 (1:87)</strong>: Trix C-Gleis (Zweileiter-Gleichstrom mit C-Gleis-Bettungsgeometrie)</li>
            <li><strong>H0 (1:87)</strong>: Piko A-Gleis (Zweileiter-Gleichstrom Schwellengleis)</li>
            <li><strong>H0 (1:87)</strong>: Roco Line (Zweileiter-Gleichstrom Gleis mit Bettung)</li>
            <li><strong>TT (1:120)</strong>: Tillig TT Modellgleis</li>
            <li><strong>N (1:160)</strong>: Fleischmann Piccolo (Spur N Bettungsgleis)</li>
          </ul>

          <div class="help-callout">
            <strong>Plattform-Unterstützung:</strong>
            <br>• Getestet & Verifiziert auf: <strong>Windows x86_64</strong> und <strong>Linux (x86_64 AppImage / Flatpak)</strong>.
            <br>• Verfügbar für: <strong>macOS</strong> (Apple Silicon arm64 & Intel x64), <strong>Windows ARM64</strong> und <strong>Linux ARM64</strong>.
          </div>

          <p style="font-size: 11px; color: var(--text-muted); margin-top: 20px;">
            Lizenziert unter der GNU General Public License v3 (GPLv3). Alle integrierten prozeduralen 3D-Modelle sind quelloffen und frei nutzbar. Alle Gleisproduktnamen, Artikelnummern und Herstellermarken (Märklin, Trix, Piko, Roco, Fleischmann, Tillig) sind Warenzeichen der jeweiligen Eigentümer.
          </p>
        </div>
      `;
      break;
  }
}

function renderEnglishTab(tab: HelpTab, contentArea: HTMLElement) {
  switch (tab) {
    case "overview":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Getting Started with Railbench</h2>
          <p>
            <strong>Railbench</strong> is a high-precision, web- and desktop-based track planning application
            engineered for model railway hobbyists. It combines millimetric CAD geometry with electrical simulation,
            multi-level layout design, and print-ready schematics.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Adding Tracks</h3>
              <p>
                Browse track pieces in the <strong>left catalog sidebar</strong> (press <code>B</code> to toggle).
                Click any piece to append it, or <strong>drag and drop</strong> it directly onto the canvas.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Smart Snapping</h3>
              <p>
                Automatic magnetic snapping (<code>S</code> key) seamlessly aligns track ends, calculates exact
                angles, and connects track geometry within millimetric tolerances.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Rotating & Moving</h3>
              <p>
                Drag tracks with the mouse to move them. Rotate in 7.5° steps using <code>R</code> (or <code>Shift+R</code> for counter-clockwise).
                Hold <code>Shift + Mouse Wheel</code> to rotate smoothly, or drag the blue rotation handle.
                You can also enter any custom angle directly in the Angle input field in the bottom selection bar!
              </p>
            </div>

            <div class="help-card">
              <h3>4. Multi-Selection (Marquee Box)</h3>
              <p>
                Click directly on any track to select it. Click and drag with the <strong>Left Mouse Button in an empty area</strong> to draw a selection rectangle (marquee box).
                All tracks within the rectangle will be selected. Hold <code>Ctrl</code> or <code>Shift</code> to add to your existing selection or toggle individual tracks.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Pro-Tip for Navigation:</strong>
            Hold <strong>Right Mouse Button</strong> or the <strong>Mouse Wheel (Middle Click)</strong> to pan the canvas. Use the <strong>Mouse Wheel</strong> to zoom in and out. Press <code>Ctrl+0</code> anytime to re-center the layout.
          </div>
        </div>
      `;
      break;

    case "layers":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Multi-Deck Layers & Height Management</h2>
          <p>
            Railbench allows designing multi-level model train layouts (e.g. Ground level, Upper mountain branches,
            ramps, and hidden Staging Yards / Shadow Stations) with real vertical millimeter elevations.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Floating Layers Menu</h3>
              <p>
                Press <code>Shift+B</code> or click the <strong>Ebenen</strong> icon in the toolbar to toggle the floating
                layers manager. Layers are sorted by height with the highest deck at the top.
              </p>
            </div>

            <div class="help-card">
              <h3>Visibility & Lock Protection</h3>
              <p>
                Click the <strong>Eye icon</strong> icon to hide a layer when working on tracks underneath.
                Click the <strong>Lock icon</strong> icon to freeze a layer against accidental movement or editing.
              </p>
            </div>

            <div class="help-card">
              <h3>Rename Layers Inline</h3>
              <p>
                Double-click any layer name to rename it immediately inline. Press <code>Enter</code> to confirm or <code>Esc</code> to cancel. Click the elevation badge (e.g. <code>+80 mm</code>) to adjust its base height.
              </p>
            </div>

            <div class="help-card">
              <h3>Move Tracks to Layer (M)</h3>
              <p>
                Select tracks and press <code>M</code> or click <strong>"↳ Move & Set Height"</strong> on any layer card.
                The tracks will be assigned to that layer and their height automatically synchronized.
              </p>
            </div>

            <div class="help-card">
              <h3>Ramp & Incline Calculator</h3>
              <p>
                Configure gradual elevation transitions (e.g. 0 mm → 80 mm). The built-in calculator divides elevation
                difference by total track length and shows a live gradient percentage:
                <br>• <strong>&le; 3.0%</strong>: Recommended (Gentle incline)
                <br>• <strong>3.0% – 4.5%</strong>: Moderate (Watch heavy trains)
                <br>• <strong>&gt; 4.5%</strong>: Warning (Risk of wheel slippage)
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Canvas Selection Bar:</strong> When one or more tracks are selected, a sleek floating bar appears at the bottom center of the screen, allowing you to change layer and height in a single click!
          </div>
        </div>
      `;
      break;

    case "print":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Layer-Separated Layout Printing (Ctrl+P)</h2>
          <p>
            Traditional track planning software prints the whole layout as one overlapping tangle. Railbench introduces
            <strong>true layer-separated printing</strong> designed for building real layouts benchwork by benchwork.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. One Page Per Layer</h3>
              <p>
                Every layer is printed on its own dedicated sheet. Automatic CSS print pagination guarantees clean page breaks
                without overlapping content.
              </p>
            </div>

            <div class="help-card">
              <h3>2. 100% Identical Scale & Bounding Box</h3>
              <p>
                All printed layer sheets share the exact same bounding box, origin, and scale. You can print transparent or paper
                sheets, stack them on top of each other, and verify that bridge pillars, ramp portals, and track curves align perfectly!
              </p>
            </div>

            <div class="help-card">
              <h3>3. Ghosted Reference Outlines</h3>
              <p>
                Optionally show faint, dimmed outlines of the layers above and below on the current sheet so you have full spatial context.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Layer-Specific Bill of Materials</h3>
              <p>
                Each sheet includes a bottom inventory banner showing exactly which track pieces and quantities are required to construct
                that specific layer (e.g., <code>24188: 8×, 24172: 4×</code>).
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>How to Print:</strong> Press <code>Ctrl+P</code> (or <code>Cmd+P</code> on macOS), choose your layers in the preview dialog, and click <strong>"System Print..."</strong> or <strong>"Direct Export as PDF"</strong>.
          </div>
        </div>
      `;
      break;

    case "power":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Traction Voltage Drop & Analog Simulation</h2>
          <p>
            Ensure reliable operation of digital and analog locomotives with Railbench's integrated
            electrical simulation engine.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Color Heatmap (P Key)</h3>
              <p>
                Press <code>P</code> to toggle the voltage heatmap:
                <br>• <strong style="color: #10b981;">Green</strong>: Optimal voltage (&gt; 92% of supply)
                <br>• <strong style="color: #f59e0b;">Amber</strong>: Moderate drop (&gt; 80% of supply)
                <br>• <strong style="color: #ef4444;">Red</strong>: Critical drop (&lt; 80% — add feed point!)
                <br>• <strong style="color: #dc2626;">Dashed Red</strong>: Isolated / disconnected block
              </p>
            </div>

            <div class="help-card">
              <h3>Power Feeder Points (F Key)</h3>
              <p>
                Select a track and press <code>F</code> to attach a power feed point (e.g. from your booster or transformer).
                Railbench immediately recomputes voltage distribution across the entire rail network.
              </p>
            </div>

            <div class="help-card">
              <h3>Track Isolators (I Key)</h3>
              <p>
                Select a track and press <code>I</code> to place an electrical gap (Trennstelle). Use isolators to simulate separate
                booster districts, analog block control, signal stopping sections, or staging yard siding cutoffs.
              </p>
            </div>

            <div class="help-card">
              <h3>Supply Voltage & Traction Current</h3>
              <p>
                Click the arrow next to the Voltage button in the toolbar and select <strong>Custom Voltage & Current...</strong> to configure voltage and traction currents:
                <br>• <strong>16V AC / Digital</strong>: Märklin H0 Standard
                <br>• <strong>14V DC / DCC</strong>: 2-Rail H0 (Piko, Roco, Trix)
                <br>• <strong>12V DC</strong>: N Scale & TT Scale
                <br>• <strong>18V DC</strong>: Gauge 1 & LGB Garden Railways
                <br>• <strong>Locomotive Current Presets</strong>:
                <br>&nbsp;&nbsp;• <strong>N / Z</strong>: 50–150 mA (typ. 100 mA)
                <br>&nbsp;&nbsp;• <strong>TT</strong>: 200–400 mA (typ. 300 mA)
                <br>&nbsp;&nbsp;• <strong>H0</strong>: 300–600 mA (typ. 450 mA)
                <br>&nbsp;&nbsp;• <strong>Max. per Loco</strong>: 1 A (1000 mA)
                <br>• <strong>Multi-Train Load</strong>: Multiply current by the number of simultaneously operating locomotives.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "baseboard":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Table & Baseboard Benchwork Planning</h2>
          <p>
            Design your layout to fit your actual room and carpentry constraints using realistic baseboard boundaries.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Modular Table Shapes</h3>
              <p>
                Choose between four common model railway benchwork forms:
                <br>• <strong>Rectangular</strong>: Classic flat table or island layout
                <br>• <strong>L-Shape</strong>: Corner layouts with left or right return wing
                <br>• <strong>U-Shape</strong>: Walk-in operator pit layout
                <br>• <strong>T-Shape</strong>: Peninsula or branch line extension
              </p>
            </div>

            <div class="help-card">
              <h3>Wood Finishes & Materials</h3>
              <p>
                Select realistic table textures including <strong>Natural Birch Wood</strong>, <strong>Dark Walnut</strong>,
                <strong>Plywood</strong>, <strong>Dark Slate</strong>, or clean <strong>Drafting White</strong>.
              </p>
            </div>

            <div class="help-card">
              <h3>Live Dimensions & Rulers</h3>
              <p>
                Displays millimetric dimension arrows directly on the board. Toggle millimeter rulers along canvas borders with <code>Shift+L</code>.
              </p>
            </div>

            <div class="help-card">
              <h3>Fit Table to Tracks</h3>
              <p>
                In the Table tab of the left sidebar, click <strong>"Fit Table to Tracks"</strong> to automatically calculate the minimum table dimensions
                needed to enclose all track pieces with a safe buffer.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "view3d":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>3D Perspective & Elevation View <span class="v3d-alpha-tag">ALPHA</span></h2>
          <p>
            The 3D Viewer renders your entire model railroad layout in real-time three dimensions, including the baseboard table, multi-deck elevations, bridge pillars, ballasting, and tracks.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Camera Navigation</h3>
              <p>
                • <strong>Orbit / Rotate:</strong> Left-click and drag across the 3D scene.<br>
                • <strong>Pan:</strong> Right-click and drag (or Shift + Left-click).<br>
                • <strong>Zoom:</strong> Scroll wheel or trackpad pinch.<br>
                • <strong>Reset:</strong> Click "⌖ Reset View" to re-center on the benchwork.
              </p>
            </div>

            <div class="help-card">
              <h3>Bridges & Elevated Decks</h3>
              <p>
                Tracks on elevated layers (or custom ramps) are rendered at their true vertical heights. Tracks elevated &gt; 15 mm automatically generate realistic bridge support pillars connecting to the baseboard!
              </p>
            </div>

            <div class="help-card">
              <h3>Modular 3D Benchwork</h3>
              <p>
                The wooden baseboard table (Rectangular, L-shape, U-shape, or T-profile) is extruded in full 3D with authentic wood grain shading and sturdy steel table legs.
              </p>
            </div>

            <div class="help-card">
              <h3>3D Models & Scenery (.glb / .gltf)</h3>
              <p>
                Place built-in open-source models (Passenger Station, Signal Box, Engine Shed, Water Tower, Half-Timbered House, Light Signal, Buffer Stop, Trees, Platform Lamps) or import custom <code>.glb</code> / <code>.gltf</code> files via the <strong>3D Models</strong> sidebar tab or by dragging files onto the canvas!
              </p>
            </div>

            <div class="help-card">
              <h3>3D Models Licensing & Origin</h3>
              <p>
                • <strong>Built-in 3D Scenery:</strong> All 10 included models are <strong>100% procedurally generated pure code</strong> (Three.js procedural geometry in <code>src/render/builtin-models-3d.ts</code>). They contain no third-party binary assets or proprietary textures and are fully covered by the project's open-source license (GPLv3 / royalty-free for personal, hobby, and commercial layout design).<br>
                • <strong>Imported Models (.glb / .gltf):</strong> Custom models imported from external sources remain governed by their respective authors' licenses (e.g. Creative Commons CC0, CC-BY).
              </p>
            </div>

            <div class="help-card">
              <h3>Alpha Status Notice</h3>
              <p>
                The 3D viewer is currently in <strong>Alpha preview</strong>. Features like 3D track connection editing, detailed scenery, and catenary are actively in development. Press <code>3</code>, <code>V</code>, or <code>Esc</code> anytime to return to 2D planning.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Quick Switch:</strong> Press <code>3</code> or <code>V</code> at any time to toggle between the 2D CAD drafting mode and the 3D perspective visualization.
          </div>
        </div>
      `;
      break;

    case "server":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Multi-User Collaboration & Server Auto-Start</h2>
          <p>
            Railbench allows multiple modelers to build and design layouts simultaneously in real time across a local area network (LAN) or over the Internet.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Real-Time Synchronization</h3>
              <p>
                Track placements, drags, rotations, gradient ramps, and baseboard adjustments propagate instantly without lag to all connected peers.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Live Cursors & Awareness</h3>
              <p>
                Every participant has a unique color and name tag. See remote mouse movements and track selections live on the canvas.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Password Protection & Rooms</h3>
              <p>
                Rooms can be secured with passwords to prevent unauthorized access and protect layouts during collaborative sessions.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Automatic Persistence</h3>
              <p>
                The server automatically persists all changes to disk under <code>./saved_plans/&lt;room&gt;.gleisplan</code>.
              </p>
            </div>
          </div>

          <div class="help-topic-section" style="margin-top: 20px;">
            <h3>Start Variables for Server Auto-Start (CLI & Environment)</h3>
            <p>
              Launch Railbench with the collaboration server auto-started in the background without needing to toggle menus manually.
            </p>

            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Start Variable / Flag</th>
                  <th style="width: 25%;">Default</th>
                  <th style="width: 40%;">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>--server</code> or <code>--collab-server</code></td><td>off</td><td>Automatically starts the embedded WebSocket server on application launch.</td></tr>
                <tr><td><code>--port &lt;Port&gt;</code> or <code>--server-port</code></td><td>3000</td><td>Custom network port (e.g. <code>--port 8080</code>).</td></tr>
                <tr><td><code>--password &lt;PW&gt;</code></td><td>none</td><td>Configures a password requirement for the default room.</td></tr>
                <tr><td><code>--room &lt;Name&gt;</code></td><td>default</td><td>Sets the active collaborative room name (e.g. <code>--room central-station</code>).</td></tr>
                <tr><td><code>--connect</code> or <code>--client</code></td><td>off</td><td>Automatically connects the local window to the server upon launch.</td></tr>
              </tbody>
            </table>

            <h4 style="margin-top: 14px; margin-bottom: 6px;">Environment Variables:</h4>
            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Variable</th>
                  <th style="width: 25%;">Example</th>
                  <th style="width: 40%;">Effect</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>RAILBENCH_SERVER</code></td><td><code>1</code> or <code>true</code></td><td>Auto-starts the server on launch.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PORT</code></td><td><code>3000</code></td><td>Configures the server port (or <code>COLLAB_PORT</code> / <code>PORT</code>).</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PASSWORD</code></td><td><code>secret123</code></td><td>Sets the room password.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_ROOM</code></td><td><code>staging-yard</code></td><td>Configures default room name.</td></tr>
                <tr><td><code>RAILBENCH_CONNECT</code></td><td><code>1</code></td><td>Auto-connects local client on start.</td></tr>
              </tbody>
            </table>
          </div>

          <div class="help-callout" style="margin-top: 16px;">
            <strong>Quickstart Examples:</strong><br>
            • Desktop App with Server & Password: <code>railbench --server --port 3000 --password secret --connect</code><br>
            • Standalone Node.js Server: <code>node server.mjs --port 8080 --password secret --room yard1</code><br>
            • 1-Click Browser Link for Collaborators: <code>http://&lt;IP-Address&gt;:3000/?collab=1&room=default&pass=secret</code>
          </div>
        </div>
      `;
      break;

    case "shortcuts":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>⌨️ Keyboard Shortcuts Reference</h2>
          <p>Master these shortcuts for rapid layout design and editing:</p>

          <table class="help-shortcuts-table">
            <thead>
              <tr>
                <th style="width: 25%;">Shortcut</th>
                <th style="width: 40%;">Action</th>
                <th style="width: 35%;">Category</th>
              </tr>
            </thead>
            <tbody>
              <tr><td><kbd>Ctrl</kbd> + <kbd>N</kbd></td><td>New Layout Plan</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>O</kbd></td><td>Open Plan File (.gleisplan / XML)</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>S</kbd></td><td>Save Plan (Quick Save)</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd></td><td>Save Plan As...</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>W</kbd></td><td><strong>Close Plan / Dialog / Window</strong></td><td>File / Navigation</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>P</kbd></td><td><strong>Print Plan (Layer-Separated Pages)</strong></td><td>Print / Export</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>E</kbd></td><td>Export Bill of Materials (CSV)</td><td>Print / Export</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Z</kbd></td><td>Undo last action</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Y</kbd></td><td>Redo undone action</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>C</kbd></td><td>Copy selected tracks</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>V</kbd></td><td>Paste tracks from clipboard</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>D</kbd></td><td><strong>Duplicate selected tracks</strong></td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>A</kbd></td><td>Select all tracks</td><td>Selection</td></tr>
              <tr><td><kbd>R</kbd></td><td>Rotate selected item +7.5° clockwise</td><td>Editing</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>R</kbd></td><td>Rotate selected item -7.5° counter-clockwise</td><td>Editing</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>Wheel</kbd></td><td>Rotate track under cursor by mouse wheel</td><td>Editing</td></tr>
              <tr><td><kbd>Left Click</kbd> + Drag (empty area)</td><td><strong>Marquee Box Selection (Multiple Tracks)</strong></td><td>Selection</td></tr>
              <tr><td><kbd>Right Click</kbd> / <kbd>Middle Click</kbd> + Drag</td><td>Pan Canvas Viewport</td><td>Navigation</td></tr>
              <tr><td><kbd>Ctrl</kbd> / <kbd>Shift</kbd> + <kbd>Left Click</kbd></td><td>Toggle track in selection (Multi-selection)</td><td>Selection</td></tr>
              <tr><td><kbd>M</kbd></td><td><strong>Change Track Height / Move to Layer</strong></td><td>Layers</td></tr>
              <tr><td><kbd>Del</kbd> / <kbd>Backspace</kbd></td><td>Delete selected tracks</td><td>Editing</td></tr>
              <tr><td><kbd>F</kbd></td><td>Toggle Power Feeder point</td><td>Electrical</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>F</kbd></td><td>Toggle Catenary Power Feeder</td><td>Electrical / Catenary</td></tr>
              <tr><td><kbd>I</kbd></td><td>Toggle Track Isolator / Gap</td><td>Electrical</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>I</kbd></td><td>Toggle Catenary Section Insulator</td><td>Electrical / Catenary</td></tr>
              <tr><td><kbd>O</kbd></td><td>Toggle Catenary (Overhead wire & masts)</td><td>Electrical / 3D</td></tr>
              <tr><td><kbd>W</kbd> / <kbd>Shift</kbd> + <kbd>W</kbd></td><td>Toggle Turnout Lantern / Decoder</td><td>Electrical / Turnouts</td></tr>
              <tr><td><kbd>P</kbd></td><td>Toggle Traction Voltage Heatmap Simulation</td><td>Electrical</td></tr>
              <tr><td><kbd>3</kbd> or <kbd>V</kbd></td><td><strong>Toggle 3D Elevation & Perspective View (Alpha)</strong></td><td>View / 3D</td></tr>
              <tr><td><kbd>Esc</kbd></td><td>Exit 3D View / Close Dialogs</td><td>View / Navigation</td></tr>
              <tr><td><kbd>S</kbd></td><td>Toggle Automatic Track Snapping</td><td>View / Snapping</td></tr>
              <tr><td><kbd>C</kbd></td><td>Toggle Track Geometry Color-Coding</td><td>View</td></tr>
              <tr><td><kbd>N</kbd></td><td>Toggle Track Item Part Numbers</td><td>View</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>B</kbd></td><td>Toggle Floating Layers Manager Menu</td><td>Workspace</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>L</kbd></td><td>Toggle Millimeter Edge Rulers</td><td>View</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>0</kbd></td><td>Reset View / Center on Layout</td><td>View</td></tr>
              <tr><td><kbd>F1</kbd> or <kbd>?</kbd></td><td>Open this Help & Feature Guide</td><td>Help</td></tr>
            </tbody>
          </table>
        </div>
      `;
      break;

    case "about":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>About Railbench & Track Libraries</h2>
          <p>
            <strong>Railbench v0.5.0</strong> is an open-source model railway track design and layout planning CAD tool.
            It is built using modern TypeScript, PixiJS WebGL rendering, Vite, and Electron.
          </p>

          <h3>Supported Scale Systems & Catalogs:</h3>
          <ul class="help-systems-list">
            <li><strong>H0 (1:87)</strong>: Märklin C-Gleis (Full 3-rail with roadbed geometry, curves R1–R5, turnouts, crossings)</li>
            <li><strong>H0 (1:87)</strong>: Märklin K-Gleis (Prototypical 3-rail tie-only track without roadbed, R1–R5, slim turnouts 14° 26' and flex-track)</li>
            <li><strong>H0 (1:87)</strong>: Märklin M-Gleis (Classic sheet-metal roadbed 3-rail track, R1, R2, industrial circle and crossings)</li>
            <li><strong>H0 (1:87)</strong>: Trix C-Gleis (2-rail DC with C-track roadbed geometry, identical footprint)</li>
            <li><strong>H0 (1:87)</strong>: Piko A-Gleis (2-rail standard gauge, flexible geometries, turnouts)</li>
            <li><strong>H0 (1:87)</strong>: Roco Line (2-rail realistic high-fidelity track with roadbed)</li>
            <li><strong>TT (1:120)</strong>: Tillig TT Modellgleis</li>
            <li><strong>N (1:160)</strong>: Fleischmann Piccolo (N-scale with roadbed ballasting)</li>
          </ul>

          <div class="help-callout">
            <strong>Testing & Platform Support:</strong>
            <br>• Tested & Verified on: <strong>Windows x86_64</strong> and <strong>Linux (x86_64 AppImage / Flatpak)</strong>.
            <br>• Built & Available for: <strong>macOS</strong> (Apple Silicon M1-M4 arm64 & Intel x64), <strong>Windows ARM64</strong>, and <strong>Linux ARM64</strong>.
          </div>

          <p style="font-size: 11px; color: var(--text-muted); margin-top: 20px;">
            Licensed under the GNU General Public License v3 (GPLv3). All built-in procedural 3D models are open source and royalty-free. All track product names, numbers, and brands (Märklin, Trix, Piko, Roco, Fleischmann, Tillig) are trademarks of their respective manufacturers.
          </p>
        </div>
      `;
      break;
  }
}
