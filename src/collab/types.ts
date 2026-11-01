import type { Layout } from "@/core/types";

export interface CollabUser {
  id: string;
  name: string;
  color: string;
  cursor?: { x: number; y: number } | null;
  selectedIds?: string[];
  isLocal?: boolean;
}

export type CollabConnectionStatus = "disconnected" | "connecting" | "connected";

export interface CollabChatMessage {
  id: string;
  userId: string;
  userName: string;
  userColor: string;
  text: string;
  timestamp: number;
}

export type CollabClientMessage =
  | {
      type: "join";
      room: string;
      password?: string;
      user: { id: string; name: string; color: string };
      initialLayout?: Layout;
    }
  | { type: "layout_update"; layout: Layout }
  | { type: "cursor"; x: number; y: number }
  | { type: "selection"; selectedIds: string[] }
  | { type: "chat"; text: string };

export type CollabServerMessage =
  | {
      type: "init";
      room: string;
      user: CollabUser;
      users: CollabUser[];
      layout: Layout | null;
      hasPassword?: boolean;
      isHost?: boolean;
    }
  | { type: "user_joined"; user: CollabUser }
  | { type: "user_left"; userId: string; userName?: string }
  | { type: "layout_update"; layout: Layout; fromUserId: string; fromUserName: string }
  | { type: "cursor"; userId: string; x: number; y: number }
  | { type: "selection"; userId: string; selectedIds: string[] }
  | { type: "chat"; id: string; userId: string; userName: string; userColor: string; text: string; timestamp: number }
  | { type: "error"; message: string; code?: "INVALID_PASSWORD" | "AUTH_FAILED" | "ROOM_LOCKED" | "SERVER_ERROR" | string };

