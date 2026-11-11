import type { getPieceListSummary } from "@/io/export";
import { getTranslations } from "@/i18n";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import type { Sidebar } from "../sidebar";

export function renderBomTab(
  sidebar: Sidebar,
  container: HTMLElement,
  summary: ReturnType<typeof getPieceListSummary>,
) {
  const tBom = getTranslations().sidebar.bom;

  const filterBar = document.createElement("div");
  filterBar.className = "bom-filter-bar";
  filterBar.innerHTML = `
    <button type="button" class="bom-filter-btn ${sidebar.activePartsCategory === "all" ? "active" : ""}" data-cat="all">
      ${tBom.categories.all} (${summary.totalObjects})
    </button>
    <button type="button" class="bom-filter-btn ${sidebar.activePartsCategory === "tracks" ? "active" : ""}" data-cat="tracks">
      ${tBom.categories.tracks} (${summary.tracks.totalPieces})
    </button>
    <button type="button" class="bom-filter-btn ${sidebar.activePartsCategory === "roads" ? "active" : ""}" data-cat="roads">
      ${tBom.categories.roads} (${summary.roads.totalPieces})
    </button>
    <button type="button" class="bom-filter-btn ${sidebar.activePartsCategory === "buildings" ? "active" : ""}" data-cat="buildings">
      ${tBom.categories.buildings} (${summary.buildings.totalPieces})
    </button>
  `;

  filterBar.querySelectorAll(".bom-filter-btn").forEach((b) => {
    b.addEventListener("click", () => {
      const cat = b.getAttribute("data-cat") as typeof sidebar.activePartsCategory;
      if (cat) {
        sidebar.activePartsCategory = cat;
        sidebar.render();
      }
    });
  });
  container.appendChild(filterBar);

  const summaryBox = document.createElement("div");
  summaryBox.className = "bom-summary-box";

  if (sidebar.activePartsCategory === "all") {
    summaryBox.innerHTML = `
      <div class="summary-metric">
        <span class="metric-val">${summary.totalObjects}</span>
        <span class="metric-lbl">${tBom.categories.all}</span>
      </div>
      <div class="summary-metric">
        <span class="metric-val">${summary.tracks.totalPieces}</span>
        <span class="metric-lbl">${summary.tracks.totalLengthMeters.toFixed(2)} m ${tBom.categories.tracks}</span>
      </div>
      <div class="summary-metric">
        <span class="metric-val">${summary.roads.totalPieces}</span>
        <span class="metric-lbl">${summary.roads.totalLengthMeters.toFixed(2)} m ${tBom.categories.roads}</span>
      </div>
    `;
  } else if (sidebar.activePartsCategory === "tracks") {
    summaryBox.innerHTML = `
      <div class="summary-metric">
        <span class="metric-val">${summary.tracks.totalPieces}</span>
        <span class="metric-lbl">${getTranslations().toolbar.status.tracksUnitPlural}</span>
      </div>
      <div class="summary-metric">
        <span class="metric-val">${summary.tracks.totalLengthMeters.toFixed(2)} m</span>
        <span class="metric-lbl">${tBom.colLength}</span>
      </div>
    `;
  } else if (sidebar.activePartsCategory === "roads") {
    summaryBox.innerHTML = `
      <div class="summary-metric">
        <span class="metric-val">${summary.roads.totalPieces}</span>
        <span class="metric-lbl">${tBom.categories.roads}</span>
      </div>
      <div class="summary-metric">
        <span class="metric-val">${summary.roads.totalLengthMeters.toFixed(2)} m</span>
        <span class="metric-lbl">${tBom.colLength}</span>
      </div>
    `;
  } else {
    summaryBox.innerHTML = `
      <div class="summary-metric">
        <span class="metric-val">${summary.buildings.totalPieces}</span>
        <span class="metric-lbl">${tBom.categories.buildings}</span>
      </div>
    `;
  }
  container.appendChild(summaryBox);

  const exportBtn = document.createElement("button");
  exportBtn.className = "btn-export-csv";
  exportBtn.innerHTML = tBom.exportCsvBtn;
  exportBtn.disabled = summary.totalObjects === 0;
  exportBtn.addEventListener("click", () => sidebar.callbacks.onExportCSV());
  container.appendChild(exportBtn);

  if (summary.totalObjects === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.innerHTML = `<p>${tBom.emptyState}</p>`;
    container.appendChild(empty);
    return;
  }

  const showTracks = sidebar.activePartsCategory === "all" || sidebar.activePartsCategory === "tracks";
  const showRoads = sidebar.activePartsCategory === "all" || sidebar.activePartsCategory === "roads";
  const showBuildings = sidebar.activePartsCategory === "all" || sidebar.activePartsCategory === "buildings";

  if (showTracks) {
    if (summary.tracks.items.length > 0) {
      const secHeader = document.createElement("div");
      secHeader.className = "bom-section-header";
      secHeader.innerHTML = `<span>${tBom.tracksHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.tracks.totalPieces} Stk. • ${summary.tracks.totalLengthMeters.toFixed(2)} m</span>`;
      container.appendChild(secHeader);

      const table = document.createElement("table");
      table.className = "bom-table";
      table.innerHTML = `
        <thead>
          <tr>
            <th>${tBom.colId}</th>
            <th>${tBom.colName}</th>
            <th style="text-align: right;">${tBom.colCount}</th>
          </tr>
        </thead>
        <tbody>
          ${summary.tracks.items
            .map((it) => {
              const bp = getTrackBlueprintColor(it.catalogId);
              return `
            <tr>
              <td>
                <span class="bom-id-badge" style="background: ${bp.badgeBg}; color: #ffffff; border: 1px solid ${bp.border};">
                  ${it.catalogId}
                </span>
              </td>
              <td>
                <div class="bom-name">${it.name}</div>
                <div class="bom-sub">${it.specification}</div>
              </td>
              <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
            </tr>
          `;
            })
            .join("")}
        </tbody>
      `;
      container.appendChild(table);
    } else if (sidebar.activePartsCategory === "tracks") {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
      container.appendChild(empty);
    }
  }

  if (showRoads) {
    if (summary.roads.items.length > 0) {
      const secHeader = document.createElement("div");
      secHeader.className = "bom-section-header";
      secHeader.innerHTML = `<span>${tBom.roadsHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.roads.totalPieces} Stk. • ${summary.roads.totalLengthMeters.toFixed(2)} m</span>`;
      container.appendChild(secHeader);

      const table = document.createElement("table");
      table.className = "bom-table";
      table.innerHTML = `
        <thead>
          <tr>
            <th>${tBom.colId}</th>
            <th>${tBom.colName}</th>
            <th style="text-align: right;">${tBom.colCount}</th>
          </tr>
        </thead>
        <tbody>
          ${summary.roads.items
            .map(
              (it) => `
            <tr>
              <td>
                <span class="bom-id-badge" style="background: #334155; color: #f8fafc; border: 1px solid #64748b;">
                  ${it.catalogId}
                </span>
              </td>
              <td>
                <div class="bom-name">${it.name}</div>
                <div class="bom-sub">${it.specification}</div>
              </td>
              <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
            </tr>
          `,
            )
            .join("")}
        </tbody>
      `;
      container.appendChild(table);
    } else if (sidebar.activePartsCategory === "roads") {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
      container.appendChild(empty);
    }
  }

  if (showBuildings) {
    if (summary.buildings.items.length > 0) {
      const secHeader = document.createElement("div");
      secHeader.className = "bom-section-header";
      secHeader.innerHTML = `<span>${tBom.buildingsHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.buildings.totalPieces} Stk.</span>`;
      container.appendChild(secHeader);

      const table = document.createElement("table");
      table.className = "bom-table";
      table.innerHTML = `
        <thead>
          <tr>
            <th>${tBom.colName}</th>
            <th>${tBom.colDimensions}</th>
            <th style="text-align: right;">${tBom.colCount}</th>
          </tr>
        </thead>
        <tbody>
          ${summary.buildings.items
            .map(
              (it) => `
            <tr>
              <td>
                <div class="bom-name" style="font-weight: 600;">${it.name}</div>
                <div class="bom-sub">${it.category} • ${it.type}</div>
              </td>
              <td>
                <span style="font-size: 10px; color: var(--text-muted); font-family: monospace;">${it.dimensions}</span>
              </td>
              <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
            </tr>
          `,
            )
            .join("")}
        </tbody>
      `;
      container.appendChild(table);
    } else if (sidebar.activePartsCategory === "buildings") {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
      container.appendChild(empty);
    }
  }
}
