import type { Layout, TrackPiece, Transform, Model3D } from "@/core/types";
import { getBaseboards } from "@/core/layout";

export interface Command {
  do(layout: Layout): void;
  undo(layout: Layout): void;
}

export class History {
  private undoStack: Command[] = [];
  private redoStack: Command[] = [];

  constructor(private layout: Layout) {}

  run(cmd: Command) {
    cmd.do(this.layout);
    this.undoStack.push(cmd);
    this.redoStack = [];
  }

  undo() {
    const c = this.undoStack.pop();
    if (c) {
      c.undo(this.layout);
      this.redoStack.push(c);
    }
  }

  redo() {
    const c = this.redoStack.pop();
    if (c) {
      c.do(this.layout);
      this.undoStack.push(c);
    }
  }

  canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
  }
}

export class AddPieceCommand implements Command {
  constructor(private piece: TrackPiece) {}

  do(layout: Layout): void {
    layout.pieces.push(this.piece);
  }

  undo(layout: Layout): void {
    layout.pieces = layout.pieces.filter((p) => p.id !== this.piece.id);
  }
}

export class AddPiecesCommand implements Command {
  constructor(
    private pieces: TrackPiece[],
    private models?: Model3D[],
  ) {}

  do(layout: Layout): void {
    if (this.pieces.length > 0) {
      layout.pieces.push(...this.pieces);
    }
    if (this.models && this.models.length > 0) {
      if (!layout.models) layout.models = [];
      layout.models.push(...this.models);
    }
  }

  undo(layout: Layout): void {
    const pieceIds = new Set(this.pieces.map((p) => p.id));
    layout.pieces = layout.pieces.filter((p) => !pieceIds.has(p.id));
    if (this.models && this.models.length > 0 && layout.models) {
      const modelIds = new Set(this.models.map((m) => m.id));
      layout.models = layout.models.filter((m) => !modelIds.has(m.id));
    }
  }
}

export class RemovePieceCommand implements Command {
  private removedPiece: TrackPiece | null = null;
  private index = -1;

  constructor(private pieceId: string) {}

  do(layout: Layout): void {
    this.index = layout.pieces.findIndex((p) => p.id === this.pieceId);
    if (this.index !== -1) {
      this.removedPiece = layout.pieces[this.index]!;
      layout.pieces.splice(this.index, 1);
    }
  }

  undo(layout: Layout): void {
    if (this.removedPiece && this.index !== -1) {
      layout.pieces.splice(this.index, 0, this.removedPiece);
    }
  }
}

export class MovePieceCommand implements Command {
  constructor(
    private pieceId: string,
    private oldTransform: Transform,
    private newTransform: Transform,
  ) {}

  do(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (piece) piece.transform = { ...this.newTransform };
  }

  undo(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (piece) piece.transform = { ...this.oldTransform };
  }
}

export class ToggleFeederCommand implements Command {
  constructor(private pieceId: string) {}

  do(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (piece) piece.powerFeed = !piece.powerFeed;
  }

  undo(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (piece) piece.powerFeed = !piece.powerFeed;
  }
}

export class ToggleIsolatorCommand implements Command {
  private prevIsIsolator?: boolean;
  private prevIsoIndex?: number;
  private targetIndex?: number;

  constructor(private pieceId: string, targetIndex?: number) {
    this.targetIndex = targetIndex;
  }

  do(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (!piece) return;
    this.prevIsIsolator = piece.isIsolator;
    this.prevIsoIndex = piece.isolatedConnectorIndex;

    const numConnectors = Math.max(piece.connectors?.length ?? 2, 2);

    if (this.targetIndex !== undefined) {
      piece.isIsolator = true;
      piece.isolatedConnectorIndex = this.targetIndex;
      return;
    }

    if (!piece.isIsolator) {
      piece.isIsolator = true;
      piece.isolatedConnectorIndex = 0;
    } else {
      const nextIdx = (piece.isolatedConnectorIndex ?? 0) + 1;
      if (nextIdx >= numConnectors) {
        piece.isIsolator = false;
        piece.isolatedConnectorIndex = 0;
      } else {
        piece.isIsolator = true;
        piece.isolatedConnectorIndex = nextIdx;
      }
    }
  }

  undo(layout: Layout): void {
    const piece = layout.pieces.find((p) => p.id === this.pieceId);
    if (!piece) return;
    piece.isIsolator = this.prevIsIsolator;
    piece.isolatedConnectorIndex = this.prevIsoIndex;
  }
}

export class ToggleCatenaryCommand implements Command {
  private prevStates = new Map<string, boolean>();
  private targetState?: boolean;

  constructor(private pieceIds: string[], targetState?: boolean) {
    this.targetState = targetState;
  }

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    if (pieces.length === 0) return;

    const shouldEnable =
      this.targetState !== undefined ? this.targetState : !pieces.every((p) => !!p.hasCatenary);

    for (const p of pieces) {
      this.prevStates.set(p.id, !!p.hasCatenary);
      p.hasCatenary = shouldEnable;
    }
  }

  undo(layout: Layout): void {
    for (const [id, prev] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) p.hasCatenary = prev;
    }
  }
}

export class ToggleCatenaryFeederCommand implements Command {
  private prevStates = new Map<string, { catenaryPowerFeed?: boolean; hasCatenary?: boolean }>();
  private targetState?: boolean;

  constructor(private pieceIds: string[], targetState?: boolean) {
    this.targetState = targetState;
  }

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    if (pieces.length === 0) return;

    const shouldEnable =
      this.targetState !== undefined ? this.targetState : !pieces.every((p) => !!p.catenaryPowerFeed);

    for (const p of pieces) {
      this.prevStates.set(p.id, {
        catenaryPowerFeed: p.catenaryPowerFeed,
        hasCatenary: p.hasCatenary,
      });
      p.catenaryPowerFeed = shouldEnable;
      if (shouldEnable) {
        // Automatically activate catenary if not yet active
        p.hasCatenary = true;
      }
    }
  }

  undo(layout: Layout): void {
    for (const [id, state] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        p.catenaryPowerFeed = state.catenaryPowerFeed;
        p.hasCatenary = state.hasCatenary;
      }
    }
  }
}

export class ToggleCatenaryIsolatorCommand implements Command {
  private prevStates = new Map<
    string,
    { catenaryIsolator?: boolean; catenaryIsolatedConnectorIndex?: number; hasCatenary?: boolean }
  >();
  private targetIndex?: number;

  constructor(private pieceIds: string[], targetIndex?: number) {
    this.targetIndex = targetIndex;
  }

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    if (pieces.length === 0) return;

    for (const p of pieces) {
      this.prevStates.set(p.id, {
        catenaryIsolator: p.catenaryIsolator,
        catenaryIsolatedConnectorIndex: p.catenaryIsolatedConnectorIndex,
        hasCatenary: p.hasCatenary,
      });

      const numConnectors = Math.max(p.connectors?.length ?? 2, 2);

      if (this.targetIndex !== undefined) {
        p.catenaryIsolator = true;
        p.catenaryIsolatedConnectorIndex = this.targetIndex;
        p.hasCatenary = true;
      } else if (!p.catenaryIsolator) {
        p.catenaryIsolator = true;
        p.catenaryIsolatedConnectorIndex = 0;
        p.hasCatenary = true;
      } else {
        const nextIdx = (p.catenaryIsolatedConnectorIndex ?? 0) + 1;
        if (nextIdx >= numConnectors) {
          p.catenaryIsolator = false;
          p.catenaryIsolatedConnectorIndex = 0;
        } else {
          p.catenaryIsolator = true;
          p.catenaryIsolatedConnectorIndex = nextIdx;
          p.hasCatenary = true;
        }
      }
    }
  }

  undo(layout: Layout): void {
    for (const [id, state] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        p.catenaryIsolator = state.catenaryIsolator;
        p.catenaryIsolatedConnectorIndex = state.catenaryIsolatedConnectorIndex;
        p.hasCatenary = state.hasCatenary;
      }
    }
  }
}

export class ToggleTurnoutLanternCommand implements Command {
  private prevStates = new Map<string, boolean | undefined>();

  constructor(private pieceIds: string[]) {}

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    if (pieces.length === 0) return;

    const shouldEnable = !pieces.every((p) => p.hasLantern);
    for (const p of pieces) {
      this.prevStates.set(p.id, p.hasLantern);
      p.hasLantern = shouldEnable;
    }
  }

  undo(layout: Layout): void {
    for (const [id, state] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) p.hasLantern = state;
    }
  }
}

export class ToggleTurnoutDecoderCommand implements Command {
  private prevStates = new Map<string, boolean | undefined>();

  constructor(private pieceIds: string[]) {}

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    if (pieces.length === 0) return;

    const shouldEnable = !pieces.every((p) => p.hasDecoder);
    for (const p of pieces) {
      this.prevStates.set(p.id, p.hasDecoder);
      p.hasDecoder = shouldEnable;
    }
  }

  undo(layout: Layout): void {
    for (const [id, state] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) p.hasDecoder = state;
    }
  }
}

export class SetTurnoutAccessoriesCommand implements Command {
  private prevStates = new Map<string, { hasLantern?: boolean; hasDecoder?: boolean; turnoutCurrentMa?: number }>();

  constructor(
    private pieceIds: string[],
    private settings: { hasLantern?: boolean; hasDecoder?: boolean; turnoutCurrentMa?: number },
  ) {}

  do(layout: Layout): void {
    this.prevStates.clear();
    const pieces = layout.pieces.filter((p) => this.pieceIds.includes(p.id));
    for (const p of pieces) {
      this.prevStates.set(p.id, {
        hasLantern: p.hasLantern,
        hasDecoder: p.hasDecoder,
        turnoutCurrentMa: p.turnoutCurrentMa,
      });
      if (this.settings.hasLantern !== undefined) p.hasLantern = this.settings.hasLantern;
      if (this.settings.hasDecoder !== undefined) p.hasDecoder = this.settings.hasDecoder;
      if (this.settings.turnoutCurrentMa !== undefined) p.turnoutCurrentMa = this.settings.turnoutCurrentMa;
    }
  }

  undo(layout: Layout): void {
    for (const [id, state] of this.prevStates.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        p.hasLantern = state.hasLantern;
        p.hasDecoder = state.hasDecoder;
        p.turnoutCurrentMa = state.turnoutCurrentMa;
      }
    }
  }
}

export class MovePiecesToLayerCommand implements Command {
  private prevStates: {
    pieceId: string;
    oldLayerId?: string;
    oldElevation?: { start: number; end: number };
  }[] = [];

  constructor(
    private pieceIds: string[],
    private targetLayerId: string,
    private newElevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ) {}

  do(layout: Layout): void {
    this.prevStates = [];
    for (const id of this.pieceIds) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece) {
        this.prevStates.push({
          pieceId: id,
          oldLayerId: piece.layerId,
          oldElevation: piece.elevation ? { ...piece.elevation } : undefined,
        });
        piece.layerId = this.targetLayerId;
        if (this.newElevation instanceof Map) {
          const elev = this.newElevation.get(id);
          if (elev) {
            piece.elevation = { ...elev };
          }
        } else if (this.newElevation !== undefined) {
          piece.elevation = { ...this.newElevation };
        }
      } else if (layout.models) {
        const model = layout.models.find((m) => m.id === id);
        if (model) {
          this.prevStates.push({
            pieceId: id,
            oldLayerId: model.layerId,
            oldElevation: undefined,
          });
          model.layerId = this.targetLayerId;
        }
      }
    }
  }

  undo(layout: Layout): void {
    for (const prev of this.prevStates) {
      const piece = layout.pieces.find((p) => p.id === prev.pieceId);
      if (piece) {
        piece.layerId = prev.oldLayerId;
        piece.elevation = prev.oldElevation ? { ...prev.oldElevation } : undefined;
      } else if (layout.models) {
        const model = layout.models.find((m) => m.id === prev.pieceId);
        if (model) {
          model.layerId = prev.oldLayerId;
        }
      }
    }
  }
}

export class AddModelCommand implements Command {
  constructor(private model: Model3D) {}

  do(layout: Layout): void {
    if (!layout.models) layout.models = [];
    layout.models.push(this.model);
  }

  undo(layout: Layout): void {
    if (layout.models) {
      layout.models = layout.models.filter((m) => m.id !== this.model.id);
    }
  }
}

export class RemoveModelCommand implements Command {
  private removedModel: Model3D | null = null;
  private index = -1;

  constructor(private modelId: string) {}

  do(layout: Layout): void {
    if (!layout.models) return;
    this.index = layout.models.findIndex((m) => m.id === this.modelId);
    if (this.index !== -1) {
      this.removedModel = layout.models[this.index]!;
      layout.models.splice(this.index, 1);
    }
  }

  undo(layout: Layout): void {
    if (layout.models && this.removedModel && this.index !== -1) {
      layout.models.splice(this.index, 0, this.removedModel);
    }
  }
}

export class MoveModelCommand implements Command {
  constructor(
    private modelId: string,
    private oldTransform: Transform,
    private newTransform: Transform,
  ) {}

  do(layout: Layout): void {
    const model = layout.models?.find((m) => m.id === this.modelId);
    if (model) model.transform = { ...this.newTransform };
  }

  undo(layout: Layout): void {
    const model = layout.models?.find((m) => m.id === this.modelId);
    if (model) model.transform = { ...this.oldTransform };
  }
}

export class MoveBaseboardCommand implements Command {
  constructor(
    private boardId: string,
    private oldPos: { positionMode: "center" | "top-left" | "custom"; customX: number; customY: number },
    private newPos: { positionMode: "center" | "top-left" | "custom"; customX: number; customY: number },
  ) {}

  do(layout: Layout): void {
    const board = getBaseboards(layout).find((b) => b.id === this.boardId);
    if (board) {
      board.positionMode = this.newPos.positionMode;
      board.customX = this.newPos.customX;
      board.customY = this.newPos.customY;
    }
  }

  undo(layout: Layout): void {
    const board = getBaseboards(layout).find((b) => b.id === this.boardId);
    if (board) {
      board.positionMode = this.oldPos.positionMode;
      board.customX = this.oldPos.customX;
      board.customY = this.oldPos.customY;
    }
  }
}

