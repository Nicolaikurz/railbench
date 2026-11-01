import { defineConfig, type Plugin } from "vite";
import { fileURLToPath } from "node:url";
import { WebSocketServer, WebSocket } from "ws";

function collabDevPlugin(): Plugin {
  return {
    name: "collab-dev-server",
    configureServer(server) {
      if (!server.httpServer) return;
      const wss = new WebSocketServer({ noServer: true });
      const rooms = new Map<string, any>();

      server.httpServer.on("upgrade", (req, socket, head) => {
        const url = new URL(req.url || "/", "http://localhost");
        if (url.pathname === "/collab" || url.pathname === "/ws") {
          wss.handleUpgrade(req, socket, head, (ws) => {
            wss.emit("connection", ws, req);
          });
        }
      });

      wss.on("connection", (ws) => {
        let currentRoom: any = null;
        let currentUser: any = null;

        ws.on("message", (raw) => {
          try {
            const msg = JSON.parse(raw.toString());
            switch (msg.type) {
              case "join": {
                const roomName = (msg.room || "default").trim();
                const requestedPass = msg.password ? String(msg.password).trim() : null;
                let room = rooms.get(roomName);
                if (!room) {
                  room = {
                    name: roomName,
                    password: requestedPass,
                    clients: new Map(),
                    layout: null,
                  };
                  rooms.set(roomName, room);
                } else if (!room.password && requestedPass) {
                  room.password = requestedPass;
                }

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
                  id: msg.user?.id || `user-${Date.now()}`,
                  name: (msg.user?.name || "Planer").trim().substring(0, 32),
                  color: msg.user?.color || "#3b82f6",
                  cursor: null,
                  selectedIds: [],
                };
                currentRoom.clients.set(ws, currentUser);

                if (!currentRoom.layout && msg.initialLayout) {
                  currentRoom.layout = msg.initialLayout;
                }

                ws.send(
                  JSON.stringify({
                    type: "init",
                    room: currentRoom.name,
                    user: currentUser,
                    users: Array.from(currentRoom.clients.values()),
                    layout: currentRoom.layout,
                    hasPassword: !!currentRoom.password,
                    isHost: currentRoom.clients.size === 1,
                  })
                );

                const joinPacket = JSON.stringify({ type: "user_joined", user: currentUser });
                for (const [otherWs] of currentRoom.clients) {
                  if (otherWs !== ws && otherWs.readyState === WebSocket.OPEN) {
                    otherWs.send(joinPacket);
                  }
                }
                break;
              }

              case "layout_update": {
                if (!currentRoom || !currentUser) return;
                currentRoom.layout = msg.layout;
                const updatePacket = JSON.stringify({
                  type: "layout_update",
                  layout: currentRoom.layout,
                  fromUserId: currentUser.id,
                  fromUserName: currentUser.name,
                });
                for (const [otherWs] of currentRoom.clients) {
                  if (otherWs !== ws && otherWs.readyState === WebSocket.OPEN) {
                    otherWs.send(updatePacket);
                  }
                }
                break;
              }

              case "cursor": {
                if (!currentRoom || !currentUser) return;
                currentUser.cursor = { x: msg.x, y: msg.y };
                const cursorPacket = JSON.stringify({
                  type: "cursor",
                  userId: currentUser.id,
                  x: msg.x,
                  y: msg.y,
                });
                for (const [otherWs] of currentRoom.clients) {
                  if (otherWs !== ws && otherWs.readyState === WebSocket.OPEN) {
                    otherWs.send(cursorPacket);
                  }
                }
                break;
              }

              case "selection": {
                if (!currentRoom || !currentUser) return;
                currentUser.selectedIds = msg.selectedIds || [];
                const selPacket = JSON.stringify({
                  type: "selection",
                  userId: currentUser.id,
                  selectedIds: currentUser.selectedIds,
                });
                for (const [otherWs] of currentRoom.clients) {
                  if (otherWs !== ws && otherWs.readyState === WebSocket.OPEN) {
                    otherWs.send(selPacket);
                  }
                }
                break;
              }

              case "chat": {
                if (!currentRoom || !currentUser) return;
                const text = (msg.text || "").trim().substring(0, 500);
                if (!text) return;
                const chatPacket = JSON.stringify({
                  type: "chat",
                  id: `msg-${Date.now()}`,
                  userId: currentUser.id,
                  userName: currentUser.name,
                  userColor: currentUser.color,
                  text,
                  timestamp: Date.now(),
                });
                for (const [otherWs] of currentRoom.clients) {
                  if (otherWs.readyState === WebSocket.OPEN) {
                    otherWs.send(chatPacket);
                  }
                }
                break;
              }
            }
          } catch {}
        });

        ws.on("close", () => {
          if (currentRoom && currentUser) {
            currentRoom.clients.delete(ws);
            const leavePacket = JSON.stringify({
              type: "user_left",
              userId: currentUser.id,
              userName: currentUser.name,
            });
            for (const [otherWs] of currentRoom.clients) {
              if (otherWs.readyState === WebSocket.OPEN) {
                otherWs.send(leavePacket);
              }
            }
          }
        });
      });
    },
  };
}

export default defineConfig({
  base: "./",
  resolve: { alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) } },
  plugins: [collabDevPlugin()],
});
