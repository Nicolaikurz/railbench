export function showToast(msg: string, icon = "ℹ️") {
  const existing = document.getElementById("app-global-toast");
  if (existing) existing.remove();
  const toast = document.createElement("div");
  toast.id = "app-global-toast";
  toast.className = "app-global-toast";
  toast.style.position = "fixed";
  toast.style.bottom = "64px";
  toast.style.left = "50%";
  toast.style.transform = "translateX(-50%)";
  toast.style.background = "var(--bg-toolbar, #1e293b)";
  toast.style.color = "var(--toolbar-text, #f8fafc)";
  toast.style.border = "1px solid var(--border-main, #334155)";
  toast.style.borderRadius = "20px";
  toast.style.padding = "7px 18px";
  toast.style.fontSize = "13px";
  toast.style.fontWeight = "600";
  toast.style.boxShadow = "0 8px 24px rgba(0,0,0,0.35)";
  toast.style.zIndex = "999";
  toast.style.pointerEvents = "none";
  toast.style.display = "flex";
  toast.style.alignItems = "center";
  toast.style.gap = "8px";
  toast.style.transition = "opacity 0.2s ease, transform 0.2s ease";
  toast.innerHTML = `<span>${icon}</span> <span>${msg}</span>`;

  document.body.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transform = "translateX(-50%) translateY(6px)";
    setTimeout(() => toast.remove(), 250);
  }, 2200);
}

export function showCollabToast(msg: string) {
  showToast(msg, "👥");
}
