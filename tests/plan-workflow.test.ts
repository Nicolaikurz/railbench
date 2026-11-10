import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { TrackOperations, type EditorContext } from "@/editor/track-operations";
import { PlanManager } from "@/io/plan-manager";
import { History } from "@/editor/commands";
import { Selection } from "@/editor/selection";
import type { Layout, TrackPiece } from "@/core/types";
import { setupDoubleAction } from "@/ui/sidebar";

describe("Plan Workflow & Dirty State", () => {
  let layout: Layout;
  let planManager: PlanManager;
  let history: History;
  let selection: Selection;
  let syncDirtyVal: boolean | undefined;
  let ops: TrackOperations;
  let catalogEntriesMap: Map<string, any>;

  beforeEach(() => {
    catalogEntriesMap = new Map();
    (globalThis as any).confirm = vi.fn().mockReturnValue(true);
    layout = {
      name: "My Plan",
      scale: "H0",
      pieces: [
        {
          id: "piece-1",
          catalogId: "24188",
          layerId: "default",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [],
        },
      ],
      layers: [{ id: "default", name: "Default", color: "#3b82f6", visible: true, locked: false, heightMm: 0 }],
      baseboards: [],
      models: [],
    };
    planManager = new PlanManager();
    history = new History(layout);
    selection = new Selection();
    syncDirtyVal = undefined;

    const mockRenderer: any = {
      resetView: vi.fn(),
      render: vi.fn(),
    };

    const ctx: EditorContext = {
      getLayout: () => layout,
      setLayout: (l: Layout) => { layout = l; },
      getCatalog: () => ({ id: "test", name: "Test", manufacturer: "Märklin", system: "K", scale: "H0", gauge: 16.5, entries: [] }),
      getSelection: () => selection,
      getHistory: () => history,
      setHistory: (h: History) => { history = h; },
      getRenderer: () => mockRenderer,
      getPlanManager: () => planManager,
      isSnappingEnabled: () => true,
      isPieceInteractive: () => true,
      isModelInteractive: () => true,
      getCatalogEntry: (p) => catalogEntriesMap.get(p.catalogId),
      sync: (dirty?: boolean) => {
        syncDirtyVal = dirty;
        if (dirty) planManager.markDirty(true);
      },
      showToast: vi.fn(),
    };

    ops = new TrackOperations(ctx);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("should not prompt confirm if plan is saved (not dirty) and reset cleanly", () => {
    planManager.setPlanInfo("Saved Plan.railbench", null, "railbench");
    expect(planManager.getState().isDirty).toBe(false);

    const confirmSpy = vi.spyOn(globalThis, "confirm").mockReturnValue(false);

    ops.triggerNewPlan();

    // confirm should NOT have been called because isDirty is false
    expect(confirmSpy).not.toHaveBeenCalled();
    expect(layout.pieces.length).toBe(0);
    expect(layout.models?.length).toBe(0);
    expect(syncDirtyVal).toBe(false);
    expect(planManager.getState().isDirty).toBe(false);
  });

  it("removes buildings and 3D models when creating a new plan", () => {
    layout.models = [
      {
        id: "station-1",
        builtinId: "station-standard",
        name: "Bahnhof",
        category: "buildings",
        transform: { x: 100, y: 100, rotation: 0 },
        width: 200,
        depth: 70,
        height: 90,
        modelType: "builtin",
      },
    ];
    layout.pieces = [];
    planManager.setPlanInfo("Plan With Buildings.railbench", null, "railbench");

    ops.triggerNewPlan();

    expect(layout.pieces.length).toBe(0);
    expect(layout.models?.length).toBe(0);
  });

  it("should not prompt confirm if plan is empty even if marked dirty", () => {
    layout.pieces = [];
    planManager.markDirty(true);
    expect(planManager.getState().isDirty).toBe(true);

    const confirmSpy = vi.spyOn(globalThis, "confirm").mockReturnValue(false);

    ops.triggerNewPlan();

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(layout.pieces.length).toBe(0);
    expect(syncDirtyVal).toBe(false);
    expect(planManager.getState().isDirty).toBe(false);
  });

  it("should prompt confirm if plan is dirty and has pieces; cancel should abort", () => {
    planManager.markDirty(true);
    expect(planManager.getState().isDirty).toBe(true);
    expect(layout.pieces.length).toBe(1);

    const confirmSpy = vi.spyOn(globalThis, "confirm").mockReturnValue(false);

    ops.triggerNewPlan();

    expect(confirmSpy).toHaveBeenCalled();
    // Layout should NOT have been cleared
    expect(layout.pieces.length).toBe(1);
  });

  it("should prompt confirm if plan is dirty and has pieces; OK should proceed and reset dirty state", () => {
    planManager.markDirty(true);
    expect(planManager.getState().isDirty).toBe(true);
    expect(layout.pieces.length).toBe(1);

    const confirmSpy = vi.spyOn(globalThis, "confirm").mockReturnValue(true);

    ops.triggerNewPlan();

    expect(confirmSpy).toHaveBeenCalled();
    expect(layout.pieces.length).toBe(0);
    expect(syncDirtyVal).toBe(false);
    expect(planManager.getState().isDirty).toBe(false);
  });

  it("history.clear() should empty undo and redo stacks", () => {
    expect(history.canUndo()).toBe(false);
    history.run({
      do: (l) => l.pieces.push({} as TrackPiece),
      undo: (l) => l.pieces.pop(),
    });
    expect(history.canUndo()).toBe(true);
    history.clear();
    expect(history.canUndo()).toBe(false);
    expect(history.canRedo()).toBe(false);
  });

  it("addPieceToLayout snaps to open connector of selected piece to build a continuous turn", () => {
    // Start with empty layout
    layout.pieces = [];
    selection.clear();

    const straightEntry = {
      id: "24188",
      name: "Straight",
      category: "Straight",
      kind: "straight" as const,
      description: "Straight",
      length: 188.3,
    };

    const curveEntry = {
      id: "24130",
      name: "Curve 30 deg",
      category: "Curve",
      kind: "curve" as const,
      description: "Curve R1 30 deg",
      radius: 360,
      angle: 30,
    };

    // 1. Add straight track at (0, 0)
    ops.addPieceToLayout(straightEntry);
    expect(layout.pieces.length).toBe(1);
    const p1 = layout.pieces[0]!;
    expect(selection.all()).toEqual([p1.id]);
    expect(p1.connectors[1]!.x).toBeCloseTo(188.3, 1);

    // 2. Add first curve track (clicking + in catalog while straight is selected)
    ops.addPieceToLayout(curveEntry);
    expect(layout.pieces.length).toBe(2);
    const p2 = layout.pieces[1]!;
    expect(selection.all()).toEqual([p2.id]);
    // p2 entrance should be snapped to p1 exit (188.3, 0)
    expect(p2.transform.x).toBeCloseTo(188.3, 1);
    expect(p2.transform.y).toBeCloseTo(0, 1);
    expect(p2.transform.rotation).toBeCloseTo(0, 1);
    // Connectors should be linked
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(p2.id);
    expect(p2.connectors[0]!.connectedTo?.pieceId).toBe(p1.id);

    // 3. Add second curve track (clicking + again)
    ops.addPieceToLayout(curveEntry);
    expect(layout.pieces.length).toBe(3);
    const p3 = layout.pieces[2]!;
    expect(selection.all()).toEqual([p3.id]);
    // p3 entrance should be snapped to p2 exit and rotation should be 30 deg
    expect(p3.transform.rotation).toBeCloseTo(30, 1);
    expect(p2.connectors[1]!.connectedTo?.pieceId).toBe(p3.id);
    expect(p3.connectors[0]!.connectedTo?.pieceId).toBe(p2.id);

    // 4. Add third curve track (clicking + again)
    ops.addPieceToLayout(curveEntry);
    expect(layout.pieces.length).toBe(4);
    const p4 = layout.pieces[3]!;
    expect(selection.all()).toEqual([p4.id]);
    expect(p4.transform.rotation).toBeCloseTo(60, 1);
  });

  it("checks the turn of the track and builds continuous right turns", () => {
    layout.pieces = [];
    selection.clear();

    const turnoutRightEntry = {
      id: "24612",
      name: "Turnout Right",
      category: "Turnouts",
      kind: "turnout-right" as const,
      description: "Turnout Right R2 24.3 deg",
      length: 188.3,
      branchRadius: 437.5,
      branchAngle: 24.3,
    };

    const curveEntry = {
      id: "24130",
      name: "Curve 30 deg",
      category: "Curve",
      kind: "curve" as const,
      description: "Curve R1 30 deg",
      radius: 360,
      angle: 30,
    };

    catalogEntriesMap.set("24612", turnoutRightEntry);
    catalogEntriesMap.set("24130", curveEntry);

    // 1. Add turnout right
    ops.addPieceToLayout(turnoutRightEntry);
    const turnout = layout.pieces[0]!;
    expect(selection.all()).toEqual([turnout.id]);

    // 2. Add curve track while turnout is selected -> should snap to curved branch (index 2) and turn right (-24.3 deg)
    ops.addPieceToLayout(curveEntry);
    expect(layout.pieces.length).toBe(2);
    const c1 = layout.pieces[1]!;
    expect(selection.all()).toEqual([c1.id]);
    expect(turnout.connectors[2]!.connectedTo?.pieceId).toBe(c1.id);
    const c1ExitAngle = c1.connectors[0]!.angle;
    expect(c1ExitAngle).toBeCloseTo(305.7, 0.5);

    // 3. Add another curve track -> should continue turning right by another 30 deg
    ops.addPieceToLayout(curveEntry);
    expect(layout.pieces.length).toBe(3);
    const c2 = layout.pieces[2]!;
    expect(selection.all()).toEqual([c2.id]);
    expect(c1.connectors[0]!.connectedTo?.pieceId).toBe(c2.id);
    const c2ExitAngle = c2.connectors[0]!.angle;
    expect(c2ExitAngle).toBeCloseTo(275.7, 0.5);
  });

  it("does not snap to random other pieces if selected piece has no empty snap points", () => {
    layout.pieces = [];
    selection.clear();

    const straightEntry = {
      id: "24188",
      name: "Straight",
      category: "Straight",
      kind: "straight" as const,
      description: "Straight",
      length: 188.3,
    };

    catalogEntriesMap.set("24188", straightEntry);

    ops.addPieceToLayout(straightEntry); // p1
    ops.addPieceToLayout(straightEntry); // p2 attached to p1 exit
    ops.addPieceToLayout(straightEntry); // p3 attached to p2 exit

    // Select middle piece p2 (which has 0 empty snap points)
    const p2 = layout.pieces[1]!;
    expect(p2.connectors[0]!.connectedTo).toBeDefined();
    expect(p2.connectors[1]!.connectedTo).toBeDefined();
    selection.set(p2.id);

    // Add another piece: since p2 has NO empty snap points, it should NOT snap to p3 exit
    const p3 = layout.pieces[2]!;
    expect(p3.connectors[1]!.connectedTo).toBeUndefined();

    ops.addPieceToLayout(straightEntry);
    // p3 exit must remain open/unconnected
    expect(p3.connectors[1]!.connectedTo).toBeUndefined();
  });

  it("double-clicking or double-tapping a catalog card triggers addition to the free snap point of selected piece", () => {
    layout.pieces = [];
    selection.clear();

    const roadStraight = {
      id: "road-straight-100",
      name: "Road Straight",
      category: "Roads",
      kind: "straight" as const,
      description: "Road Straight 100mm",
      length: 100,
      isRoad: true,
    };

    catalogEntriesMap.set("road-straight-100", roadStraight);

    // Initial piece added
    ops.addPieceToLayout(roadStraight);
    expect(layout.pieces.length).toBe(1);
    const p1 = layout.pieces[0]!;
    expect(selection.all()).toEqual([p1.id]);

    // Setup mock element with setupDoubleAction
    const listeners: Record<string, ((e: any) => void)[]> = {};
    const mockCard = {
      addEventListener: (type: string, fn: (e: any) => void) => {
        listeners[type] = listeners[type] || [];
        listeners[type].push(fn);
      },
    };
    const trigger = (type: string, evt: any = {}) => {
      for (const fn of listeners[type] || []) {
        fn(evt);
      }
    };

    let addCallbackCalledWith: any = null;
    setupDoubleAction(mockCard, () => {
      addCallbackCalledWith = roadStraight;
      ops.addPieceToLayout(roadStraight);
    });

    // 1) Test desktop double click
    trigger("dblclick", { preventDefault: vi.fn() });
    expect(addCallbackCalledWith).toEqual(roadStraight);
    expect(layout.pieces.length).toBe(2);
    const p2 = layout.pieces[1]!;
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(p2.id);
    expect(p2.connectors[0]!.connectedTo?.pieceId).toBe(p1.id);

    // 2) Test touch double-tap (<350ms)
    addCallbackCalledWith = null;
    trigger("pointerdown", { pointerType: "touch", preventDefault: vi.fn() });
    trigger("pointerdown", { pointerType: "touch", preventDefault: vi.fn() });
    expect(addCallbackCalledWith).toEqual(roadStraight);
    expect(layout.pieces.length).toBe(3);
    const p3 = layout.pieces[2]!;
    expect(p2.connectors[1]!.connectedTo?.pieceId).toBe(p3.id);
    expect(p3.connectors[0]!.connectedTo?.pieceId).toBe(p2.id);
  });
});
