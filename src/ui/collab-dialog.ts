import type { CollabClient } from "@/collab/collab-client";
import type { Layout } from "@/core/types";
import { getTranslations } from "@/i18n";

const COLOR_PALETTE = [
  "#3b82f6",
  "#10b981",
  "#f59e0b",
  "#ec4899",
  "#8b5cf6",
  "#06b6d4",
  "#f97316",
  "#14b8a6",
];

export interface CollabDialogOptions {
  client: CollabClient;
  layout: Layout;
  onConnect: (serverUrl: string, room: string, name: string, color: string, password?: string) => void;
  onDisconnect: () => void;
  onStartServer?: (options: { port?: number; password?: string; defaultRoom?: string }) => Promise<{
    running: boolean;
    port?: number;
    localUrl?: string;
    networkUrls?: string[];
    wsUrl?: string;
    networkWsUrls?: string[];
    password?: string;
    defaultRoom?: string;
  } | null>;
  onStopServer?: () => Promise<unknown>;
  getServerStatus?: () => Promise<{
    running: boolean;
    port?: number;
    localUrl?: string;
    networkUrls?: string[];
    wsUrl?: string;
    networkWsUrls?: string[];
    password?: string;
    defaultRoom?: string;
  } | null>;
}

export function openCollabDialog(options: CollabDialogOptions): void {
  const { client, layout, onConnect, onDisconnect, onStartServer, onStopServer } = options;
  const t = getTranslations().collabDialog;

  const existing = document.getElementById("collab-modal");
  if (existing) existing.remove();

  let isConnected = client.getStatus() === "connected";
  const user = client.getUser();
  let currentRoom = client.getRoom() || "default";

  const hasStorage = typeof localStorage !== "undefined";
  let savedPassword = client.getPassword() || (hasStorage ? localStorage.getItem("railbench_collab_password") || "" : "");
  let activeTab: "host" | "join" = "host";

  // Determine default WebSocket URL
  const defaultWsProtocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const defaultWsHost = window.location.host || "localhost:3000";
  const defaultServerUrl = client.getServerUrl() || `${defaultWsProtocol}//${defaultWsHost}`;

  let selectedColor = user.color || COLOR_PALETTE[0]!;
  let serverUrlsInfo: { localUrl?: string; networkUrls?: string[]; wsUrl?: string } | null = null;
  let errorMessage: string | null = null;

  const modal = document.createElement("div");
  modal.id = "collab-modal";
  modal.className = "rb-modal-overlay";

  function renderDialog() {
    isConnected = client.getStatus() === "connected";

    modal.innerHTML = `
      <div class="rb-modal-card collab-dialog-card">
        <div class="rb-modal-header" style="display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--border-main);">
          <div class="rb-modal-title" style="display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px;">
            <span>👥</span>
            <span>${t.title}</span>
            ${isConnected ? `<span class="collab-badge-live">● ${t.statusConnected}</span>` : ""}
          </div>
          <button class="rb-modal-close-btn" id="collab-modal-close" title="✕" style="background: none; border: none; font-size: 16px; cursor: pointer; color: var(--text-muted);">✕</button>
        </div>

        <div class="collab-dialog-body" style="padding: 16px 18px; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 14px;">
          
          <!-- ERROR BANNER IF ANY -->
          ${
            errorMessage
              ? `<div style="background: rgba(239, 68, 68, 0.12); border: 1px solid #ef4444; color: #ef4444; padding: 10px 14px; border-radius: 8px; font-size: 12.5px; font-weight: 600; display: flex; align-items: center; gap: 8px;">
                  <span>⚠️</span>
                  <span>${errorMessage}</span>
                </div>`
              : ""
          }

          <!-- 1. TOGGLE HERO: EIN / AUS SCHALTER MIT PASSWORT -->
          <div class="collab-toggle-hero">
            <div class="collab-toggle-info">
              <div class="collab-hero-title">
                <span>🌐</span>
                <span>${t.serverToggleLabel}</span>
              </div>
              <div class="collab-hero-sub">
                ${t.serverToggleSub}
              </div>
            </div>
            <div class="collab-toggle-wrapper">
              <label class="rb-switch" title="${t.serverToggleLabel}">
                <input type="checkbox" id="collab-server-toggle" ${isConnected ? "checked" : ""} />
                <span class="rb-switch-slider"></span>
              </label>
              <span id="collab-server-toggle-label" class="collab-switch-label" style="color: ${isConnected ? "#10b981" : "var(--text-muted)"};">
                ${isConnected ? t.serverToggleOn : t.serverToggleOff}
              </span>
            </div>
          </div>

          <!-- TAB SWITCHER: HOST (EIGENER SERVER) vs BEITRETEN (CLIENT) -->
          <div class="collab-tabs">
            <button type="button" class="collab-tab-btn ${activeTab === "host" ? "active" : ""}" id="collab-tab-host">
              <span>🚀</span> <span>${t.modeHost}</span>
            </button>
            <button type="button" class="collab-tab-btn ${activeTab === "join" ? "active" : ""}" id="collab-tab-join">
              <span>🔗</span> <span>${t.modeJoin}</span>
            </button>
          </div>

          ${
            activeTab === "host"
              ? `
            <!-- HOST SETTINGS -->
            <div style="display: flex; flex-direction: column; gap: 12px; background: var(--bg-surface); border: 1px solid var(--border-main); border-radius: 8px; padding: 12px 14px;">
              <!-- PASSWORD FIELD -->
              <div>
                <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;">
                  <label for="collab-input-password" style="font-size: 12px; font-weight: 700; color: var(--text-main);">
                    ${t.passwordLabel}
                  </label>
                  <span style="font-size: 10.5px; color: var(--text-muted);">${t.passwordHint}</span>
                </div>
                <div class="collab-password-box">
                  <input type="password" id="collab-input-password" class="rb-input" value="${escapeHtml(savedPassword)}" placeholder="${t.passwordPlaceholder}" style="font-size: 13px;" />
                  <button type="button" id="collab-btn-toggle-pwd" class="collab-pwd-eye-btn" title="Passwort anzeigen / verbergen">👁️</button>
                </div>
              </div>

              <!-- ROOM NAME & USER NAME -->
              <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                <div>
                  <label style="font-size: 12px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.roomLabel}</label>
                  <input type="text" id="collab-input-room" class="rb-input" value="${escapeHtml(currentRoom)}" placeholder="default" style="font-size: 13px;" />
                </div>
                <div>
                  <label style="font-size: 12px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.nameLabel}</label>
                  <input type="text" id="collab-input-name" class="rb-input" value="${escapeHtml(user.name)}" placeholder="Planer" style="font-size: 13px;" />
                </div>
              </div>

              <!-- COLOR PICKER -->
              <div>
                <label style="font-size: 11.5px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.colorLabel}</label>
                <div id="collab-color-picker" style="display: flex; gap: 8px; align-items: center;">
                  ${COLOR_PALETTE.map(
                    (c) => `
                    <button type="button" class="collab-color-btn" data-color="${c}" style="width: 22px; height: 22px; border-radius: 50%; background: ${c}; border: 2px solid ${c === selectedColor ? "var(--text-main)" : "transparent"}; cursor: pointer; transform: ${c === selectedColor ? "scale(1.15)" : "scale(1)"}; transition: transform 0.1s;"></button>
                  `,
                  ).join("")}
                </div>
              </div>
            </div>

            <!-- NETWORK SHARE & INVITE CARD (IF CONNECTED / HOSTING) -->
            ${
              isConnected
                ? `
              <div class="collab-share-card">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                  <span style="font-weight: 700; font-size: 12px; color: var(--text-main);">
                    <span>🔗</span> <span>${t.networkShareTitle}</span>
                  </span>
                  <button id="collab-btn-copy-link" class="rb-btn rb-btn-primary" style="font-size: 11.5px; padding: 4px 10px; display: flex; align-items: center; gap: 5px;">
                    <span>📋</span>
                    <span>${t.copyLinkBtn}</span>
                  </button>
                </div>
                <div id="collab-url-display" class="collab-url-chip">
                  <span id="collab-text-share-url" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${getShareUrl()}</span>
                  ${savedPassword ? `<span style="font-size: 10px; background: rgba(59, 130, 246, 0.15); color: #3b82f6; padding: 2px 6px; border-radius: 4px; font-weight: 700; margin-left: 8px;">🔒 PW aktiv</span>` : ""}
                </div>
              </div>
            `
                : ""
            }
          `
              : `
            <!-- JOIN EXTERNAL SERVER FORM -->
            <div style="display: flex; flex-direction: column; gap: 12px; background: var(--bg-surface); border: 1px solid var(--border-main); border-radius: 8px; padding: 12px 14px;">
              <div>
                <label style="font-size: 12px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.serverUrlLabel}</label>
                <input type="text" id="collab-input-url" class="rb-input" value="${escapeHtml(defaultServerUrl)}" placeholder="ws://192.168.1.50:3000" style="font-size: 13px;" />
              </div>

              <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
                <div>
                  <label style="font-size: 12px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.roomLabel}</label>
                  <input type="text" id="collab-join-room" class="rb-input" value="${escapeHtml(currentRoom)}" placeholder="default" style="font-size: 13px;" />
                </div>
                <div>
                  <label style="font-size: 12px; font-weight: 700; color: var(--text-sub); display: block; margin-bottom: 4px;">${t.nameLabel}</label>
                  <input type="text" id="collab-join-name" class="rb-input" value="${escapeHtml(user.name)}" placeholder="Planer" style="font-size: 13px;" />
                </div>
              </div>

              <div>
                <label for="collab-join-password" style="font-size: 12px; font-weight: 700; color: var(--text-main); display: block; margin-bottom: 4px;">
                  ${t.passwordLabel}
                </label>
                <div class="collab-password-box">
                  <input type="password" id="collab-join-password" class="rb-input" value="${escapeHtml(savedPassword)}" placeholder="${t.passwordPlaceholder}" style="font-size: 13px;" />
                  <button type="button" id="collab-btn-toggle-pwd-join" class="collab-pwd-eye-btn" title="Passwort anzeigen / verbergen">👁️</button>
                </div>
              </div>

              <button id="collab-btn-connect-join" style="margin-top: 4px; background: var(--primary-color); color: white; border: none; padding: 9px; border-radius: 6px; font-weight: 700; font-size: 13px; cursor: pointer; transition: background 0.15s;">
                ${t.connectBtn}
              </button>
            </div>
          `
          }

          <!-- ACTIVE PARTICIPANTS & CHAT (ONLY WHEN CONNECTED) -->
          ${
            isConnected
              ? `
            <div>
              <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;">
                <span style="font-weight: 700; font-size: 12px; color: var(--text-main);">${t.participantsTitle}</span>
              </div>
              <div id="collab-participants-list" style="display: flex; flex-direction: column; gap: 4px; max-height: 110px; overflow-y: auto;">
                <!-- User items rendered dynamically -->
              </div>
            </div>

            <!-- MINI CHAT LOG -->
            <div style="display: flex; flex-direction: column; gap: 6px; flex: 1;">
              <span style="font-weight: 700; font-size: 12px; color: var(--text-main);">${t.chatTitle}</span>
              <div id="collab-chat-log" style="background: var(--bg-surface); border: 1px solid var(--border-main); border-radius: 6px; height: 130px; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 5px; font-size: 11.5px;">
                <!-- Chat messages rendered dynamically -->
              </div>
              <form id="collab-chat-form" style="display: flex; gap: 6px;">
                <input type="text" id="collab-chat-input" class="rb-input" placeholder="${t.chatPlaceholder}" style="flex: 1; padding: 5px 8px; border-radius: 6px; font-size: 11.5px;" />
                <button type="submit" style="background: var(--primary-color); color: white; border: none; padding: 5px 12px; border-radius: 6px; font-weight: 600; font-size: 11.5px; cursor: pointer;">${t.chatSendBtn}</button>
              </form>
            </div>
          `
              : ""
          }

        </div>
      </div>
    `;

    wireEvents();
  }

  function getShareUrl(): string {
    const hostBase = serverUrlsInfo?.networkUrls?.[0] || serverUrlsInfo?.localUrl || window.location.origin;
    let url = `${hostBase}?collab=1&room=${encodeURIComponent(currentRoom)}`;
    if (savedPassword) {
      url += `&pass=${encodeURIComponent(savedPassword)}`;
    }
    return url;
  }

  function wireEvents() {
    // Close handlers
    const closeBtn = modal.querySelector("#collab-modal-close");
    closeBtn?.addEventListener("click", () => modal.remove());
    modal.addEventListener("click", (e) => {
      if (e.target === modal) modal.remove();
    });

    // Tab buttons
    const tabHost = modal.querySelector("#collab-tab-host");
    const tabJoin = modal.querySelector("#collab-tab-join");
    tabHost?.addEventListener("click", () => {
      activeTab = "host";
      renderDialog();
    });
    tabJoin?.addEventListener("click", () => {
      activeTab = "join";
      renderDialog();
    });

    // Color picker
    const colorBtns = modal.querySelectorAll<HTMLButtonElement>(".collab-color-btn");
    colorBtns.forEach((btn) => {
      btn.addEventListener("click", () => {
        selectedColor = btn.dataset.color || selectedColor;
        colorBtns.forEach((b) => {
          const isSel = b.dataset.color === selectedColor;
          b.style.borderColor = isSel ? "var(--text-main)" : "transparent";
          b.style.transform = isSel ? "scale(1.15)" : "scale(1)";
        });
      });
    });

    // Password visibility toggle (host tab)
    const pwdInput = modal.querySelector<HTMLInputElement>("#collab-input-password");
    const togglePwdBtn = modal.querySelector<HTMLButtonElement>("#collab-btn-toggle-pwd");
    togglePwdBtn?.addEventListener("click", () => {
      if (!pwdInput) return;
      if (pwdInput.type === "password") {
        pwdInput.type = "text";
        togglePwdBtn.textContent = "🙈";
      } else {
        pwdInput.type = "password";
        togglePwdBtn.textContent = "👁️";
      }
    });

    if (pwdInput) {
      pwdInput.addEventListener("input", () => {
        savedPassword = pwdInput.value.trim();
        if (hasStorage) {
          localStorage.setItem("railbench_collab_password", savedPassword);
        }
        client.setPassword(savedPassword);
        const shareChip = modal.querySelector("#collab-text-share-url");
        if (shareChip) shareChip.textContent = getShareUrl();
      });
    }

    // Password visibility toggle (join tab)
    const pwdJoinInput = modal.querySelector<HTMLInputElement>("#collab-join-password");
    const togglePwdJoinBtn = modal.querySelector<HTMLButtonElement>("#collab-btn-toggle-pwd-join");
    togglePwdJoinBtn?.addEventListener("click", () => {
      if (!pwdJoinInput) return;
      if (pwdJoinInput.type === "password") {
        pwdJoinInput.type = "text";
        togglePwdJoinBtn.textContent = "🙈";
      } else {
        pwdJoinInput.type = "password";
        togglePwdJoinBtn.textContent = "👁️";
      }
    });

    if (pwdJoinInput) {
      pwdJoinInput.addEventListener("input", () => {
        savedPassword = pwdJoinInput.value.trim();
        if (hasStorage) {
          localStorage.setItem("railbench_collab_password", savedPassword);
        }
      });
    }

    // Server Toggle Switch [ EIN / AUS ]
    const serverToggle = modal.querySelector<HTMLInputElement>("#collab-server-toggle");
    serverToggle?.addEventListener("change", async () => {
      errorMessage = null;

      if (serverToggle.checked) {
        // TURN ON: Start server & connect
        const inputName = (modal.querySelector("#collab-input-name") as HTMLInputElement)?.value || user.name;
        const inputRoom = (modal.querySelector("#collab-input-room") as HTMLInputElement)?.value || currentRoom || "default";
        const inputPwd = pwdInput ? pwdInput.value.trim() : savedPassword;
        savedPassword = inputPwd;
        currentRoom = inputRoom;

        if (hasStorage) {
          localStorage.setItem("railbench_collab_password", savedPassword);
        }

        let targetWsUrl = defaultServerUrl;

        // In Electron, start embedded server directly without console command!
        if (onStartServer) {
          try {
            const serverResult = await onStartServer({ password: savedPassword, defaultRoom: currentRoom });
            if (serverResult) {
              serverUrlsInfo = serverResult;
              targetWsUrl = serverResult.wsUrl || targetWsUrl;
            }
          } catch (err) {
            console.warn("Collab: embedded server start error:", err);
          }
        }

        // Connect client
        onConnect(targetWsUrl, currentRoom, inputName, selectedColor, savedPassword);
      } else {
        // TURN OFF: Disconnect & stop server
        onDisconnect();
        if (onStopServer) {
          try {
            await onStopServer();
          } catch {}
        }
        serverUrlsInfo = null;
        renderDialog();
      }
    });

    // Join Button (Join tab)
    const btnConnectJoin = modal.querySelector<HTMLButtonElement>("#collab-btn-connect-join");
    btnConnectJoin?.addEventListener("click", () => {
      errorMessage = null;
      const inputUrl = (modal.querySelector("#collab-input-url") as HTMLInputElement)?.value || defaultServerUrl;
      const inputRoom = (modal.querySelector("#collab-join-room") as HTMLInputElement)?.value || "default";
      const inputName = (modal.querySelector("#collab-join-name") as HTMLInputElement)?.value || user.name;
      const inputPwd = (modal.querySelector("#collab-join-password") as HTMLInputElement)?.value || "";

      savedPassword = inputPwd.trim();
      currentRoom = inputRoom.trim();
      if (hasStorage) {
        localStorage.setItem("railbench_collab_password", savedPassword);
      }

      onConnect(inputUrl, currentRoom, inputName, selectedColor, savedPassword);
    });

    // Copy Invite Link Button
    const btnCopyLink = modal.querySelector<HTMLButtonElement>("#collab-btn-copy-link");
    btnCopyLink?.addEventListener("click", () => {
      const inviteUrl = getShareUrl();
      navigator.clipboard.writeText(inviteUrl).then(() => {
        const span = btnCopyLink.querySelector("span:last-child");
        if (span) span.textContent = savedPassword ? t.linkCopiedWithPass : t.linkCopied;
        setTimeout(() => {
          if (span) span.textContent = t.copyLinkBtn;
        }, 2200);
      });
    });

    // Render participants & chat if connected
    if (isConnected) {
      renderParticipants();
      renderChat();

      const chatForm = modal.querySelector<HTMLFormElement>("#collab-chat-form");
      const chatInput = modal.querySelector<HTMLInputElement>("#collab-chat-input");
      chatForm?.addEventListener("submit", (e) => {
        e.preventDefault();
        if (!chatInput) return;
        const text = chatInput.value.trim();
        if (text) {
          client.sendChat(text);
          chatInput.value = "";
          renderChat();
        }
      });
    }
  }

  function renderParticipants() {
    const list = modal.querySelector("#collab-participants-list");
    if (!list) return;

    const allUsers = [client.getUser(), ...client.getUsers()];
    list.innerHTML = allUsers
      .map(
        (u) => `
        <div style="display: flex; align-items: center; justify-content: space-between; padding: 5px 8px; background: var(--bg-card); border: 1px solid var(--border-main); border-radius: 6px; font-size: 11.5px;">
          <div style="display: flex; align-items: center; gap: 7px;">
            <span style="display: inline-block; width: 9px; height: 9px; border-radius: 50%; background: ${u.color};"></span>
            <span style="font-weight: 600; color: var(--text-main);">${escapeHtml(u.name)}</span>
            ${u.isLocal ? `<span style="font-size: 9.5px; background: var(--bg-surface); padding: 1px 4px; border-radius: 3px; color: var(--text-muted);">(Du)</span>` : ""}
          </div>
          <span style="font-size: 10.5px; color: #10b981; font-weight: 600;">● Online</span>
        </div>
      `,
      )
      .join("");
  }

  function renderChat() {
    const log = modal.querySelector("#collab-chat-log");
    if (!log) return;

    const messages = client.getChatMessages();
    log.innerHTML = messages
      .map(
        (m) => `
        <div>
          <span style="font-weight: 700; color: ${m.userColor};">${escapeHtml(m.userName)}:</span>
          <span style="color: var(--text-main); margin-left: 4px;">${escapeHtml(m.text)}</span>
        </div>
      `,
      )
      .join("");
    log.scrollTop = log.scrollHeight;
  }

  // Hook into client callbacks while modal is open
  const origStatusChange = client.onStatusChange;
  const origUsersChange = client.onUsersChange;
  const origChat = client.onChat;
  const origPasswordError = client.onPasswordError;

  client.onStatusChange = (status, error) => {
    origStatusChange?.(status, error);
    if (error) {
      errorMessage = error;
    }
    renderDialog();
  };

  client.onUsersChange = (users) => {
    origUsersChange?.(users);
    renderParticipants();
  };

  client.onChat = (msg) => {
    origChat?.(msg);
    renderChat();
  };

  client.onPasswordError = (msg) => {
    origPasswordError?.(msg);
    errorMessage = `${t.invalidPasswordAlert} (${msg})`;
    renderDialog();
  };

  document.body.appendChild(modal);
  renderDialog();
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
