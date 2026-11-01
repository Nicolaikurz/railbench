#!/usr/bin/env node
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";
import { WebSocketServer, WebSocket } from "ws";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Start variables & configuration
const args = process.argv.slice(2);
function getArg(flags, fallback = undefined) {
  const flagList = Array.isArray(flags) ? flags : [flags];
  for (const flag of flagList) {
    const idx = args.indexOf(flag);
    if (idx !== -1 && args[idx + 1] && !args[idx + 1].startsWith("--")) {
      return args[idx + 1];
    }
    const prefix = `${flag}=`;
    const match = args.find((a) => a.startsWith(prefix));
    if (match) return match.substring(prefix.length);
  }
  return fallback;
}

const rawPort =
  getArg(["--port", "--server-port", "--collab-port"]) ||
  process.env.RAILBENCH_SERVER_PORT ||
  process.env.COLLAB_PORT ||
  process.env.PORT ||
  "3000";
const PORT = parseInt(rawPort, 10);

const HOST =
  getArg(["--host", "--server-host"]) ||
  process.env.RAILBENCH_SERVER_HOST ||
  process.env.HOST ||
  "0.0.0.0";

const DEFAULT_PASSWORD =
  getArg(["--password", "--server-password", "--collab-password"]) ||
  process.env.RAILBENCH_SERVER_PASSWORD ||
  process.env.COLLAB_PASSWORD ||
  process.env.PASSWORD ||
  null;

const DEFAULT_ROOM =
  getArg(["--room", "--server-room", "--collab-room"]) ||
  process.env.RAILBENCH_SERVER_ROOM ||
  process.env.COLLAB_ROOM ||
  process.env.ROOM ||
  "default";

const DIST_DIR = path.resolve(__dirname, "dist");
const PLANS_DIR = path.resolve(__dirname, "saved_plans");

// Ensure plans storage directory exists
if (!fs.existsSync(PLANS_DIR)) {
  fs.mkdirSync(PLANS_DIR, { recursive: true });
}

// Pre-initialize default room with configured password if set
if (DEFAULT_PASSWORD) {
  setTimeout(() => {
    getOrCreateRoom(DEFAULT_ROOM, DEFAULT_PASSWORD);
  }, 0);
}

// MIME types for static assets
const MIME_TYPES = {
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
  ".txt": "text/plain; charset=utf-8",
};

/**
 * Collaboration Room Class
 */
class CollabRoom {
  constructor(name, password = null) {
    this.name = name;
    this.password = password;
    this.clients = new Map(); // ws -> user profile
    this.layout = null;
    this.fileName = `${name}.gleisplan`;
    this.saveTimeout = null;

    this.loadFromDisk();
  }

  loadFromDisk() {
    const filePath = path.join(PLANS_DIR, `${this.name}.gleisplan`);
    if (fs.existsSync(filePath)) {
      try {
        const raw = fs.readFileSync(filePath, "utf-8");
        this.layout = JSON.parse(raw);
        console.log(`[Room: ${this.name}] Geladen aus ${filePath}`);
      } catch (err) {
        console.error(`[Room: ${this.name}] Fehler beim Laden von ${filePath}:`, err);
      }
    }
  }

  scheduleSave() {
    if (this.saveTimeout) clearTimeout(this.saveTimeout);
    this.saveTimeout = setTimeout(() => {
      this.saveToDisk();
    }, 2000);
  }

  saveToDisk() {
    if (!this.layout) return;
    const filePath = path.join(PLANS_DIR, `${this.name}.gleisplan`);
    try {
      fs.writeFileSync(filePath, JSON.stringify(this.layout, null, 2), "utf-8");
      console.log(`[Room: ${this.name}] Gespeichert unter ${filePath}`);
    } catch (err) {
      console.error(`[Room: ${this.name}] Fehler beim Speichern:`, err);
    }
  }

  getUsersList() {
    return Array.from(this.clients.values()).map((u) => ({
      id: u.id,
      name: u.name,
      color: u.color,
      cursor: u.cursor,
      selectedIds: u.selectedIds || [],
    }));
  }

  broadcast(messageObj, excludeWs = null) {
    const raw = JSON.stringify(messageObj);
    for (const [ws] of this.clients) {
      if (ws !== excludeWs && ws.readyState === WebSocket.OPEN) {
        try {
          ws.send(raw);
        } catch (e) {
          console.error(`[Room: ${this.name}] Sende-Fehler:`, e);
        }
      }
    }
  }
}

const rooms = new Map(); // roomName -> CollabRoom

function getOrCreateRoom(roomName, password = null) {
  const cleanName = (roomName || "default").trim().replace(/[^a-zA-Z0-9_\-\.]/g, "_") || "default";
  let room = rooms.get(cleanName);
  if (!room) {
    room = new CollabRoom(cleanName, password);
    rooms.set(cleanName, room);
  } else if (!room.password && password) {
    room.password = password;
  }
  return room;
}

// HTTP Server for static frontend & status API
const server = http.createServer((req, res) => {
  const urlObj = new URL(req.url, `http://${req.headers.host || "localhost"}`);
  let pathname = decodeURIComponent(urlObj.pathname);

  // Status API
  if (pathname === "/api/status") {
    let totalUsers = 0;
    const roomSummaries = [];
    for (const [name, room] of rooms.entries()) {
      totalUsers += room.clients.size;
      roomSummaries.push({
        name,
        usersCount: room.clients.size,
        hasLayout: !!room.layout,
      });
    }
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(
      JSON.stringify({
        ok: true,
        version: "0.4.0",
        uptimeSeconds: Math.floor(process.uptime()),
        totalConnections: totalUsers,
        rooms: roomSummaries,
      })
    );
    return;
  }

  // Static files from dist/
  if (!fs.existsSync(DIST_DIR)) {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    res.end(`
      <!doctype html>
      <html>
        <head><title>Railbench Collaboration Server</title><meta charset="utf-8"></head>
        <body style="font-family: system-ui; max-width: 600px; margin: 40px auto; padding: 20px; line-height: 1.6; background: #0f172a; color: #f8fafc;">
          <h2>🚆 Railbench Server läuft!</h2>
          <p>Der WebSocket-Server für Multi-User-Kollaboration ist aktiv auf Port <strong>${PORT}</strong>.</p>
          <div style="background: #1e293b; padding: 15px; border-radius: 8px; border: 1px solid #334155;">
            <p><strong>Frontend bereitstellen:</strong></p>
            <p>Führen Sie im Projektverzeichnis aus:</p>
            <code style="background: #0284c7; color: white; padding: 4px 8px; border-radius: 4px;">npm run build</code>
            <p>Danach neu laden, um die Web-App direkt über diesen Server zu nutzen.</p>
          </div>
        </body>
      </html>
    `);
    return;
  }

  let filePath = path.join(DIST_DIR, pathname === "/" ? "index.html" : pathname);

  // Check if target is inside DIST_DIR to prevent directory traversal
  if (!filePath.startsWith(DIST_DIR)) {
    res.writeHead(403);
    res.end("403 Forbidden");
    return;
  }

  // If path doesn't exist or is a directory, fallback to index.html (SPA)
  if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
    filePath = path.join(DIST_DIR, "index.html");
  }

  try {
    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] || "application/octet-stream";
    const data = fs.readFileSync(filePath);
    res.writeHead(200, {
      "Content-Type": contentType,
      "Cache-Control": ext === ".html" ? "no-cache" : "public, max-age=31536000, immutable",
    });
    res.end(data);
  } catch (err) {
    res.writeHead(500);
    res.end("500 Internal Server Error");
  }
});

// WebSocket Server for collaboration
const wss = new WebSocketServer({ server });

wss.on("connection", (ws, req) => {
  let currentRoom = null;
  let currentUser = null;
  let isAlive = true;

  ws.on("pong", () => {
    isAlive = true;
  });

  ws.on("message", (raw) => {
    try {
      const msg = JSON.parse(raw.toString());

      switch (msg.type) {
        case "join": {
          const roomName = msg.room || "default";
          const requestedPass = msg.password ? String(msg.password).trim() : null;
          const targetRoom = getOrCreateRoom(roomName, requestedPass);

          // Password validation
          if (targetRoom.password && targetRoom.password !== requestedPass) {
            ws.send(
              JSON.stringify({
                type: "error",
                code: "INVALID_PASSWORD",
                message: "Falsches Passwort für diesen Raum.",
              })
            );
            return;
          }

          currentRoom = targetRoom;

          currentUser = {
            id: msg.user?.id || `user-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
            name: (msg.user?.name || "Planer").trim().substring(0, 32),
            color: msg.user?.color || "#3b82f6",
            cursor: null,
            selectedIds: [],
          };

          currentRoom.clients.set(ws, currentUser);

          // If client sent initial layout and room has none yet, adopt it
          if (!currentRoom.layout && msg.initialLayout) {
            currentRoom.layout = msg.initialLayout;
            currentRoom.scheduleSave();
          }

          // Welcome new client with current state
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

          // Broadcast to others in room
          currentRoom.broadcast(
            {
              type: "user_joined",
              user: currentUser,
            },
            ws
          );

          console.log(`[Room: ${currentRoom.name}] + ${currentUser.name} beigetreten (${currentRoom.clients.size} online)`);
          break;
        }

        case "layout_update": {
          if (!currentRoom || !currentUser) return;
          if (msg.layout) {
            currentRoom.layout = msg.layout;
            currentRoom.scheduleSave();

            // Broadcast to other collaborators
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

        default:
          break;
      }
    } catch (err) {
      console.error("Fehler beim Verarbeiten der WebSocket-Nachricht:", err);
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

      console.log(`[Room: ${currentRoom.name}] - ${currentUser.name} verlassen (${currentRoom.clients.size} online)`);
    }
  });

  ws.on("error", (err) => {
    console.error("WebSocket Client Error:", err);
  });
});

// Periodic ping/pong check to clean up dead sockets
const heartbeatInterval = setInterval(() => {
  for (const client of wss.clients) {
    if (client.isAlive === false) {
      client.terminate();
      continue;
    }
    client.isAlive = false;
    client.ping();
  }
}, 30000);

wss.on("close", () => {
  clearInterval(heartbeatInterval);
});

// Graceful shutdown
process.on("SIGINT", () => {
  console.log("\nServer wird heruntergefahren... Speichere aktive Räume...");
  for (const room of rooms.values()) {
    room.saveToDisk();
  }
  process.exit(0);
});

process.on("SIGTERM", () => {
  for (const room of rooms.values()) {
    room.saveToDisk();
  }
  process.exit(0);
});

// Start listening
server.listen(PORT, HOST, () => {
  const nets = os.networkInterfaces();
  const ips = [];
  for (const name of Object.keys(nets)) {
    for (const net of nets[name] || []) {
      if (net.family === "IPv4" && !net.internal) {
        ips.push(net.address);
      }
    }
  }

  console.log("\n==================================================================");
  console.log(" 🚆 Railbench Multi-User Collaboration Server gestartet!");
  console.log("==================================================================");
  console.log(` Local:      http://localhost:${PORT}`);
  if (ips.length > 0) {
    for (const ip of ips) {
      console.log(` Netzwerk:   http://${ip}:${PORT}`);
    }
  }
  console.log(` WebSocket:  ws://<host>:${PORT}/collab`);
  console.log(` Raum:       ${DEFAULT_ROOM}`);
  console.log(` Schutz:     ${DEFAULT_PASSWORD ? "Passwort aktiv" : "Kein Passwort (offen)"}`);
  console.log(` Pläne:      ${PLANS_DIR}`);
  console.log("==================================================================\n");
});

