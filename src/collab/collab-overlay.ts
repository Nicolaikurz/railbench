import type { Renderer } from "@/render/renderer";
import type { CollabUser } from "./types";

export class CollabOverlay {
  private container: HTMLElement;
  private cursorsMap: Map<string, HTMLElement> = new Map();
  private renderer: Renderer;
  private users: CollabUser[] = [];

  constructor(host: HTMLElement, renderer: Renderer) {
    this.renderer = renderer;

    this.container = document.createElement("div");
    this.container.className = "collab-overlay";
    this.container.style.position = "absolute";
    this.container.style.inset = "0";
    this.container.style.pointerEvents = "none";
    this.container.style.zIndex = "85";
    this.container.style.overflow = "hidden";
    host.appendChild(this.container);
  }

  setUsers(users: CollabUser[]) {
    this.users = users;
    const activeIds = new Set(users.map((u) => u.id));

    // Remove cursors for users that left
    for (const [id, el] of this.cursorsMap.entries()) {
      if (!activeIds.has(id)) {
        el.remove();
        this.cursorsMap.delete(id);
      }
    }

    this.update();
  }

  updateCursor(userId: string, worldX: number, worldY: number) {
    const user = this.users.find((u) => u.id === userId);
    if (!user) return;
    user.cursor = { x: worldX, y: worldY };

    let el = this.cursorsMap.get(userId);
    if (!el) {
      el = this.createCursorElement(user);
      this.cursorsMap.set(userId, el);
      this.container.appendChild(el);
    }

    const screenPos = this.renderer.worldToScreen(worldX, worldY);
    el.style.transform = `translate3d(${screenPos.x}px, ${screenPos.y}px, 0)`;
    el.style.display = "flex";
  }

  update() {
    for (const user of this.users) {
      if (user.isLocal || !user.cursor) continue;

      let el = this.cursorsMap.get(user.id);
      if (!el) {
        el = this.createCursorElement(user);
        this.cursorsMap.set(user.id, el);
        this.container.appendChild(el);
      }

      const screenPos = this.renderer.worldToScreen(user.cursor.x, user.cursor.y);
      el.style.transform = `translate3d(${screenPos.x}px, ${screenPos.y}px, 0)`;
      el.style.display = "flex";
    }
  }

  clear() {
    this.container.innerHTML = "";
    this.cursorsMap.clear();
    this.users = [];
  }

  private createCursorElement(user: CollabUser): HTMLElement {
    const el = document.createElement("div");
    el.className = "collab-cursor";
    el.style.position = "absolute";
    el.style.top = "0";
    el.style.left = "0";
    el.style.display = "flex";
    el.style.flexDirection = "column";
    el.style.alignItems = "flex-start";
    el.style.pointerEvents = "none";
    el.style.transition = "transform 0.06s ease-out";
    el.style.zIndex = "88";

    // SVG Cursor arrow
    const svg = `
      <svg width="20" height="20" viewBox="0 0 24 24" fill="${user.color}" stroke="#ffffff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="filter: drop-shadow(0 2px 4px rgba(0,0,0,0.3));">
        <polygon points="3 3 10.07 19.97 12.58 12.58 19.97 10.07 3 3"/>
      </svg>
    `;

    // Name badge
    const badge = document.createElement("span");
    badge.className = "collab-cursor-badge";
    badge.textContent = user.name;
    badge.style.backgroundColor = user.color;
    badge.style.color = "#ffffff";
    badge.style.fontSize = "11px";
    badge.style.fontWeight = "700";
    badge.style.padding = "2px 6px";
    badge.style.borderRadius = "4px";
    badge.style.marginLeft = "12px";
    badge.style.marginTop = "-4px";
    badge.style.boxShadow = "0 2px 6px rgba(0,0,0,0.25)";
    badge.style.whiteSpace = "nowrap";

    el.innerHTML = svg;
    el.appendChild(badge);
    return el;
  }
}

