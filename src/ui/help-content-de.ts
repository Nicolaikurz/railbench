import type { HelpTab } from "./help-dialog";
import { getAppVersionTag } from "@/version";

export function renderGermanTab(tab: HelpTab, contentArea: HTMLElement) {
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
          <h2>️ Ebenengetrennter PDF-Export & Plan-Druck (Strg+P)</h2>
          <p>
            Herkömmliche Programme exportieren oder drucken alle Ebenen unübersichtlich übereinander. Railbench bietet echten ebenengetrennten Export für den schrittweisen Anlagenbau.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Eine Seite je Ebene</h3>
              <p>
                Jede Ebene wird sauber auf einem eigenen Blatt ausgegeben – ideal als maßstäbliche Bau- und Trassenunterlage.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Exakt identischer Ausschnitt & Maßstab</h3>
              <p>
                Alle exportierten Ebenenseiten teilen sich exakt denselben Ursprung, Maßstab und Begrenzungsrahmen. So können Seiten deckungsgleich übereinandergelegt werden!
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
            <strong>So exportieren Sie:</strong> Drücken Sie <code>Strg+P</code>, wählen Sie die Ebenen in der Vorschau aus und klicken Sie auf <strong>"System-Druck..."</strong> oder <strong>"Als PDF exportieren"</strong>.
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
              <tr><td><kbd>Strg</kbd> + <kbd>O</kbd></td><td>Plan öffnen (.railbench / .gleisplan / XML)</td><td>Datei</td></tr>
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
              <tr><td><kbd>Alt</kbd> + <kbd>L</kbd></td><td><strong>Auswahlsperre aufheben (Gesperrte & gelöschte Ebenen auswählen)</strong></td><td>Arbeitsbereich / Ebenen</td></tr>
              <tr><td><kbd>M</kbd></td><td><strong>Gleishöhe & Ebene anpassen</strong></td><td>Ebenen</td></tr>
              <tr><td><kbd>Entf</kbd> / <kbd>Rückschritt</kbd></td><td>Markierte Gleise löschen</td><td>Bearbeiten</td></tr>
              <tr><td><kbd>F</kbd></td><td>Stromeinspeisepunkt setzen / umschalten</td><td>Elektrik</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>F</kbd></td><td>Oberleitungs-Einspeisung setzen / umschalten</td><td>Elektrik / Oberleitung</td></tr>
              <tr><td><kbd>I</kbd></td><td>Trennstelle / Isolierverbinder setzen</td><td>Elektrik</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>I</kbd></td><td>Oberleitungs-Trennstelle / Streckentrenner setzen</td><td>Elektrik / Oberleitung</td></tr>
              <tr><td><kbd>O</kbd></td><td>Oberleitung (Fahrdraht & Masten) umschalten</td><td>Elektrik / 3D</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>D</kbd></td><td>Digitalen Weichendecoder einbauen / entfernen</td><td>Elektrik / Weichen</td></tr>
              <tr><td><kbd>Umschalt</kbd> + <kbd>L</kbd></td><td>Beleuchtete Weichenlaterne montieren / ausschalten</td><td>Elektrik / Weichen</td></tr>
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
            <strong>Railbench ${getAppVersionTag()}</strong> ist eine Open-Source-Modellbahn-CAD-Anwendung für die Planung und elektrische Netzanalyse von Modellbahnanlagen.
            Sie basiert auf moderner Web-Technologie (TypeScript, PixiJS WebGL-Rendering, Vite und Electron).
          </p>

          <h3>Unterstützte Nenngrößen & Gleisbibliotheken:</h3>
          <ul class="help-systems-list">
            <!-- H0 (1:87) -->
            <li><strong>H0 (1:87)</strong>: Märklin C-Gleis (Dreileiter-Bettungsgleis mit allen Radien R1–R5, Weichen, schlanken Weichen und Kreuzungen)</li>
            <li><strong>H0 (1:87)</strong>: Märklin K-Gleis (Vorbildgetreues Dreileiter-Schwellengleis ohne Bettung, R1–R5, schlanke Weichen 14° 26' und Flexgleis)</li>
            <li><strong>H0 (1:87)</strong>: Märklin M-Gleis (Klassisches Metallgleis mit Blechbettung und Punktkontakten, R1, R2, Industriekreis und DKW)</li>
            <li><strong>H0 (1:87)</strong>: Trix C-Gleis (Zweileiter-Gleichstrom mit C-Gleis-Bettungsgeometrie)</li>
            <li><strong>H0 (1:87)</strong>: Piko A-Gleis (Zweileiter-Gleichstrom Schwellengleis)</li>
            <li><strong>H0 (1:87)</strong>: Roco Line (Zweileiter-Gleichstrom Gleis mit Bettung)</li>
            <li><strong>H0 (1:87)</strong>: Peco Streamline Code 100 (Universelles Zweileiter-Schwellengleissystem)</li>
            <li><strong>H0 (1:87)</strong>: Tillig Elite (Vorbildgetreues Zweileiter-Gleissystem mit feinem Schienenprofil)</li>

            <!-- TT (1:120) -->
            <li><strong>TT (1:120)</strong>: Tillig TT Modellgleis</li>
            <li><strong>TT (1:120)</strong>: Tillig TT Bettungsgleis (Gleissystem mit integrierter Schotterbettung)</li>
            <li><strong>TT (1:120)</strong>: Kuehn TT Gleis (Zweileiter-Gleissystem für Spur TT)</li>

            <!-- N (1:160) -->
            <li><strong>N (1:160)</strong>: Fleischmann Piccolo (Spur N Bettungsgleis)</li>
            <li><strong>N (1:160)</strong>: Kato Unitrack (N-Gleis mit integriertem Schotterbett und Unijoiner-Steckverbindung)</li>
            <li><strong>N (1:160)</strong>: Minitrix (Klassisches Zweileiter-Schwellengleis für Spur N)</li>

            <!-- Z (1:220) -->
            <li><strong>Z (1:220)</strong>: Märklin mini-club (Klassisches Spur-Z-Zweileitergleis)</li>
            <li><strong>Z (1:220)</strong>: Rokuhan (Spur-Z-Gleissystem mit integriertem Schotterbett)</li>
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
