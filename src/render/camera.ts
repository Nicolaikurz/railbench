import type { Container } from "pixi.js";

/** Pan (Mittelklick) und Zoom zum Cursor. */
export function attachCamera(canvas: HTMLCanvasElement, world: Container, onChange?: () => void) {
  let panning = false;
  let last = { x: 0, y: 0 };

  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const wx = (mx - world.x) / world.scale.x;
    const wy = (my - world.y) / world.scale.y;
    world.scale.set(world.scale.x * factor);
    world.x = mx - wx * world.scale.x;
    world.y = my - wy * world.scale.y;
    onChange?.();
  }, { passive: false });

  canvas.addEventListener("pointerdown", (e) => {
    if (e.button === 1 || e.button === 2) {
      panning = true;
      last = { x: e.clientX, y: e.clientY };
      canvas.style.cursor = "grabbing";
    }
  });
  window.addEventListener("pointerup", () => {
    if (panning) {
      panning = false;
      canvas.style.cursor = "default";
    }
  });
  window.addEventListener("pointermove", (e) => {
    if (!panning) return;
    world.x += e.clientX - last.x;
    world.y += e.clientY - last.y;
    last = { x: e.clientX, y: e.clientY };
    onChange?.();
  });
}
