import type { TrackPiece, Model3D, Transform, BaseboardConfig } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { computeWorldConnectors } from "@/catalog/catalog";
import { updatePieceConnections } from "@/core/snapping";
import { snapBaseboard, type BaseboardSnapGuide } from "@/core/baseboard-snapping";
import {
  getBaseboards, getBaseboardBounds, isPointInsideBaseboard, isPointNearBaseboardEdge, isPointInBaseboardHeader,
} from "@/core/layout";
import { isPointInsideModel, getModelRotationHandleWorld } from "@/core/models-3d";
import {
  dist, normAngle, rad2deg, getPieceCenterWorld,
  getPieceRotationHandleWorld, isPieceInSelectionBox, computeGroupCentroid, type RotatableItem,
} from "@/core/geometry";
import { MoveModelCommand, MoveBaseboardCommand } from "@/editor/commands";
import { handleCanvasDrop } from "./canvas-drop-handler";
import { KeyboardController } from "./keyboard-controller";
import { adjustFlexTrackOpenConnector, bendFlexTrackBetweenFixedEnds } from "@/core/flextrack";
import { closeTrackContextMenu } from "@/ui/track-context-menu";
import type { CanvasControllerDeps } from "./canvas-controller-types";
import { openTrackContextMenu } from "./canvas-context-menu";
import {
  findOpenConnectorAt, findPointMarkerAt, checkFlexHandleHit, isNearFlexHandle, findTrackPieceAt,
} from "./canvas-hit-test";
import {
  populateDraggedPiecesMap, commitMovedItems, applyGroupRotationWithSnapping, applyGroupDragWithSnapping,
  handleMeasurePointerDown, handleMeasurePointerMove, handleMeasurePointerUp,
} from "./canvas-drag-helper";
import { terrainBrush } from "./terrain-brush-controller";
import { boardSketcher } from "./board-sketcher-controller";

export type { CanvasControllerDeps } from "./canvas-controller-types";

export class CanvasController {
  private deps: CanvasControllerDeps;
  private isDraggingPiece = false; private isRotatingPiece = false; private isRotatingModel = false;
  private isDraggingPin = false;
  private activePin: { x: number; y: number; label: string; color: number; id?: string } | null = null;
  private activePinOriginalPos: { x: number; y: number } | null = null;
  private isBoxSelecting = false; private boxStartWorld = { x: 0, y: 0 };
  private initialBoxSelection = new Set<string>();
  private activePiece: TrackPiece | null = null; private activeModel: Model3D | null = null;
  private dragStartWorld = { x: 0, y: 0 }; private originalTransform: Transform = { x: 0, y: 0, rotation: 0 };
  private draggedPiecesMap = new Map<string, Transform>();
  private draggedInitialConnections: { pieceId: string; connectorIndex: number }[] = [];
  private pieceCenterWorld = { x: 0, y: 0 }; private pendingSingleSelect: string | null = null; private hasDragMoved = false;
  private hasUnlatchedFromInitial = false;
  private isDraggingBaseboard = false; private activeBaseboard: BaseboardConfig | null = null;
  private baseboardDragStartWorld = { x: 0, y: 0 };
  private baseboardOriginalPos = { positionMode: "custom" as const, customX: 0, customY: 0 };
  private activeBaseboardSnapGuides: BaseboardSnapGuide[] = [];
  private baseboardInitialTrackTransforms = new Map<string, Transform>();
  private isAdjustingFlex = false;
  private isAdjustingFlexMiddle = false;
  private flexMiddleFixedC0: { x: number; y: number } | null = null;
  private flexMiddleFixedC1: { x: number; y: number } | null = null;
  private activeFlexPivotFraction = 0.5;
  private activeFlexPiece: TrackPiece | null = null;
  private activeFlexEntry: CatalogEntry | null = null;
  private handleDragOver = (e: DragEvent) => this.onDragOver(e);
  private handleDrop = (e: DragEvent) => this.onDrop(e);
  private handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    const { renderer } = this.deps;
    const layout = this.deps.getLayout();
    const selection = this.deps.getSelection();
    const wPos = renderer.screenToWorld(e.clientX, e.clientY);
    const scale = renderer.world.scale.x || 1;
    const catalog = this.deps.getCatalog();
    const hitGauge = catalog.gauge ?? 16.5;
    const hit = findTrackPieceAt(
      layout,
      wPos,
      scale,
      hitGauge,
      (p) => this.deps.isPieceInteractive(p),
      (p) => this.deps.getCatalogEntryForPiece(p),
    );

    if (hit) {
      if (!selection.has(hit.id)) {
        selection.set(hit.id);
      }
      this.deps.sync();
      openTrackContextMenu(this.deps, hit, e.clientX, e.clientY);
    } else {
      closeTrackContextMenu();
    }
  };
  private handlePointerDown = (e: PointerEvent) => this.onPointerDown(e);
  private rafMoveId: number | null = null;
  private latestMoveEvent: PointerEvent | null = null;

  private handlePointerMove = (e: PointerEvent) => {
    this.latestMoveEvent = e;
    if (typeof requestAnimationFrame === "undefined") {
      this.onPointerMove(e);
      return;
    }
    if (this.rafMoveId === null) {
      this.rafMoveId = requestAnimationFrame(() => {
        this.rafMoveId = null;
        if (this.latestMoveEvent) {
          this.onPointerMove(this.latestMoveEvent);
        }
      });
    }
  };

  private handlePointerUp = (e: PointerEvent) => {
    if (this.rafMoveId !== null) {
      cancelAnimationFrame(this.rafMoveId);
      this.rafMoveId = null;
      if (this.latestMoveEvent) {
        this.onPointerMove(this.latestMoveEvent);
      }
    }
    this.onPointerUp(e);
  };
  private handleWheel = (e: WheelEvent) => this.onWheel(e);
  private keyboardController: KeyboardController;

  constructor(deps: CanvasControllerDeps) {
    this.deps = deps;
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
    if (this.rafMoveId !== null) {
      cancelAnimationFrame(this.rafMoveId);
      this.rafMoveId = null;
    }
    closeTrackContextMenu();
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
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  }
  private async onDrop(e: DragEvent): Promise<void> {
    await handleCanvasDrop(e, this.deps.renderer, this.deps.getOperations(), this.deps.sync);
  }
  private onPointerDown(e: PointerEvent): void {
    closeTrackContextMenu();
    const { renderer, canvas } = this.deps;
    const layout = this.deps.getLayout();
    const selection = this.deps.getSelection();

    if (this.deps.isMeasureToolActive()) {
      handleMeasurePointerDown(e, this.deps);
      return;
    }

    if (boardSketcher.isActive) {
      const wPos = renderer.screenToWorld(e.clientX, e.clientY);
      const scale = renderer.world.scale.x || 1;
      boardSketcher.onPointerDown(wPos, layout, scale, e.button, e.altKey);
      canvas.style.cursor = "crosshair";
      this.deps.sync(true);
      return;
    }

    if (terrainBrush.isActive) {
      if (e.button === 2) {
        terrainBrush.setActive(false);
        canvas.style.cursor = "default";
        this.deps.sync();
      } else if (e.button === 0) {
        const wPos = renderer.screenToWorld(e.clientX, e.clientY);
        terrainBrush.startStroke(wPos, layout);
        canvas.style.cursor = "crosshair";
        this.deps.sync(terrainBrush.mode === "erase");
      }
      return;
    }

    if (e.button !== 0) return;

    const wPos = renderer.screenToWorld(e.clientX, e.clientY);
    const scale = renderer.world.scale.x || 1;

    // Pin Drag & Drop: Start- und Stoppunkt-Pins (A / B) mit der Maus greifen
    const hitPin = findPointMarkerAt(this.deps.getPointMarkers?.(), wPos, scale);
    if (hitPin && (this.deps.getPinDropHandler?.() || this.deps.getPickHandler?.())) {
      this.isDraggingPin = true;
      this.activePin = hitPin;
      this.activePinOriginalPos = { x: hitPin.x, y: hitPin.y };
      canvas.setPointerCapture(e.pointerId);
      canvas.style.cursor = "grabbing";
      return;
    }

    const pickHandler = this.deps.getPickHandler?.();
    if (pickHandler) {
      const catalog = this.deps.getCatalog();
      const hitGauge = catalog.gauge ?? 16.5;
      const hit = findTrackPieceAt(
        layout,
        wPos,
        scale,
        hitGauge,
        (p) => this.deps.isPieceInteractive(p),
        (p) => this.deps.getCatalogEntryForPiece(p),
      );
      pickHandler(wPos, hit);
      return;
    }

    const isMultiSelect = e.ctrlKey || e.metaKey || e.shiftKey;
    this.hasDragMoved = false;
    this.pendingSingleSelect = null;

    const selectedIds = selection.all();
    const handleHitRadius = Math.max(24, 20 / scale);

    // Flexgleis-Griffe zum Kürzen und Biegen immer zuerst vor Plus-Buttons prüfen
    if (!isMultiSelect) {
      const flexHit = checkFlexHandleHit(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        (p) => this.deps.getCatalogEntryForPiece(p),
        wPos,
        handleHitRadius,
      );
      if (flexHit) {
        if (flexHit.handleType === "middle" && flexHit.fixedC0 && flexHit.fixedC1) {
          this.isAdjustingFlexMiddle = true;
          this.activeFlexPivotFraction = flexHit.pivotFraction ?? 0.5;
          this.flexMiddleFixedC0 = { ...flexHit.fixedC0 };
          this.flexMiddleFixedC1 = { ...flexHit.fixedC1 };
        } else {
          this.isAdjustingFlex = true;
        }
        this.activeFlexPiece = flexHit.piece;
        this.activeFlexEntry = flexHit.entry;
        this.deps.canvas.style.cursor = "crosshair";
        this.deps.sync();
        return;
      }
    }

    // Klick auf Plus-Button an offenem Gleisende
    if (!isMultiSelect) {
      const openHit = findOpenConnectorAt(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        wPos,
        scale,
        (p) => this.deps.getCatalogEntryForPiece(p),
      );
      if (openHit) {
        const added = this.deps.getOperations().appendTrackToConnector(openHit.piece, openHit.connectorIndex);
        if (added) {
          this.deps.sync(true);
          return;
        }
      }
    }

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
            if (selModel.builtinId === "mountain") {
              this.deps.getSidebar()?.selectMountain(selModel.id);
            }
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

    const catalog = this.deps.getCatalog();
    const hitGauge = catalog.gauge ?? 16.5;
    const hit = findTrackPieceAt(
      layout,
      wPos,
      scale,
      hitGauge,
      (p) => this.deps.isPieceInteractive(p),
      (p) => this.deps.getCatalogEntryForPiece(p),
    );

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

    // Bei Klick auf ein Gleis/eine Straße im Berg hat das Gleis Vorrang (außer Klick direkt aufs Berg-Zentrum)
    if (
      hitModel?.builtinId === "mountain" &&
      hit &&
      !selection.has(hitModel.id) &&
      Math.hypot(wPos.x - hitModel.transform.x, wPos.y - hitModel.transform.y) > 40
    ) {
      hitModel = null;
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
      if (hitModel.builtinId === "mountain") {
        this.deps.getSidebar()?.selectMountain(hitModel.id);
      }
      this.isDraggingPiece = true;
      this.activePiece = null;
      this.activeModel = hitModel;
      this.dragStartWorld = { ...wPos };
      this.originalTransform = { ...hitModel.transform };
      populateDraggedPiecesMap(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        (m) => this.deps.isModelInteractive(m),
        this.draggedPiecesMap,
        this.draggedInitialConnections,
      );
      canvas.style.cursor = "move";
      this.deps.sync();
      return;
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
      populateDraggedPiecesMap(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        (m) => this.deps.isModelInteractive(m),
        this.draggedPiecesMap,
        this.draggedInitialConnections,
      );
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
      handleMeasurePointerMove(e, currentWPos, this.deps);
      return;
    }

    if (boardSketcher.isActive) {
      const scale = renderer.world.scale.x || 1;
      boardSketcher.onPointerMove(currentWPos, layout, scale, e.altKey);
      canvas.style.cursor =
        boardSketcher.draggingVertexIndex !== null
          ? "grabbing"
          : boardSketcher.hoveredVertexIndex !== null || boardSketcher.hoveredMidpointIndex !== null
            ? "pointer"
            : "crosshair";
      this.deps.sync(boardSketcher.draggingVertexIndex !== null);
      return;
    }

    if (terrainBrush.isActive) {
      terrainBrush.moveStroke(currentWPos, layout);
      canvas.style.cursor = "crosshair";
      this.deps.sync(terrainBrush.isDrawing && terrainBrush.mode === "erase");
      return;
    }

    // Pin Dragging aktiv: Pin folgt der Maus und snappt an naheliegende Anschlüsse
    if (this.isDraggingPin && this.activePin) {
      canvas.style.cursor = "grabbing";
      const scale = renderer.world.scale.x || 1;
      const layout = this.deps.getLayout();
      let snapTarget: { x: number; y: number } | null = null;
      let bestSnapDist = 32 / scale;

      for (const p of layout.pieces) {
        if (!this.deps.isPieceInteractive(p)) continue;
        for (const c of p.connectors) {
          const d = Math.hypot(currentWPos.x - c.x, currentWPos.y - c.y);
          if (d < bestSnapDist) {
            bestSnapDist = d;
            snapTarget = { x: c.x, y: c.y };
          }
        }
      }

      if (snapTarget) {
        this.activePin.x = snapTarget.x;
        this.activePin.y = snapTarget.y;
      } else {
        this.activePin.x = currentWPos.x;
        this.activePin.y = currentWPos.y;
      }

      this.deps.sync(false, true);
      return;
    }

    // Hover über Pin Marker: Mauszeiger wird zur Hand ("grab")
    const hoveringPin = findPointMarkerAt(this.deps.getPointMarkers?.(), currentWPos, renderer.world.scale.x || 1);
    if (hoveringPin && (this.deps.getPinDropHandler?.() || this.deps.getPickHandler?.())) {
      canvas.style.cursor = "grab";
      return;
    }

    if (this.deps.getPickHandler?.()) {
      canvas.style.cursor = "crosshair";
      return;
    }

    if (this.isAdjustingFlexMiddle && this.activeFlexPiece && this.activeFlexEntry && this.flexMiddleFixedC0 && this.flexMiddleFixedC1) {
      bendFlexTrackBetweenFixedEnds(
        this.activeFlexPiece,
        this.activeFlexEntry,
        this.flexMiddleFixedC0,
        this.flexMiddleFixedC1,
        currentWPos,
        layout.layers,
        this.activeFlexPivotFraction,
      );
      canvas.style.cursor = "crosshair";
      this.deps.sync(false, true);
      return;
    }

    if (this.isAdjustingFlex && this.activeFlexPiece && this.activeFlexEntry) {
      adjustFlexTrackOpenConnector(
        this.activeFlexPiece,
        this.activeFlexEntry,
        currentWPos,
        layout.pieces,
        layout.layers,
        this.deps.isSnappingEnabled() ? 25 : 0,
      );
      canvas.style.cursor = "crosshair";
      this.deps.sync(false, true);
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
        if (newlyFound.size === 1) {
          const onlyId = newlyFound.values().next().value;
          const m = layout.models?.find((x) => x.id === onlyId);
          if (m?.builtinId === "mountain") {
            this.deps.getSidebar()?.selectMountain(m.id);
          }
        }
      }
      this.deps.sync(false, true);
      return;
    }

    if (this.isRotatingModel && this.activeModel) {
      const dx = currentWPos.x - this.activeModel.transform.x;
      const dy = currentWPos.y - this.activeModel.transform.y;
      const pointerAngleDeg = rad2deg(Math.atan2(dy, dx));
      let newRot = normAngle(pointerAngleDeg + 90);
      const isLocked = this.deps.isRotationSnapLocked ? this.deps.isRotationSnapLocked() : true;
      const step = this.deps.getRotationStep ? this.deps.getRotationStep() : 7.5;
      const shouldSnap = isLocked ? !e.altKey : e.altKey;
      if (shouldSnap) {
        newRot = Math.round(newRot / step) * step;
      }
      this.activeModel.transform.rotation = newRot;
      this.deps.sync(false, true);
      return;
    }

    if (this.isRotatingPiece && this.draggedPiecesMap.size > 0) {
      const startAngle = Math.atan2(this.dragStartWorld.y - this.pieceCenterWorld.y, this.dragStartWorld.x - this.pieceCenterWorld.x);
      const currentAngle = Math.atan2(currentWPos.y - this.pieceCenterWorld.y, currentWPos.x - this.pieceCenterWorld.x);
      let deltaDeg = rad2deg(currentAngle - startAngle);

      const isLocked = this.deps.isRotationSnapLocked ? this.deps.isRotationSnapLocked() : true;
      const step = this.deps.getRotationStep ? this.deps.getRotationStep() : 7.5;
      const shouldSnap = isLocked ? !e.altKey : e.altKey;
      if (shouldSnap) {
        deltaDeg = Math.round(deltaDeg / step) * step;
      }

      applyGroupRotationWithSnapping(
        layout,
        this.draggedPiecesMap,
        this.pieceCenterWorld,
        deltaDeg,
        this.deps.isSnappingEnabled() && shouldSnap,
        (p) => this.deps.getCatalogEntryForPiece(p),
      );
      this.deps.sync(false, true);
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

      this.deps.setDraggingBaseboardState(true, this.activeBaseboard, this.activeBaseboardSnapGuides);
      this.deps.sync(false, true);
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

      applyGroupDragWithSnapping(
        layout,
        this.draggedPiecesMap,
        dx,
        dy,
        dragDist,
        this.deps.isSnappingEnabled(),
        this.hasUnlatchedFromInitial,
        this.draggedInitialConnections,
        (p) => this.deps.getCatalogEntryForPiece(p),
      );

      this.deps.sync(false, true);
      return;
    }

    if (!this.isDraggingPiece && !this.isRotatingPiece && !this.isRotatingModel && !this.isBoxSelecting && !this.isDraggingBaseboard && !this.isAdjustingFlex && !this.isAdjustingFlexMiddle) {
      const selectedIds = selection.all();
      const scale = renderer.world.scale.x || 1;
      const handleHitRadius = Math.max(24, 20 / scale);

      // Flexgleis-Griffe zum Kürzen und Biegen prüfen
      if (isNearFlexHandle(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        (p) => this.deps.getCatalogEntryForPiece(p),
        currentWPos,
        handleHitRadius,
      )) {
        canvas.style.cursor = "crosshair";
        return;
      }

      // Hover über offenem Gleisende / Plus-Button
      const openHit = findOpenConnectorAt(
        layout,
        selection,
        (p) => this.deps.isPieceInteractive(p),
        currentWPos,
        scale,
        (p) => this.deps.getCatalogEntryForPiece(p),
      );
      const currentHovered = this.deps.getHoveredOpenConnector?.() ?? null;
      const newHovered = openHit ? { pieceId: openHit.piece.id, connectorIndex: openHit.connectorIndex } : null;

      const hoveredChanged = (currentHovered?.pieceId !== newHovered?.pieceId) || (currentHovered?.connectorIndex !== newHovered?.connectorIndex);
      if (hoveredChanged && this.deps.setHoveredOpenConnector) {
        this.deps.setHoveredOpenConnector(newHovered);
        this.deps.sync(false, true);
      }

      if (openHit) {
        canvas.style.cursor = "pointer";
        return;
      }

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

      const isOverTrack = Boolean(
        findTrackPieceAt(
          layout,
          currentWPos,
          scale,
          catalog.gauge ?? 16.5,
          (p) => this.deps.isPieceInteractive(p),
          (p) => this.deps.getCatalogEntryForPiece(p),
        ),
      );

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
      handleMeasurePointerUp(e, this.deps);
      return;
    }

    if (boardSketcher.isActive) {
      const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
      if (boardSketcher.onPointerUp(currentWPos, layout, e.altKey)) {
        this.deps.sync(true);
      }
      return;
    }

    if (terrainBrush.isActive && terrainBrush.isDrawing) {
      const created = terrainBrush.finishStroke(layout);
      if (created) {
        selection.set(created.id);
        this.deps.getSidebar()?.selectMountain(created.id);
      }
      this.deps.sync(true);
      return;
    }

    // Pin Drag & Drop Abschluss: Pin auf Gleis fallen lassen
    if (this.isDraggingPin && this.activePin) {
      const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
      const scale = renderer.world.scale.x || 1;
      this.isDraggingPin = false;
      try { canvas.releasePointerCapture(e.pointerId); } catch (_) {}
      canvas.style.cursor = "default";

      const catalog = this.deps.getCatalog();
      const hitGauge = catalog.gauge ?? 16.5;
      const hit = findTrackPieceAt(
        layout,
        currentWPos,
        scale,
        hitGauge,
        (p) => this.deps.isPieceInteractive(p),
        (p) => this.deps.getCatalogEntryForPiece(p),
      );

      const pinId = this.activePin.id || (this.activePin.label.includes("B") || this.activePin.label === "Stop" ? "B" : "A");
      const pinDropHandler = this.deps.getPinDropHandler?.();

      if (hit && pinDropHandler) {
        pinDropHandler(pinId, currentWPos, hit);
      } else if (hit && this.deps.getPickHandler?.()) {
        this.deps.getPickHandler()!(currentWPos, hit);
      } else if (this.activePinOriginalPos) {
        this.activePin.x = this.activePinOriginalPos.x;
        this.activePin.y = this.activePinOriginalPos.y;
        this.deps.sync(false);
      }

      this.activePin = null;
      this.activePinOriginalPos = null;
      return;
    }

    if (this.isAdjustingFlexMiddle) {
      if (this.activeFlexPiece) updatePieceConnections(layout.pieces, layout.layers);
      this.isAdjustingFlexMiddle = false;
      this.activeFlexPivotFraction = 0.5;
      this.flexMiddleFixedC0 = null;
      this.flexMiddleFixedC1 = null;
      this.activeFlexPiece = null;
      this.activeFlexEntry = null;
      canvas.style.cursor = "default";
      this.deps.sync(true);
      return;
    }

    if (this.isAdjustingFlex) {
      if (this.activeFlexPiece) updatePieceConnections(layout.pieces, layout.layers);
      this.isAdjustingFlex = false;
      this.activeFlexPiece = null;
      this.activeFlexEntry = null;
      canvas.style.cursor = "default";
      this.deps.sync(true);
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
      if (commitMovedItems(layout, history, this.draggedPiecesMap)) {
        updatePieceConnections(layout.pieces, layout.layers);
        this.deps.sync(true);
      }
      this.isRotatingPiece = false; this.activePiece = null; this.draggedPiecesMap.clear();
      this.draggedInitialConnections = []; this.hasUnlatchedFromInitial = false;
      canvas.style.cursor = "default";
      this.deps.sync();
    } else if (this.isDraggingPiece) {
      if (!this.hasDragMoved && this.pendingSingleSelect) {
        selection.set(this.pendingSingleSelect);
        const m = layout.models?.find((x) => x.id === this.pendingSingleSelect);
        if (m?.builtinId === "mountain") {
          this.deps.getSidebar()?.selectMountain(m.id);
        }
      } else if (this.hasDragMoved && commitMovedItems(layout, history, this.draggedPiecesMap)) {
        updatePieceConnections(layout.pieces, layout.layers);
        this.deps.sync(true);
      }
      this.isDraggingPiece = false; this.activePiece = null; this.activeModel = null;
      this.pendingSingleSelect = null; this.hasDragMoved = false; this.hasUnlatchedFromInitial = false;
      this.draggedPiecesMap.clear(); this.draggedInitialConnections = [];
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
    closeTrackContextMenu();
    if (e.shiftKey || e.altKey) {
      if (terrainBrush.isActive) {
        e.preventDefault();
        e.stopPropagation();
        terrainBrush.adjustBrushRadius(e.deltaY < 0 ? 20 : -20);
        this.deps.sync();
        return;
      }
      const sel = this.deps.getSelection().all()[0];
      if (sel) {
        e.preventDefault();
        e.stopPropagation();
        const rotStep = this.deps.getRotationStep ? this.deps.getRotationStep() : 7.5;
        const step = e.deltaY < 0 ? -rotStep : rotStep;
        this.deps.getOperations().rotateSelected(step);
      }
    }
  }
}
