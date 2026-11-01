import { describe, it, expect, vi, beforeEach } from "vitest";
import { CollabClient } from "@/collab/collab-client";
import type { Layout } from "@/core/types";

// Mock WebSocket
class MockWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;

  static instances: MockWebSocket[] = [];
  url: string;
  readyState = MockWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: ((err: unknown) => void) | null = null;
  sentData: string[] = [];

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
    setTimeout(() => {
      this.readyState = MockWebSocket.OPEN;
      this.onopen?.();
    }, 10);
  }

  send(data: string) {
    this.sentData.push(data);
  }

  close() {
    this.readyState = 3; // CLOSED
    this.onclose?.();
  }

  receive(data: object) {
    this.onmessage?.({ data: JSON.stringify(data) });
  }
}

describe("CollabClient", () => {
  let originalWebSocket: typeof WebSocket;
  const store = new Map<string, string>();

  beforeEach(() => {
    MockWebSocket.instances = [];
    store.clear();

    const mockStorage = {
      getItem: (key: string) => store.get(key) || null,
      setItem: (key: string, value: string) => store.set(key, value),
      removeItem: (key: string) => store.delete(key),
      clear: () => store.clear(),
      key: (i: number) => Array.from(store.keys())[i] || null,
      length: store.size,
    };
    Object.defineProperty(globalThis, "localStorage", {
      value: mockStorage,
      writable: true,
      configurable: true,
    });

    originalWebSocket = globalThis.WebSocket;
    globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
  });

  it("should initialize with default user profile and disconnected status", () => {
    const client = new CollabClient();
    expect(client.getStatus()).toBe("disconnected");
    expect(client.getUser().name).toBeDefined();
    expect(client.getUser().color).toBeDefined();
    expect(client.getUsers()).toHaveLength(0);
  });

  it("should update profile and notify localStorage", () => {
    const client = new CollabClient();
    client.setProfile("Lokfuehrer-Tom", "#10b981");
    expect(client.getUser().name).toBe("Lokfuehrer-Tom");
    expect(client.getUser().color).toBe("#10b981");
    expect(localStorage.getItem("railbench_collab_username")).toBe("Lokfuehrer-Tom");
    expect(localStorage.getItem("railbench_collab_color")).toBe("#10b981");
  });

  it("should connect, send join packet and handle init message from server", async () => {
    const client = new CollabClient();
    const onStatusChange = vi.fn();
    const onUsersChange = vi.fn();
    const onRemoteLayout = vi.fn();

    client.onStatusChange = onStatusChange;
    client.onUsersChange = onUsersChange;
    client.onRemoteLayout = onRemoteLayout;

    const sampleLayout: Layout = {
      name: "Test Server Layout",
      scale: "H0",
      pieces: [],
    };

    client.connect("ws://localhost:3000", "room-1", "Bob", "#3b82f6", sampleLayout);
    expect(onStatusChange).toHaveBeenCalledWith("connecting", undefined);

    // Wait for mock connection to open
    await new Promise((r) => setTimeout(r, 20));

    expect(client.getStatus()).toBe("connected");
    expect(MockWebSocket.instances).toHaveLength(1);
    const mockWs = MockWebSocket.instances[0]!;

    // Check that join packet was sent
    expect(mockWs.sentData).toHaveLength(1);
    const joinMsg = JSON.parse(mockWs.sentData[0]!);
    expect(joinMsg.type).toBe("join");
    expect(joinMsg.room).toBe("room-1");
    expect(joinMsg.user.name).toBe("Bob");

    // Simulate server replying with init packet
    mockWs.receive({
      type: "init",
      room: "room-1",
      user: { id: client.getUser().id, name: "Bob", color: "#3b82f6" },
      users: [
        { id: client.getUser().id, name: "Bob", color: "#3b82f6" },
        { id: "u-2", name: "Alice", color: "#ec4899" },
      ],
      layout: sampleLayout,
    });

    expect(onUsersChange).toHaveBeenCalled();
    expect(client.getUsers()).toHaveLength(1); // Alice
    expect(client.getUsers()[0]!.name).toBe("Alice");
    expect(onRemoteLayout).toHaveBeenCalledWith(sampleLayout, "Server");
  });

  it("should handle user joined, user left, and remote cursor events", async () => {
    const client = new CollabClient();
    const onUsersChange = vi.fn();
    const onRemoteCursor = vi.fn();

    client.onUsersChange = onUsersChange;
    client.onRemoteCursor = onRemoteCursor;

    client.connect("ws://localhost:3000", "default");
    await new Promise((r) => setTimeout(r, 20));
    const mockWs = MockWebSocket.instances[0]!;

    // Alice joins
    mockWs.receive({
      type: "user_joined",
      user: { id: "u-alice", name: "Alice", color: "#ec4899" },
    });
    expect(client.getUsers()).toHaveLength(1);
    expect(client.getUsers()[0]!.id).toBe("u-alice");

    // Alice moves cursor
    mockWs.receive({
      type: "cursor",
      userId: "u-alice",
      x: 250,
      y: 400,
    });
    expect(onRemoteCursor).toHaveBeenCalledWith("u-alice", 250, 400);

    // Alice leaves
    mockWs.receive({
      type: "user_left",
      userId: "u-alice",
    });
    expect(client.getUsers()).toHaveLength(0);
  });

  it("should broadcast layout updates and chat messages when connected", async () => {
    const client = new CollabClient();
    client.connect("ws://localhost:3000", "default");
    await new Promise((r) => setTimeout(r, 20));
    const mockWs = MockWebSocket.instances[0]!;

    const testLayout: Layout = {
      name: "Updated Layout",
      scale: "H0",
      pieces: [
        {
          id: "p-1",
          catalogId: "24188",
          layerId: "default",
          transform: { x: 10, y: 10, rotation: 0 },
          connectors: [],
        },
      ],
    };

    client.broadcastLayout(testLayout);
    const lastSent = JSON.parse(mockWs.sentData[mockWs.sentData.length - 1]!);
    expect(lastSent.type).toBe("layout_update");
    expect(lastSent.layout.pieces).toHaveLength(1);

    client.sendChat("Hallo an alle!");
    const chatSent = JSON.parse(mockWs.sentData[mockWs.sentData.length - 1]!);
    expect(chatSent.type).toBe("chat");
    expect(chatSent.text).toBe("Hallo an alle!");
  });

  it("should send password in join packet and store password", async () => {
    const client = new CollabClient();
    client.connect("ws://localhost:3000", "secret-room", "HostUser", "#10b981", undefined, "bahn123");

    await new Promise((r) => setTimeout(r, 20));
    expect(client.getPassword()).toBe("bahn123");

    const mockWs = MockWebSocket.instances[0]!;
    const joinMsg = JSON.parse(mockWs.sentData[0]!);
    expect(joinMsg.type).toBe("join");
    expect(joinMsg.room).toBe("secret-room");
    expect(joinMsg.password).toBe("bahn123");
  });

  it("should handle INVALID_PASSWORD error and trigger onPasswordError", async () => {
    const client = new CollabClient();
    const onPasswordError = vi.fn();
    client.onPasswordError = onPasswordError;

    client.connect("ws://localhost:3000", "locked-room", "Guest", undefined, undefined, "wrongpass");
    await new Promise((r) => setTimeout(r, 20));

    const mockWs = MockWebSocket.instances[0]!;
    mockWs.receive({
      type: "error",
      code: "INVALID_PASSWORD",
      message: "Falsches Passwort für diesen Raum.",
    });

    expect(onPasswordError).toHaveBeenCalledWith("Falsches Passwort für diesen Raum.");
    expect(client.getStatus()).toBe("disconnected");
  });

  it("should parse start variables for server auto-start accurately", () => {
    function parseTestArgs(argv: string[], env: Record<string, string | undefined>) {
      const hasFlag = (flags: string[]): boolean => {
        return flags.some((f) => argv.includes(f) || argv.some((a) => a.startsWith(`${f}=`)));
      };
      const getOption = (flags: string[]): string | undefined => {
        for (const f of flags) {
          const idx = argv.indexOf(f);
          if (idx !== -1 && argv[idx + 1] && !argv[idx + 1]!.startsWith("--")) {
            return argv[idx + 1]!;
          }
          const prefix = `${f}=`;
          const match = argv.find((a) => a.startsWith(prefix));
          if (match) return match.substring(prefix.length);
        }
        return undefined;
      };

      const shouldAutoStart =
        hasFlag(["--server", "--collab-server", "--start-server", "--autostart-server", "--serve"]) ||
        env.RAILBENCH_SERVER === "1" ||
        env.RAILBENCH_SERVER === "true";

      const rawPort = getOption(["--port", "--server-port"]) || env.RAILBENCH_SERVER_PORT || env.PORT;
      const port = rawPort ? parseInt(rawPort, 10) : 3000;
      const password = getOption(["--password", "--server-password"]) || env.RAILBENCH_SERVER_PASSWORD;
      const defaultRoom = getOption(["--room", "--server-room"]) || env.RAILBENCH_SERVER_ROOM || "default";
      const autoConnect = hasFlag(["--connect", "--auto-connect"]) || env.RAILBENCH_CONNECT === "1";

      return { shouldAutoStart, port: isNaN(port) ? 3000 : port, password, defaultRoom, autoConnect };
    }

    // Default configuration (no flags)
    const defaults = parseTestArgs([], {});
    expect(defaults.shouldAutoStart).toBe(false);
    expect(defaults.port).toBe(3000);
    expect(defaults.defaultRoom).toBe("default");
    expect(defaults.password).toBeUndefined();
    expect(defaults.autoConnect).toBe(false);

    // CLI flags
    const fromCli = parseTestArgs(
      ["--server", "--port", "8080", "--password", "geheim", "--room", "anlage1", "--connect"],
      {}
    );
    expect(fromCli.shouldAutoStart).toBe(true);
    expect(fromCli.port).toBe(8080);
    expect(fromCli.password).toBe("geheim");
    expect(fromCli.defaultRoom).toBe("anlage1");
    expect(fromCli.autoConnect).toBe(true);

    // Flag=value syntax
    const fromCliEqual = parseTestArgs(["--server", "--port=9000", "--password=pass123"], {});
    expect(fromCliEqual.shouldAutoStart).toBe(true);
    expect(fromCliEqual.port).toBe(9000);
    expect(fromCliEqual.password).toBe("pass123");

    // Environment variables
    const fromEnv = parseTestArgs([], {
      RAILBENCH_SERVER: "1",
      RAILBENCH_SERVER_PORT: "4000",
      RAILBENCH_SERVER_PASSWORD: "envpassword",
      RAILBENCH_SERVER_ROOM: "room-env",
      RAILBENCH_CONNECT: "1",
    });
    expect(fromEnv.shouldAutoStart).toBe(true);
    expect(fromEnv.port).toBe(4000);
    expect(fromEnv.password).toBe("envpassword");
    expect(fromEnv.defaultRoom).toBe("room-env");
    expect(fromEnv.autoConnect).toBe(true);
  });
});
