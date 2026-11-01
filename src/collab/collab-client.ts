import type { Layout } from "@/core/types";
import type {
  CollabUser,
  CollabConnectionStatus,
  CollabChatMessage,
  CollabClientMessage,
  CollabServerMessage,
} from "./types";

const DEFAULT_COLORS = [
  "#3b82f6", // Blau
  "#10b981", // Grün
  "#f59e0b", // Bernstein
  "#ec4899", // Pink
  "#8b5cf6", // Lila
  "#06b6d4", // Cyan
  "#f97316", // Orange
  "#14b8a6", // Türkis
];

export class CollabClient {
  private ws: WebSocket | null = null;
  private status: CollabConnectionStatus = "disconnected";
  private serverUrl = "";
  private room = "default";
  private currentPassword = "";
  private isHost = false;
  private currentUser: CollabUser;
  private users: Map<string, CollabUser> = new Map();
  private chatMessages: CollabChatMessage[] = [];

  private lastCursorSent = 0;
  private pendingCursor: { x: number; y: number } | null = null;
  private cursorTimer: ReturnType<typeof setTimeout> | null = null;

  public onStatusChange?: (status: CollabConnectionStatus, error?: string) => void;
  public onUsersChange?: (users: CollabUser[]) => void;
  public onRemoteLayout?: (layout: Layout, fromUserName: string) => void;
  public onRemoteCursor?: (userId: string, x: number, y: number) => void;
  public onRemoteSelection?: (userId: string, selectedIds: string[]) => void;
  public onChat?: (msg: CollabChatMessage) => void;
  public onPasswordError?: (message: string) => void;

  constructor() {
    const hasStorage = typeof localStorage !== "undefined";
    const savedName = hasStorage ? localStorage.getItem("railbench_collab_username") : null;
    const savedColor = hasStorage ? localStorage.getItem("railbench_collab_color") : null;
    const savedId = hasStorage ? localStorage.getItem("railbench_collab_userid") : null;
    const finalId = savedId || `u-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
    if (hasStorage) {
      localStorage.setItem("railbench_collab_userid", finalId);
    }

    this.currentUser = {
      id: finalId,
      name: savedName || `Planer-${Math.floor(100 + Math.random() * 900)}`,
      color: savedColor || DEFAULT_COLORS[Math.floor(Math.random() * DEFAULT_COLORS.length)]!,
      isLocal: true,
      selectedIds: [],
    };
  }

  getUser(): CollabUser {
    return this.currentUser;
  }

  getUsers(): CollabUser[] {
    return Array.from(this.users.values());
  }

  getStatus(): CollabConnectionStatus {
    return this.status;
  }

  getRoom(): string {
    return this.room;
  }

  getServerUrl(): string {
    return this.serverUrl;
  }

  getPassword(): string {
    return this.currentPassword;
  }

  setPassword(pwd: string): void {
    this.currentPassword = pwd ? pwd.trim() : "";
  }

  isHostUser(): boolean {
    return this.isHost;
  }

  getChatMessages(): CollabChatMessage[] {
    return [...this.chatMessages];
  }

  setProfile(name: string, color: string) {
    this.currentUser.name = name.trim() || this.currentUser.name;
    this.currentUser.color = color || this.currentUser.color;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("railbench_collab_username", this.currentUser.name);
      localStorage.setItem("railbench_collab_color", this.currentUser.color);
    }

    // If already connected, re-notify users
    if (this.status === "connected" && this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.send({
        type: "join",
        room: this.room,
        password: this.currentPassword || undefined,
        user: {
          id: this.currentUser.id,
          name: this.currentUser.name,
          color: this.currentUser.color,
        },
      });
    }
  }

  connect(
    serverUrl: string,
    roomName: string,
    userName?: string,
    userColor?: string,
    initialLayout?: Layout,
    password?: string,
  ) {
    if (userName || userColor) {
      this.setProfile(userName || this.currentUser.name, userColor || this.currentUser.color);
    }

    if (password !== undefined) {
      this.currentPassword = password ? password.trim() : "";
    }

    this.disconnect();

    this.serverUrl = serverUrl.trim();
    this.room = (roomName || "default").trim();

    this.setStatus("connecting");

    try {
      this.ws = new WebSocket(this.serverUrl);

      this.ws.onopen = () => {
        this.setStatus("connected");
        // Send join packet
        this.send({
          type: "join",
          room: this.room,
          password: this.currentPassword || undefined,
          user: {
            id: this.currentUser.id,
            name: this.currentUser.name,
            color: this.currentUser.color,
          },
          initialLayout,
        });
      };

      this.ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string) as CollabServerMessage;
          this.handleServerMessage(msg);
        } catch (err) {
          console.error("Collab: Ungültige Nachricht vom Server:", err);
        }
      };

      this.ws.onerror = (err) => {
        console.warn("Collab: Verbindungsfehler:", err);
        this.setStatus("disconnected", "Verbindungsfehler zum Kollaborations-Server.");
      };

      this.ws.onclose = () => {
        this.users.clear();
        this.onUsersChange?.([]);
        this.setStatus("disconnected");
      };
    } catch (err) {
      this.setStatus("disconnected", String(err));
    }
  }

  disconnect() {
    if (this.ws) {
      try {
        this.ws.close();
      } catch {}
      this.ws = null;
    }
    this.users.clear();
    this.onUsersChange?.([]);
    this.setStatus("disconnected");
  }

  private setStatus(status: CollabConnectionStatus, error?: string) {
    this.status = status;
    this.onStatusChange?.(status, error);
  }

  private handleServerMessage(msg: CollabServerMessage) {
    switch (msg.type) {
      case "init": {
        this.isHost = !!msg.isHost;
        this.users.clear();
        for (const u of msg.users) {
          if (u.id === this.currentUser.id) {
            this.currentUser = { ...u, isLocal: true };
          } else {
            this.users.set(u.id, u);
          }
        }
        this.onUsersChange?.(Array.from(this.users.values()));

        if (msg.layout) {
          this.onRemoteLayout?.(msg.layout, "Server");
        }
        break;
      }

      case "user_joined": {
        if (msg.user.id !== this.currentUser.id) {
          this.users.set(msg.user.id, msg.user);
          this.onUsersChange?.(Array.from(this.users.values()));
        }
        break;
      }

      case "user_left": {
        this.users.delete(msg.userId);
        this.onUsersChange?.(Array.from(this.users.values()));
        break;
      }

      case "layout_update": {
        if (msg.fromUserId !== this.currentUser.id) {
          this.onRemoteLayout?.(msg.layout, msg.fromUserName || "Teilnehmer");
        }
        break;
      }

      case "cursor": {
        if (msg.userId !== this.currentUser.id) {
          const u = this.users.get(msg.userId);
          if (u) {
            u.cursor = { x: msg.x, y: msg.y };
          }
          this.onRemoteCursor?.(msg.userId, msg.x, msg.y);
        }
        break;
      }

      case "selection": {
        if (msg.userId !== this.currentUser.id) {
          const u = this.users.get(msg.userId);
          if (u) {
            u.selectedIds = msg.selectedIds;
          }
          this.onRemoteSelection?.(msg.userId, msg.selectedIds);
        }
        break;
      }

      case "chat": {
        const chatMsg: CollabChatMessage = {
          id: msg.id,
          userId: msg.userId,
          userName: msg.userName,
          userColor: msg.userColor,
          text: msg.text,
          timestamp: msg.timestamp,
        };
        this.chatMessages.push(chatMsg);
        if (this.chatMessages.length > 100) this.chatMessages.shift();
        this.onChat?.(chatMsg);
        break;
      }

      case "error": {
        console.warn("Collab Server Error:", msg.message);
        if (msg.code === "INVALID_PASSWORD") {
          this.setStatus("disconnected", msg.message);
          this.onPasswordError?.(msg.message);
        } else {
          this.setStatus("disconnected", msg.message);
        }
        break;
      }
    }
  }

  broadcastLayout(layout: Layout) {
    if (this.status !== "connected") return;
    this.send({
      type: "layout_update",
      layout,
    });
  }

  broadcastCursor(x: number, y: number) {
    if (this.status !== "connected") return;

    const now = Date.now();
    this.pendingCursor = { x, y };

    // Throttle to 50ms
    if (now - this.lastCursorSent >= 50) {
      this.sendPendingCursor();
    } else if (!this.cursorTimer) {
      this.cursorTimer = setTimeout(() => {
        this.cursorTimer = null;
        this.sendPendingCursor();
      }, 50 - (now - this.lastCursorSent));
    }
  }

  private sendPendingCursor() {
    if (!this.pendingCursor) return;
    this.lastCursorSent = Date.now();
    this.send({
      type: "cursor",
      x: Math.round(this.pendingCursor.x),
      y: Math.round(this.pendingCursor.y),
    });
    this.pendingCursor = null;
  }

  broadcastSelection(selectedIds: string[]) {
    if (this.status !== "connected") return;
    this.currentUser.selectedIds = selectedIds;
    this.send({
      type: "selection",
      selectedIds,
    });
  }

  sendChat(text: string) {
    if (this.status !== "connected") return;
    const clean = text.trim();
    if (!clean) return;
    this.send({
      type: "chat",
      text: clean,
    });
  }

  private send(msg: CollabClientMessage) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }
}
