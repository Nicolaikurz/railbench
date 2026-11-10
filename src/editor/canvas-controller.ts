import type { Layout, TrackPiece, Model3D, Transform, ColorMode } from "@/core/types";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { computeWorldConnectors } from "@/catalog/catalog";
import { findBestSnapForPiece, type BestPieceSnap } from "@/core/snapping";
import { snapBaseboard, type BaseboardSnapGuide } from "@/core/baseboard-snapping";
import type { BaseboardConfig } from "@/core/types";
import {
  getBaseboards,
  getBaseboardBounds,
  isPointInsideBaseboard,
  isPointNearBaseboardEdge,
  isPointInBaseboardHeader,
} from "@/core/layout";
import {
  isPointInsideModel,
  getModelRotationHandleWorld,
} from "@/core/models-3d";
import {
  dist,
  normAngle,
  rad2deg,
  rotate,
  distanceToTrackPiece,
  getPieceCenterWorld,
  getPieceRotationHandleWorld,
  isPieceInSelectionBox,
  computeGroupCentroid,
  rotateGroupAroundCentroid,
  type RotatableItem,
} from "@/core/geometry";
import {
  MovePieceCommand,
  MoveModelCommand,
  MoveBaseboardCommand,
  type History,
} from "@/editor/commands";
import type { Selection } from "@/editor/selection";
import type { Renderer } from "@/render/renderer";
import type { CollabClient } from "@/collab/collab-client";
import type { Sidebar } from "@/ui/sidebar";
import type { PerfMonitor } from "@/ui/perf-monitor";
import type { Ruler } from "@/ui/ruler";
import type { PlanManager } from "@/io/plan-manager";
import type { TrackOperations } from "./track-operations";
import { handleCanvasDrop } from "./canvas-drop-handler";
import { KeyboardController, type KeyboardControllerDeps } from "./keyboard-controller";

export interface CanvasControllerDeps {
  canvas: HTMLCanvasElement;
  renderer: Renderer;
  getLayout: () => Layout;
  getCatalog: () => Catalog;
  getSelection: () => Selection;
  getHistory: () => History;
  getOperations: () => TrackOperations;
  getCollabClient: () => CollabClient;
  getSidebar: () => Sidebar | null;
  getPerfMonitor: () => PerfMonitor;
  getRuler: () => Ruler;
  getPlanManager: () => PlanManager;
  is3DMode: () => boolean;
  toggle3D: (force?: boolean) => void;
  isMeasureToolActive: () => boolean;
  toggleMeasureTool: (force?: boolean) => void;
  findMeasureSnapPoint: (pos: { x: number; y: number }, snapDist?: number, forceNoSnap?: boolean) => { x: number; y: number } | null;
  updateMeasureHud: (updateInputs?: boolean) => void;
  getMeasureState: () => {
    start: { x: number; y: number } | null;
    current: { x: number; y: number } | null;
    locked: boolean;
    snapPoint: { x: number; y: number } | null;
    snapEnabled: boolean;
    draggingPoint: "A" | "B" | null;
  };
  setMeasureState: (update: Partial<{
    start: { x: number; y: number } | null;
    current: { x: number; y: number } | null;
    locked: boolean;
    snapPoint: { x: number; y: number } | null;
    draggingPoint: "A" | "B" | null;
  }>) => void;
  isSnappingEnabled: () => boolean;
  setSnappingEnabled: (val: boolean) => void;
  toggleSelectionLock?: () => void;
  getColorMode: () => ColorMode;
  cycleColorMode: (target?: ColorMode) => void;
  isTrackLabelsEnabled: () => boolean;
  setTrackLabelsEnabled: (val: boolean) => void;
  isPowerSimEnabled: () => boolean;
  setPowerSimEnabled: (val: boolean) => void;
  toggleRightSidebar: () => void;
  openHeightDialogForSelection: () => void;
  openHelpDialog: (tab?: any) => void;
  openPrintDialog: () => void;
  isPieceInteractive: (piece: TrackPiece) => boolean;
  isModelInteractive: (model: Model3D) => boolean;
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined;
  sync: (dirty?: boolean) => void;
  saveSettingsState: () => void;
  showToast: (msg: string, icon?: string) => void;
  getCurrentSimulation: () => any;
  setSelectionBox: (box: { start: { x: number; y: number }; end: { x: number; y: number } } | null) => void;
  setDraggingBaseboardState: (isDragging: boolean, activeBoard: BaseboardConfig | null, guides: BaseboardSnapGuide[]) => void;
}

export class CanvasController {
  private deps: CanvasControllerDeps;

  // Drag & Rotate State
  private isDraggingPiece = false;
  private isRotatingPiece = false;
  private isRotatingModel = false;
  private isBoxSelecting = false;
  private boxStartWorld = { x: 0, y: 0 };
  private initialBoxSelection = new Set<string>();
  private activePiece: TrackPiece | null = null;
  private activeModel: Model3D | null = null;
  private dragStartWorld = { x: 0, y: 0 };
  private originalTransform: Transform = { x: 0, y: 0, rotation: 0 };
  private draggedPiecesMap = new Map<string, Transform>();
  private draggedInitialConnections: { pieceId: string; connectorIndex: number }[] = [];
  private pieceCenterWorld = { x: 0, y: 0 };
  private pendingSingleSelect: string | null = null;
  private hasDragMoved = false;

  // Baseboard Drag State
  private isDraggingBaseboard = false;
  private activeBaseboard: BaseboardConfig | null = null;
  private baseboardDragStartWorld = { x: 0, y: 0 };
  private baseboardOriginalPos = { positionMode: "custom" as const, customX: 0, customY: 0 };
  private activeBaseboardSnapGuides: BaseboardSnapGuide[] = [];
  private baseboardInitialTrackTransforms = new Map<string, Transform>();

  // Event handler references for cleanup
  private handleDragOver: (e: DragEvent) => void;
  private handleDrop: (e: DragEvent) => Promise<void>;
  private handleContextMenu: (e: MouseEvent) => void;
  private handlePointerDown: (e: PointerEvent) => void;
  private handlePointerMove: (e: PointerEvent) => void;
  private handlePointerUp: (e: PointerEvent) => void;
  private handleWheel: (e: WheelEvent) => void;
  private keyboardController: KeyboardController;

  constructor(deps: CanvasControllerDeps) {
    this.deps = deps;

    this.handleDragOver = this.onDragOver.bind(this);
    this.handleDrop = this.onDrop.bind(this);
    this.handleContextMenu = (e) => e.preventDefault();
    this.handlePointerDown = this.onPointerDown.bind(this);
    this.handlePointerMove = this.onPointerMove.bind(this);
    this.handlePointerUp = this.onPointerUp.bind(this);
    this.handleWheel = this.onWheel.bind(this);
    this.keyboardController = new KeyboardController(deps);

    this.attach();
  }

  private attach(): void {
    const { canvas } = this.deps;
    canvas.addEventListener("dragover", this.handleDragOver);
    canvas.addEventListener("drop", this.handleDrop);
    canvas.addEventListener("contextmenu", this.handleContextMenu);
    canvas.addEventListener("pointerdown", this.handlePointerDown);
    window.addEventListener("pointermove", this.handlePointerMove);
    window.addEventListener("pointerup", this.handlePointerUp);
    canvas.addEventListener("wheel", this.handleWheel, { passive: false });
  }

  public destroy(): void {
    const { canvas } = this.deps;
    canvas.removeEventListener("dragover", this.handleDragOver);
    canvas.removeEventListener("drop", this.handleDrop);
    canvas.removeEventListener("contextmenu", this.handleContextMenu);
    canvas.removeEventListener("pointerdown", this.handlePointerDown);
    window.removeEventListener("pointermove", this.handlePointerMove);
    window.removeEventListener("pointerup", this.handlePointerUp);
    canvas.removeEventListener("wheel", this.handleWheel);
    this.keyboardController.destroy();
  }

  private onDragOver(e: DragEvent): void {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
  }

  private async onDrop(e: DragEvent): Promise<void> {
    await handleCanvasDrop(e, this.deps.renderer, this.deps.getOperations(), this.deps.sync);
  }

  private onPointerDown(e: PointerEvent): void {
    const { renderer, canvas } = this.deps;
    const layout = this.deps.getLayout();
    const selection = this.deps.getSelection();

    if (this.deps.isMeasureToolActive()) {
      const mState = this.deps.getMeasureState();
      if (e.button === 0) {
        const wPos = renderer.screenToWorld(e.clientX, e.clientY);
        const forceNoSnap = e.altKey;

        if (mState.start && Math.hypot(wPos.x - mState.start.x, wPos.y - mState.start.y) <= 15) {
          this.deps.setMeasureState({ draggingPoint: "A" });
          canvas.style.cursor = "grabbing";
          return;
        }
        if (mState.current && Math.hypot(wPos.x - mState.current.x, wPos.y - mState.current.y) <= 15) {
          this.deps.setMeasureState({ draggingPoint: "B" });
          canvas.style.cursor = "grabbing";
          return;
        }

        const snap = this.deps.findMeasureSnapPoint(wPos, 20, forceNoSnap);
        const pt = snap ? { ...snap } : { ...wPos };

        if (!mState.start || mState.locked) {
          this.deps.setMeasureState({ start: pt, current: pt, locked: false });
        } else {
          this.deps.setMeasureState({ current: pt, locked: true });
        }
        this.deps.updateMeasureHud();
        this.deps.sync();
      } else if (e.button === 2) {
        if (mState.start) {
          this.deps.setMeasureState({ start: null, current: null, locked: false, draggingPoint: null });
          this.deps.updateMeasureHud();
          this.deps.sync();
        } else {
          this.deps.toggleMeasureTool(false);
        }
      }
      return;
    }

    if (e.button !== 0) return;

    const wPos = renderer.screenToWorld(e.clientX, e.clientY);
    const isMultiSelect = e.ctrlKey || e.metaKey || e.shiftKey;
    this.hasDragMoved = false;
    this.pendingSingleSelect = null;

    const selectedIds = selection.all();
    const scale = renderer.world.scale.x || 1;
    const handleHitRadius = Math.max(24, 20 / scale);

    if (selectedIds.length > 0 && !isMultiSelect) {
      let clickedHandle = false;
      for (const selId of selectedIds) {
        const selPiece = layout.pieces.find((p) => p.id === selId);
        if (selPiece && this.deps.isPieceInteractive(selPiece)) {
          const entry = this.deps.getCatalogEntryForPiece(selPiece);
          if (entry) {
            const handlePos = getPieceRotationHandleWorld(entry, selPiece.transform);
            if (dist(wPos, handlePos) <= handleHitRadius) {
              clickedHandle = true;
              break;
            }
          }
        }
        const selModel = layout.models?.find((m) => m.id === selId);
        if (selModel && this.deps.isModelInteractive(selModel)) {
          const handlePos = getModelRotationHandleWorld(selModel);
          if (dist(wPos, handlePos) <= handleHitRadius) {
            clickedHandle = true;
            break;
          }
        }
      }

      if (clickedHandle) {
        this.isRotatingPiece = true;
        this.isRotatingModel = false;
        this.activePiece = null;
        this.activeModel = null;
        this.dragStartWorld = { ...wPos };

        this.draggedPiecesMap.clear();
        const rotatableItems: RotatableItem[] = [];
        for (const id of selectedIds) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p && this.deps.isPieceInteractive(p)) {
            this.draggedPiecesMap.set(p.id, { ...p.transform });
            const pEntry = this.deps.getCatalogEntryForPiece(p);
            rotatableItems.push({ id: p.id, type: "piece", transform: { ...p.transform }, entry: pEntry });
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m && this.deps.isModelInteractive(m)) {
            this.draggedPiecesMap.set(m.id, { ...m.transform });
            rotatableItems.push({ id: m.id, type: "model", transform: { ...m.transform } });
          }
        }

        this.pieceCenterWorld = computeGroupCentroid(rotatableItems);
        canvas.style.cursor = "grabbing";
        return;
      }
    }

    let hitModel: Model3D | null = null;
    if (layout.models) {
      for (const m of layout.models) {
        if (!this.deps.isModelInteractive(m)) continue;
        if (isPointInsideModel(wPos, m)) {
          hitModel = m;
          break;
        }
      }
    }

    if (hitModel) {
      if (isMultiSelect) {
        selection.toggle(hitModel.id);
        if (!selection.has(hitModel.id)) {
          this.deps.sync();
          return;
        }
      } else {
        if (selection.has(hitModel.id) && selection.all().length > 1) {
          this.pendingSingleSelect = hitModel.id;
        } else {
          selection.set(hitModel.id);
        }
      }
      this.isDraggingPiece = true;
      this.activePiece = null;
      this.activeModel = hitModel;
      this.dragStartWorld = { ...wPos };
      this.originalTransform = { ...hitModel.transform };

      this.draggedPiecesMap.clear();
      for (const id of selection.all()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && this.deps.isPieceInteractive(p)) this.draggedPiecesMap.set(p.id, { ...p.transform });
        const m = layout.models?.find((x) => x.id === id);
        if (m && this.deps.isModelInteractive(m)) this.draggedPiecesMap.set(m.id, { ...m.transform });
      }

      this.draggedInitialConnections = [];
      for (const [id] of this.draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          for (const c of p.connectors) {
            if (c.connectedTo && !this.draggedPiecesMap.has(c.connectedTo.pieceId)) {
              this.draggedInitialConnections.push({ pieceId: c.connectedTo.pieceId, connectorIndex: c.connectedTo.connectorIndex });
            }
          }
        }
      }

      canvas.style.cursor = "move";
      this.deps.sync();
      return;
    }

    let hit: TrackPiece | null = null;
    const catalog = this.deps.getCatalog();
    const hitGauge = catalog.gauge ?? 16.5;
    let minD = Infinity;

    for (const piece of layout.pieces) {
      if (!this.deps.isPieceInteractive(piece)) continue;
      const entry = this.deps.getCatalogEntryForPiece(piece);
      if (!entry) continue;

      const screenGrabBuffer = 20 / scale;
      const maxAllowedDist = entry.isRoad
        ? Math.max(35, (entry.roadWidth ?? 60) * 0.65, screenGrabBuffer)
        : Math.max(32, hitGauge * 1.8, screenGrabBuffer);

      const d = distanceToTrackPiece(wPos, piece.transform, entry);
      if (d < maxAllowedDist && d < minD) {
        minD = d;
        hit = piece;
      }
    }

    if (hit) {
      if (isMultiSelect) {
        selection.toggle(hit.id);
        if (!selection.has(hit.id)) {
          this.deps.sync();
          return;
        }
      } else {
        if (selection.has(hit.id) && selection.all().length > 1) {
          this.pendingSingleSelect = hit.id;
        } else {
          selection.set(hit.id);
        }
      }

      this.isDraggingPiece = true;
      this.activePiece = hit;
      this.activeModel = null;
      this.dragStartWorld = { ...wPos };
      this.originalTransform = { ...hit.transform };

      this.draggedPiecesMap.clear();
      for (const id of selection.all()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && this.deps.isPieceInteractive(p)) this.draggedPiecesMap.set(p.id, { ...p.transform });
        const m = layout.models?.find((x) => x.id === id);
        if (m && this.deps.isModelInteractive(m)) this.draggedPiecesMap.set(m.id, { ...m.transform });
      }

      this.draggedInitialConnections = [];
      for (const [id] of this.draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          for (const c of p.connectors) {
            if (c.connectedTo && !this.draggedPiecesMap.has(c.connectedTo.pieceId)) {
              this.draggedInitialConnections.push({ pieceId: c.connectedTo.pieceId, connectorIndex: c.connectedTo.connectorIndex });
            }
          }
        }
      }

      canvas.style.cursor = "move";
      this.deps.sync();
    } else {
      const sidebar = this.deps.getSidebar();
      if (sidebar?.isBaseboardTabActive()) {
        const baseboards = getBaseboards(layout);
        let hitBoard: BaseboardConfig | null = null;

        for (let i = baseboards.length - 1; i >= 0; i--) {
          const b = baseboards[i]!;
          if (!b.enabled) continue;
          if (
            isPointInBaseboardHeader(wPos, b) ||
            isPointNearBaseboardEdge(wPos, b, 24) ||
            isPointInsideBaseboard(wPos, b)
          ) {
            hitBoard = b;
            break;
          }
        }

        if (hitBoard) {
          this.isDraggingBaseboard = true;
          this.activeBaseboard = hitBoard;
          sidebar?.setActiveBoardId(hitBoard.id);
          this.baseboardDragStartWorld = { ...wPos };
          const bBounds = getBaseboardBounds(hitBoard);
          this.baseboardOriginalPos = {
            positionMode: "custom",
            customX: bBounds.x,
            customY: bBounds.y,
          };

          this.baseboardInitialTrackTransforms.clear();
          for (const piece of layout.pieces) {
            const entry = this.deps.getCatalogEntryForPiece(piece);
            if (entry) {
              const center = getPieceCenterWorld(entry, piece.transform);
              if (isPointInsideBaseboard(center, hitBoard)) {
                this.baseboardInitialTrackTransforms.set(piece.id, { ...piece.transform });
              }
            }
          }

          selection.clear();
          canvas.style.cursor = "grabbing";
          this.deps.setDraggingBaseboardState(true, this.activeBaseboard, this.activeBaseboardSnapGuides);
          this.deps.sync();
          return;
        }
      }

      this.isBoxSelecting = true;
      this.boxStartWorld = { ...wPos };
      this.deps.setSelectionBox({ start: { ...wPos }, end: { ...wPos } });
      this.initialBoxSelection = new Set(isMultiSelect ? selection.all() : []);
      if (!isMultiSelect) {
        selection.clear();
      }
      canvas.style.cursor = "crosshair";
      this.deps.sync();
    }
  }

  private onPointerMove(e: PointerEvent): void {
    const { renderer, canvas } = this.deps;
    const layout = this.deps.getLayout();
    const catalog = this.deps.getCatalog();
    const selection = this.deps.getSelection();
    const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);

    this.deps.getCollabClient().broadcastCursor(currentWPos.x, currentWPos.y);

    if (this.deps.isMeasureToolActive()) {
      const forceNoSnap = e.altKey;
      const snap = this.deps.findMeasureSnapPoint(currentWPos, 20, forceNoSnap);
      this.deps.setMeasureState({ snapPoint: snap });
      const mState = this.deps.getMeasureState();

      if (mState.draggingPoint === "A") {
        this.deps.setMeasureState({ start: snap ? { ...snap } : { ...currentWPos } });
        canvas.style.cursor = "grabbing";
        this.deps.updateMeasureHud();
      } else if (mState.draggingPoint === "B") {
        this.deps.setMeasureState({ current: snap ? { ...snap } : { ...currentWPos } });
        canvas.style.cursor = "grabbing";
        this.deps.updateMeasureHud();
      } else if (mState.start && !mState.locked) {
        this.deps.setMeasureState({ current: snap ? { ...snap } : { ...currentWPos } });
        canvas.style.cursor = "crosshair";
        this.deps.updateMeasureHud();
      } else {
        const nearA = mState.start && Math.hypot(currentWPos.x - mState.start.x, currentWPos.y - mState.start.y) <= 15;
        const nearB = mState.current && Math.hypot(currentWPos.x - mState.current.x, currentWPos.y - mState.current.y) <= 15;
        canvas.style.cursor = nearA || nearB ? "grab" : "crosshair";
      }

      this.deps.sync();
      return;
    }

    if (this.isBoxSelecting) {
      this.deps.setSelectionBox({ start: this.boxStartWorld, end: currentWPos });
      const distBox = Math.hypot(currentWPos.x - this.boxStartWorld.x, currentWPos.y - this.boxStartWorld.y);
      if (distBox > 3) {
        const newlyFound = new Set<string>(this.initialBoxSelection);
        for (const piece of layout.pieces) {
          if (!this.deps.isPieceInteractive(piece)) continue;
          const entry = this.deps.getCatalogEntryForPiece(piece);
          if (entry && isPieceInSelectionBox(piece, entry, this.boxStartWorld, currentWPos)) {
            newlyFound.add(piece.id);
          }
        }
        if (layout.models) {
          const minX = Math.min(this.boxStartWorld.x, currentWPos.x);
          const maxX = Math.max(this.boxStartWorld.x, currentWPos.x);
          const minY = Math.min(this.boxStartWorld.y, currentWPos.y);
          const maxY = Math.max(this.boxStartWorld.y, currentWPos.y);
          for (const model of layout.models) {
            if (!this.deps.isModelInteractive(model)) continue;
            if (model.transform.x >= minX && model.transform.x <= maxX && model.transform.y >= minY && model.transform.y <= maxY) {
              newlyFound.add(model.id);
            }
          }
        }
        selection.setMany([...newlyFound]);
      }
      this.deps.sync();
      return;
    }

    if (this.isRotatingModel && this.activeModel) {
      const dx = currentWPos.x - this.activeModel.transform.x;
      const dy = currentWPos.y - this.activeModel.transform.y;
      const pointerAngleDeg = rad2deg(Math.atan2(dy, dx));
      let newRot = normAngle(pointerAngleDeg + 90);
      if (!e.altKey) {
        newRot = Math.round(newRot / 7.5) * 7.5;
      }
      this.activeModel.transform.rotation = newRot;
      this.deps.sync();
      return;
    }

    if (this.isRotatingPiece && this.draggedPiecesMap.size > 0) {
      const startAngle = Math.atan2(this.dragStartWorld.y - this.pieceCenterWorld.y, this.dragStartWorld.x - this.pieceCenterWorld.x);
      const currentAngle = Math.atan2(currentWPos.y - this.pieceCenterWorld.y, currentWPos.x - this.pieceCenterWorld.x);
      let deltaDeg = rad2deg(currentAngle - startAngle);

      if (!e.altKey) {
        deltaDeg = Math.round(deltaDeg / 7.5) * 7.5;
      }

      const rotItems: RotatableItem[] = [];
      for (const [id, origT] of this.draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          const pEntry = this.deps.getCatalogEntryForPiece(p);
          rotItems.push({ id, type: "piece", transform: origT, entry: pEntry });
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m) {
          rotItems.push({ id, type: "model", transform: origT });
        }
      }

      const newTransforms = rotateGroupAroundCentroid(rotItems, deltaDeg, this.pieceCenterWorld);

      if (this.deps.isSnappingEnabled() && !e.altKey) {
        const otherPieces = layout.pieces.filter((p) => !this.draggedPiecesMap.has(p.id));
        let bestSnap: { itemSnap: BestPieceSnap; origT: Transform } | null = null;
        let bestDist = Infinity;

        for (const [id] of this.draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (!p) continue;
          const pEntry = this.deps.getCatalogEntryForPiece(p);
          if (!pEntry) continue;
          const candT = newTransforms.get(id);
          if (!candT) continue;

          const snap = findBestSnapForPiece(pEntry, candT, otherPieces, undefined, 35, 30, undefined, p, layout.layers);
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            bestSnap = { itemSnap: snap, origT: this.draggedPiecesMap.get(id)! };
          }
        }

        if (bestSnap) {
          const { itemSnap, origT } = bestSnap;
          const snapDeltaRot = normAngle(itemSnap.snappedTransform.rotation - origT.rotation);
          for (const [id, oT] of this.draggedPiecesMap.entries()) {
            const relOffset = { x: oT.x - origT.x, y: oT.y - origT.y };
            const rotOffset = rotate(relOffset, snapDeltaRot);
            const snappedPos: Transform = {
              x: itemSnap.snappedTransform.x + rotOffset.x,
              y: itemSnap.snappedTransform.y + rotOffset.y,
              rotation: normAngle(oT.rotation + snapDeltaRot),
            };
            const p = layout.pieces.find((x) => x.id === id);
            if (p) {
              p.transform = snappedPos;
              const pEntry = this.deps.getCatalogEntryForPiece(p);
              if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
            }
            const m = layout.models?.find((x) => x.id === id);
            if (m) m.transform = snappedPos;
          }
          this.deps.sync();
          return;
        }
      }

      for (const [id, newT] of newTransforms.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          p.transform = { ...newT };
          const pEntry = this.deps.getCatalogEntryForPiece(p);
          if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m) m.transform = { ...newT };
      }

      this.deps.sync();
      return;
    }

    if (this.isDraggingBaseboard && this.activeBaseboard) {
      const dx = currentWPos.x - this.baseboardDragStartWorld.x;
      const dy = currentWPos.y - this.baseboardDragStartWorld.y;

      let candidateX = this.baseboardOriginalPos.customX + dx;
      let candidateY = this.baseboardOriginalPos.customY + dy;

      const otherBoards = getBaseboards(layout).filter((b) => b.id !== this.activeBaseboard!.id && b.enabled !== false);

      if (!e.altKey && otherBoards.length > 0) {
        const snap = snapBaseboard(this.activeBaseboard, candidateX, candidateY, otherBoards, 35);
        candidateX = snap.x;
        candidateY = snap.y;
        this.activeBaseboardSnapGuides = snap.guides;
      } else {
        this.activeBaseboardSnapGuides = [];
        if (!e.altKey) {
          candidateX = Math.round(candidateX / 20) * 20;
          candidateY = Math.round(candidateY / 20) * 20;
        }
      }

      this.activeBaseboard.positionMode = "custom";
      this.activeBaseboard.customX = Math.round(candidateX);
      this.activeBaseboard.customY = Math.round(candidateY);

      if (e.shiftKey && this.baseboardInitialTrackTransforms.size > 0) {
        const deltaX = this.activeBaseboard.customX - this.baseboardOriginalPos.customX;
        const deltaY = this.activeBaseboard.customY - this.baseboardOriginalPos.customY;
        for (const [pId, initT] of this.baseboardInitialTrackTransforms.entries()) {
          const piece = layout.pieces.find((p) => p.id === pId);
          if (piece) {
            piece.transform.x = initT.x + deltaX;
            piece.transform.y = initT.y + deltaY;
            const entry = this.deps.getCatalogEntryForPiece(piece);
            if (entry) {
              piece.connectors = computeWorldConnectors(entry, piece.transform);
            }
          }
        }
      }

      this.deps.getSidebar()?.update(layout, this.deps.getCurrentSimulation());
      this.deps.setDraggingBaseboardState(true, this.activeBaseboard, this.activeBaseboardSnapGuides);
      this.deps.sync();
      return;
    }

    if (this.isDraggingPiece && (this.activePiece || this.activeModel)) {
      const dx = currentWPos.x - this.dragStartWorld.x;
      const dy = currentWPos.y - this.dragStartWorld.y;
      const dragDist = Math.hypot(dx, dy);

      if (!this.hasDragMoved) {
        if (dragDist < 3.0) return;
        this.hasDragMoved = true;
        this.pendingSingleSelect = null;
      }

      let groupSnap: {
        snappedTransform: Transform;
        sourceId: string;
        origTransform: Transform;
      } | null = null;

      if (this.deps.isSnappingEnabled()) {
        const otherPieces = layout.pieces.filter((p) => !this.draggedPiecesMap.has(p.id));
        let bestDist = Infinity;
        const ignoreConn = dragDist < 28 ? this.draggedInitialConnections : undefined;

        for (const [id, origT] of this.draggedPiecesMap.entries()) {
          const piece = layout.pieces.find((p) => p.id === id);
          if (!piece) continue;
          const entry = this.deps.getCatalogEntryForPiece(piece);
          if (!entry) continue;

          const candT: Transform = {
            x: origT.x + dx,
            y: origT.y + dy,
            rotation: origT.rotation,
          };

          const snap = findBestSnapForPiece(entry, candT, otherPieces, undefined, 45, 60, ignoreConn, piece, layout.layers);
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            groupSnap = {
              snappedTransform: snap.snappedTransform,
              sourceId: id,
              origTransform: origT,
            };
          }
        }
      }

      if (groupSnap) {
        const { snappedTransform, origTransform } = groupSnap;
        const deltaRot = normAngle(snappedTransform.rotation - origTransform.rotation);

        for (const [id, origT] of this.draggedPiecesMap.entries()) {
          const relOffset = { x: origT.x - origTransform.x, y: origT.y - origTransform.y };
          const rotOffset = rotate(relOffset, deltaRot);
          const newPos: Transform = {
            x: snappedTransform.x + rotOffset.x,
            y: snappedTransform.y + rotOffset.y,
            rotation: normAngle(origT.rotation + deltaRot),
          };

          const p = layout.pieces.find((x) => x.id === id);
          if (p) {
            p.transform = newPos;
            const pEntry = this.deps.getCatalogEntryForPiece(p);
            if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m) m.transform = newPos;
        }
      } else {
        for (const [id, origT] of this.draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p) {
            p.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
            const pEntry = this.deps.getCatalogEntryForPiece(p);
            if (pEntry) {
              p.connectors = computeWorldConnectors(pEntry, p.transform);
            }
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m) {
            m.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
          }
        }
      }

      this.deps.sync();
      return;
    }

    if (!this.isDraggingPiece && !this.isRotatingPiece && !this.isRotatingModel && !this.isBoxSelecting && !this.isDraggingBaseboard) {
      const selectedIds = selection.all();
      const scale = renderer.world.scale.x || 1;
      const handleHitRadius = Math.max(24, 20 / scale);

      for (const selId of selectedIds) {
        const selPiece = layout.pieces.find((p) => p.id === selId);
        if (selPiece && this.deps.isPieceInteractive(selPiece)) {
          const entry = this.deps.getCatalogEntryForPiece(selPiece);
          if (entry) {
            const handlePos = getPieceRotationHandleWorld(entry, selPiece.transform);
            if (dist(currentWPos, handlePos) <= handleHitRadius) {
              canvas.style.cursor = "grab";
              return;
            }
          }
        }
        const selModel = layout.models?.find((m) => m.id === selId);
        if (selModel && this.deps.isModelInteractive(selModel)) {
          const handlePos = getModelRotationHandleWorld(selModel);
          if (dist(currentWPos, handlePos) <= handleHitRadius) {
            canvas.style.cursor = "grab";
            return;
          }
        }
      }

      let isOverTrack = false;
      const screenGrabBuffer = 20 / scale;
      for (const piece of layout.pieces) {
        if (!this.deps.isPieceInteractive(piece)) continue;
        const entry = this.deps.getCatalogEntryForPiece(piece);
        if (entry) {
          const maxAllowed = entry.isRoad
            ? Math.max(35, (entry.roadWidth ?? 60) * 0.65, screenGrabBuffer)
            : Math.max(32, (catalog.gauge ?? 16.5) * 1.8, screenGrabBuffer);
          if (distanceToTrackPiece(currentWPos, piece.transform, entry) <= maxAllowed) {
            isOverTrack = true;
            break;
          }
        }
      }

      if (isOverTrack) {
        canvas.style.cursor = "pointer";
      } else {
        let isOverTable = false;
        const sidebar = this.deps.getSidebar();
        if (sidebar?.isBaseboardTabActive()) {
          const baseboards = getBaseboards(layout);
          for (const b of baseboards) {
            if (!b.enabled) continue;
            if (
              isPointInBaseboardHeader(currentWPos, b) ||
              isPointNearBaseboardEdge(currentWPos, b, 24) ||
              isPointInsideBaseboard(currentWPos, b)
            ) {
              isOverTable = true;
              break;
            }
          }
        }
        canvas.style.cursor = isOverTable ? "grab" : "default";
      }
    }
  }

  private onPointerUp(e: PointerEvent): void {
    const { renderer, canvas } = this.deps;
    const layout = this.deps.getLayout();
    const history = this.deps.getHistory();
    const selection = this.deps.getSelection();

    if (this.deps.isMeasureToolActive()) {
      const mState = this.deps.getMeasureState();
      if (mState.draggingPoint) {
        this.deps.setMeasureState({ draggingPoint: null, locked: true });
        canvas.style.cursor = "crosshair";
        this.deps.updateMeasureHud();
        this.deps.sync();
      } else if (mState.start && !mState.locked) {
        const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
        const forceNoSnap = e.altKey;
        const snap = this.deps.findMeasureSnapPoint(currentWPos, 20, forceNoSnap);
        const pt = snap ? { ...snap } : { ...currentWPos };
        if (Math.hypot(pt.x - mState.start.x, pt.y - mState.start.y) > 8) {
          this.deps.setMeasureState({ current: pt, locked: true });
          this.deps.updateMeasureHud();
          this.deps.sync();
        }
      }
      return;
    }

    if (this.isBoxSelecting) {
      const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
      const distBox = Math.hypot(currentWPos.x - this.boxStartWorld.x, currentWPos.y - this.boxStartWorld.y);
      const isMultiSelect = e.ctrlKey || e.metaKey || e.shiftKey;

      if (distBox <= 5 && !isMultiSelect) {
        selection.clear();
      }

      this.isBoxSelecting = false;
      this.deps.setSelectionBox(null);
      canvas.style.cursor = "default";
      this.deps.sync();
      return;
    }

    if (this.isRotatingModel && this.activeModel) {
      const finalT = { ...this.activeModel.transform };
      if (finalT.rotation !== this.originalTransform.rotation) {
        history.run(new MoveModelCommand(this.activeModel.id, this.originalTransform, finalT));
        this.deps.sync(true);
      }
      this.isRotatingModel = false;
      this.activeModel = null;
      canvas.style.cursor = "default";
      this.deps.sync();
    } else if (this.isRotatingPiece) {
      let movedAny = false;
      for (const [id, origT] of this.draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && (p.transform.x !== origT.x || p.transform.y !== origT.y || p.transform.rotation !== origT.rotation)) {
          history.run(new MovePieceCommand(p.id, origT, { ...p.transform }));
          movedAny = true;
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m && (m.transform.x !== origT.x || m.transform.y !== origT.y || m.transform.rotation !== origT.rotation)) {
          history.run(new MoveModelCommand(m.id, origT, { ...m.transform }));
          movedAny = true;
        }
      }
      if (movedAny) this.deps.sync(true);
      this.isRotatingPiece = false;
      this.activePiece = null;
      this.draggedPiecesMap.clear();
      canvas.style.cursor = "default";
      this.deps.sync();
    } else if (this.isDraggingPiece) {
      if (!this.hasDragMoved && this.pendingSingleSelect) {
        selection.set(this.pendingSingleSelect);
      } else if (this.hasDragMoved) {
        let movedAny = false;
        for (const [id, origT] of this.draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p && (p.transform.x !== origT.x || p.transform.y !== origT.y || p.transform.rotation !== origT.rotation)) {
            history.run(new MovePieceCommand(p.id, origT, { ...p.transform }));
            movedAny = true;
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m && (m.transform.x !== origT.x || m.transform.y !== origT.y || m.transform.rotation !== origT.rotation)) {
            history.run(new MoveModelCommand(m.id, origT, { ...m.transform }));
            movedAny = true;
          }
        }
        if (movedAny) {
          this.deps.sync(true);
        }
      }
      this.isDraggingPiece = false;
      this.activePiece = null;
      this.activeModel = null;
      this.pendingSingleSelect = null;
      this.hasDragMoved = false;
      this.draggedPiecesMap.clear();
      canvas.style.cursor = "default";
      this.deps.sync();
    } else if (this.isDraggingBaseboard && this.activeBaseboard) {
      const finalX = this.activeBaseboard.customX ?? 0;
      const finalY = this.activeBaseboard.customY ?? 0;
      if (finalX !== this.baseboardOriginalPos.customX || finalY !== this.baseboardOriginalPos.customY) {
        history.run(
          new MoveBaseboardCommand(
            this.activeBaseboard.id ?? "board-1",
            { ...this.baseboardOriginalPos },
            { positionMode: "custom", customX: finalX, customY: finalY },
          ),
        );
        this.deps.sync(true);
      }
      this.isDraggingBaseboard = false;
      this.activeBaseboardSnapGuides = [];
      this.baseboardInitialTrackTransforms.clear();
      this.deps.setDraggingBaseboardState(false, null, []);
      canvas.style.cursor = "default";
      this.deps.sync();
    }
  }

  private onWheel(e: WheelEvent): void {
    if (e.shiftKey || e.altKey) {
      const sel = this.deps.getSelection().all()[0];
      if (sel) {
        e.preventDefault();
        e.stopPropagation();
        const step = e.deltaY < 0 ? -7.5 : 7.5;
        this.deps.getOperations().rotateSelected(step);
      }
    }
  }
}
