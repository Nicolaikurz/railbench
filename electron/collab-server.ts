import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import net from "node:net";
import { WebSocketServer, WebSocket } from "ws";

interface EmbeddedUser {
  id: string;
  name: string;
  color: string;
  cursor?: { x: number; y: number } | null;
  selectedIds?: string[];
}

class EmbeddedRoom {
  public name: string;
  public password: string | null;
  public clients: Map<WebSocket, EmbeddedUser> = new Map();
  public layout: unknown = null;
  private saveTimeout: ReturnType<typeof setTimeout> | null = null;
  private plansDir: string;

  constructor(name: string, password: string | null = null, plansDir: string) {
    this.name = name;
    this.password = password;
    this.plansDir = plansDir;
    this.loadFromDisk();
  }

  loadFromDisk(): void {
    try {
      const filePath = path.join(this.plansDir, `${this.name}.railbench`);
      if (fs.existsSync(filePath)) {
        const raw = fs.readFileSync(filePath, "utf-8");
        this.layout = JSON.parse(raw);
      }
    } catch (err) {
      console.warn(`[CollabRoom: ${this.name}] Laden fehlgeschlagen:`, err);
    }
  }

  scheduleSave(): void {
    if (this.saveTimeout) clearTimeout(this.saveTimeout);
    this.saveTimeout = setTimeout(() => {
      this.saveToDisk();
    }, 2000);
  }

  saveToDisk(): void {
    if (!this.layout) return;
    try {
      if (!fs.existsSync(this.plansDir)) {
        fs.mkdirSync(this.plansDir, { recursive: true });
      }
      const filePath = path.join(this.plansDir, `${this.name}.railbench`);
      fs.writeFileSync(filePath, JSON.stringify(this.layout, null, 2), "utf-8");
    } catch (err) {
      console.warn(`[CollabRoom: ${this.name}] Speichern fehlgeschlagen:`, err);
    }
  }

  getUsersList(): EmbeddedUser[] {
    return Array.from(this.clients.values()).map((u) => ({
      id: u.id,
      name: u.name,
      color: u.color,
      cursor: u.cursor,
      selectedIds: u.selectedIds || [],
    }));
  }

  broadcast(messageObj: object, excludeWs: WebSocket | null = null): void {
    const raw = JSON.stringify(messageObj);
    for (const [ws] of this.clients) {
      if (ws !== excludeWs && ws.readyState === WebSocket.OPEN) {
        try {
          ws.send(raw);
        } catch {}
      }
    }
  }
}

export interface CollabServerOptions {
  port?: number;
  password?: string;
  defaultRoom?: string;
}

export interface CollabServerStatus {
  running: boolean;
  port?: number;
  localUrl?: string;
  networkUrls?: string[];
  wsUrl?: string;
  networkWsUrls?: string[];
  password?: string;
  defaultRoom?: string;
  connectionsCount?: number;
  roomsCount?: number;
}

const MIME_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".mjs": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".wasm": "application/wasm",
  ".glb": "model/gltf-binary",
  ".gltf": "model/gltf+json",
};

export class EmbeddedCollabServer {
  private server: http.Server | null = null;
  private wss: WebSocketServer | null = null;
  private rooms: Map<string, EmbeddedRoom> = new Map();
  private isRunning = false;
  private actualPort = 3000;
  private currentPassword = "";
  private currentDefaultRoom = "default";
  private plansDir: string;
  private distDir: string;

  constructor() {
    this.plansDir = path.resolve(process.cwd(), "saved_plans");
    this.distDir = path.resolve(__dirname, "../dist");
  }

  private async findAvailablePort(startPort: number): Promise<number> {
    return new Promise((resolve) => {
      const tester = net.createServer();
      tester.once("error", () => {
        resolve(this.findAvailablePort(startPort + 1));
      });
      tester.once("listening", () => {
        tester.close(() => resolve(startPort));
      });
      tester.listen(startPort, "0.0.0.0");
    });
  }

  private getLocalIpAddresses(): string[] {
    const nets = os.networkInterfaces();
    const ips: string[] = [];
    for (const name of Object.keys(nets)) {
      for (const netInfo of nets[name] || []) {
        if (netInfo.family === "IPv4" && !netInfo.internal) {
          ips.push(netInfo.address);
        }
      }
    }
    return ips;
  }

  public async start(options: CollabServerOptions = {}): Promise<CollabServerStatus> {
    if (this.isRunning && this.server) {
      if (options.password !== undefined) {
        this.currentPassword = options.password.trim();
        // Update default room password
        const defRoom = this.rooms.get(this.currentDefaultRoom);
        if (defRoom) {
          defRoom.password = this.currentPassword || null;
        }
      }
      return this.getStatus();
    }

    const preferredPort = options.port || 3000;
    this.actualPort = await this.findAvailablePort(preferredPort);
    this.currentPassword = options.password ? options.password.trim() : "";
    this.currentDefaultRoom = options.defaultRoom || "default";

    // Setup HTTP server
    this.server = http.createServer((req, res) => {
      const urlObj = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`);
      const pathname = decodeURIComponent(urlObj.pathname);

      if (pathname === "/api/status") {
        let totalUsers = 0;
        for (const r of this.rooms.values()) {
          totalUsers += r.clients.size;
        }
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({
            ok: true,
            running: true,
            port: this.actualPort,
            totalUsers,
            roomsCount: this.rooms.size,
          })
        );
        return;
      }

      // Serve static frontend from dist
      if (!fs.existsSync(this.distDir)) {
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(`
          <!doctype html>
          <html>
            <head><title>Railbench Collaboration</title><meta charset="utf-8"></head>
            <body style="font-family: system-ui; background: #0f172a; color: #f8fafc; padding: 40px; text-align: center;">
              <h2>🚆 Railbench Collaboration Server aktiv!</h2>
              <p>Port: <strong>${this.actualPort}</strong></p>
            </body>
          </html>
        `);
        return;
      }

      let filePath = path.join(this.distDir, pathname === "/" ? "index.html" : pathname);
      if (!filePath.startsWith(this.distDir)) {
        res.writeHead(403);
        res.end("403 Forbidden");
        return;
      }

      if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
        filePath = path.join(this.distDir, "index.html");
      }

      try {
        const ext = path.extname(filePath).toLowerCase();
        const contentType = MIME_TYPES[ext] || "application/octet-stream";
        const content = fs.readFileSync(filePath);
        res.writeHead(200, {
          "Content-Type": contentType,
          "Cache-Control": ext === ".html" ? "no-cache" : "public, max-age=31536000, immutable",
        });
        res.end(content);
      } catch {
        res.writeHead(500);
        res.end("Internal Server Error");
      }
    });

    // Setup WebSocket server
    this.wss = new WebSocketServer({ server: this.server });

    this.wss.on("connection", (ws) => {
      let currentRoom: EmbeddedRoom | null = null;
      let currentUser: EmbeddedUser | null = null;

      ws.on("message", (raw) => {
        try {
          const msg = JSON.parse(raw.toString());

          switch (msg.type) {
            case "join": {
              const roomName = (msg.room || this.currentDefaultRoom).trim() || "default";
              const requestedPass = msg.password ? String(msg.password).trim() : null;

              let room = this.rooms.get(roomName);
              if (!room) {
                const initialPass = requestedPass || (roomName === this.currentDefaultRoom && this.currentPassword ? this.currentPassword : null);
                room = new EmbeddedRoom(roomName, initialPass, this.plansDir);
                this.rooms.set(roomName, room);
              } else if (!room.password && requestedPass) {
                room.password = requestedPass;
              }

              // Password validation
              if (room.password && room.password !== requestedPass) {
                ws.send(
                  JSON.stringify({
                    type: "error",
                    code: "INVALID_PASSWORD",
                    message: "Falsches Passwort für diesen Raum.",
                  })
                );
                return;
              }

              currentRoom = room;

              currentUser = {
                id: msg.user?.id || `user-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
                name: (msg.user?.name || "Planer").trim().substring(0, 32),
                color: msg.user?.color || "#3b82f6",
                cursor: null,
                selectedIds: [],
              };

              currentRoom.clients.set(ws, currentUser);

              if (!currentRoom.layout && msg.initialLayout) {
                currentRoom.layout = msg.initialLayout;
                currentRoom.scheduleSave();
              }

              ws.send(
                JSON.stringify({
                  type: "init",
                  room: currentRoom.name,
                  user: currentUser,
                  users: currentRoom.getUsersList(),
                  layout: currentRoom.layout,
                  hasPassword: !!currentRoom.password,
                  isHost: currentRoom.clients.size === 1,
                })
              );

              currentRoom.broadcast(
                {
                  type: "user_joined",
                  user: currentUser,
                },
                ws
              );
              break;
            }

            case "layout_update": {
              if (!currentRoom || !currentUser) return;
              if (msg.layout) {
                currentRoom.layout = msg.layout;
                currentRoom.scheduleSave();
                currentRoom.broadcast(
                  {
                    type: "layout_update",
                    layout: currentRoom.layout,
                    fromUserId: currentUser.id,
                    fromUserName: currentUser.name,
                  },
                  ws
                );
              }
              break;
            }

            case "cursor": {
              if (!currentRoom || !currentUser) return;
              currentUser.cursor = { x: msg.x, y: msg.y };
              currentRoom.broadcast(
                {
                  type: "cursor",
                  userId: currentUser.id,
                  x: msg.x,
                  y: msg.y,
                },
                ws
              );
              break;
            }

            case "selection": {
              if (!currentRoom || !currentUser) return;
              currentUser.selectedIds = Array.isArray(msg.selectedIds) ? msg.selectedIds : [];
              currentRoom.broadcast(
                {
                  type: "selection",
                  userId: currentUser.id,
                  selectedIds: currentUser.selectedIds,
                },
                ws
              );
              break;
            }

            case "chat": {
              if (!currentRoom || !currentUser) return;
              const text = (msg.text || "").trim().substring(0, 500);
              if (!text) return;
              currentRoom.broadcast({
                type: "chat",
                id: `msg-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
                userId: currentUser.id,
                userName: currentUser.name,
                userColor: currentUser.color,
                text,
                timestamp: Date.now(),
              });
              break;
            }
          }
        } catch (err) {
          console.warn("WebSocket parse error:", err);
        }
      });

      ws.on("close", () => {
        if (currentRoom && currentUser) {
          currentRoom.clients.delete(ws);
          currentRoom.broadcast({
            type: "user_left",
            userId: currentUser.id,
            userName: currentUser.name,
          });
        }
      });
    });

    await new Promise<void>((resolve) => {
      this.server!.listen(this.actualPort, "0.0.0.0", () => {
        this.isRunning = true;
        resolve();
      });
    });

    return this.getStatus();
  }

  public async stop(): Promise<CollabServerStatus> {
    if (!this.isRunning) return this.getStatus();

    for (const room of this.rooms.values()) {
      room.saveToDisk();
      for (const [ws] of room.clients) {
        try {
          ws.close();
        } catch {}
      }
      room.clients.clear();
    }
    this.rooms.clear();

    if (this.wss) {
      this.wss.close();
      this.wss = null;
    }

    if (this.server) {
      await new Promise<void>((resolve) => {
        this.server!.close(() => resolve());
      });
      this.server = null;
    }

    this.isRunning = false;
    return this.getStatus();
  }

  public getStatus(): CollabServerStatus {
    const ips = this.getLocalIpAddresses();
    let totalConnections = 0;
    for (const r of this.rooms.values()) {
      totalConnections += r.clients.size;
    }

    return {
      running: this.isRunning,
      port: this.actualPort,
      localUrl: `http://localhost:${this.actualPort}`,
      networkUrls: ips.map((ip) => `http://${ip}:${this.actualPort}`),
      wsUrl: `ws://localhost:${this.actualPort}`,
      networkWsUrls: ips.map((ip) => `ws://${ip}:${this.actualPort}`),
      password: this.currentPassword,
      defaultRoom: this.currentDefaultRoom,
      connectionsCount: totalConnections,
      roomsCount: this.rooms.size,
    };
  }
}

