import { useEffect, useState } from 'react';
import { useOverlayStore } from './stores/overlayStore';
import { useSimStore } from './stores/simStore';
import { useCircuitStore } from './stores/circuitStore';
import { useHistoryStore } from './stores/historyStore';
import { appExit, toolbarSetSize } from './ipc/windowIpc';
import { simStart, simPause, simStep, simReset } from './ipc/simulationIpc';
import { saveProject, loadProject } from './ipc/projectIpc';
import type { SimTickPayload } from './ipc/simulationIpc';
import type { RustPin } from './ipc/circuitIpc';
import type { ComponentKind } from './types/components';
import { COMP_DEF } from './types/components';
import ComponentPanel from './panels/ComponentPanel';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, emit } from '@tauri-apps/api/event';
import { save, open } from '@tauri-apps/plugin-dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs';

function kindFromRust(s: string): ComponentKind | null {
  const lower = s.charAt(0).toLowerCase() + s.slice(1);
  return COMP_DEF[lower as ComponentKind] ? (lower as ComponentKind) : null;
}

export default function FloatingToolbar() {
  const editMode = useOverlayStore((s) => s.editMode);
  const positionLocked = useOverlayStore((s) => s.positionLocked);
  const toggleEditMode = useOverlayStore((s) => s.toggleEditMode);
  const togglePositionLock = useOverlayStore((s) => s.togglePositionLock);
  const setEditMode = useOverlayStore((s) => s.setEditMode);

  const simStatus = useSimStore((s) => s.status);
  const setSimStatus = useSimStore((s) => s.setStatus);
  const updateSignals = useSimStore((s) => s.updateSignals);
  const clearSignals = useSimStore((s) => s.clearSignals);

  const [panelOpen, setPanelOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && editMode) { void setEditMode(false); }
      if ((e.key === 'Delete' || e.key === 'Backspace') && editMode) {
        emit('overlay-delete-component');
      }
      if (editMode && e.ctrlKey && e.key === 'z') {
        e.preventDefault();
        if (e.shiftKey) { useHistoryStore.getState().redo(); }
        else { emit('overlay-undo'); }
      }
      if (editMode && e.ctrlKey && e.key === 'y') {
        e.preventDefault();
        emit('overlay-redo');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [editMode, setEditMode]);

  useEffect(() => {
    const unlistenPromise = listen('force-exit-edit-mode', () => {
      useOverlayStore.setState({ editMode: false });
    });
    return () => { unlistenPromise.then((u) => u()); };
  }, []);

  useEffect(() => {
    const p = listen<SimTickPayload>('sim-tick', (event) => {
      updateSignals(event.payload.changed);
    });
    return () => { p.then((u) => u()); };
  }, [updateSignals]);

  const onGrabPointerDown = async (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    if (positionLocked) return;
    e.preventDefault();
    try { await getCurrentWindow().startDragging(); }
    catch (err) { console.error('startDragging failed:', err); }
  };

  const onSave = async () => {
    try {
      const json = await saveProject();
      const path = await save({ filters: [{ name: 'CircuitForge Project', extensions: ['cfproj'] }] });
      if (path) { await writeTextFile(path, json); }
    } catch (e) { console.error('save failed:', e); }
  };

  const onLoad = async () => {
    try {
      const path = await open({ filters: [{ name: 'CircuitForge Project', extensions: ['cfproj'] }] });
      if (!path) return;
      const json = await readTextFile(path as string);
      const result = await loadProject(json);

      interface LoadedComp { id: number; kind: ComponentKind; x: number; y: number; rustId: number; rustPins: { in: RustPin[]; out: RustPin[] }; }
      const loadedComps: LoadedComp[] = [];
      for (const c of result.components) {
        const k = kindFromRust(c.kind);
        if (!k) continue;
        const inPins: RustPin[] = [];
        const outPins: RustPin[] = [];
        for (const pid of c.input_pins) {
          const p = result.pins.find((x) => x.id === pid);
          if (p) inPins.push({ id: pid, offsetX: p.offset_x, offsetY: p.offset_y });
        }
        for (const pid of c.output_pins) {
          const p = result.pins.find((x) => x.id === pid);
          if (p) outPins.push({ id: pid, offsetX: p.offset_x, offsetY: p.offset_y });
        }
        loadedComps.push({ id: c.id, kind: k, x: c.x, y: c.y, rustId: c.id, rustPins: { in: inPins, out: outPins } });
      }

      const pinToComp = new Map<number, { compId: number; pinIndex: number }>();
      for (const comp of loadedComps) {
        comp.rustPins.in.forEach((p, i) => pinToComp.set(p.id, { compId: comp.id, pinIndex: i }));
        comp.rustPins.out.forEach((p, i) => pinToComp.set(p.id, { compId: comp.id, pinIndex: i }));
      }

      interface LoadedWire { id: number; fromComponentId: number; fromPinIndex: number; toComponentId: number; toPinIndex: number; rustId: number; netId: number; }
      const loadedWires: LoadedWire[] = [];
      for (const w of result.wires) {
        const fromPin = w.start.Pin;
        const toPin = w.end.Pin;
        if (fromPin == null || toPin == null) continue;
        const from = pinToComp.get(fromPin);
        const to = pinToComp.get(toPin);
        if (!from || !to) continue;
        loadedWires.push({ id: w.id, fromComponentId: from.compId, fromPinIndex: from.pinIndex, toComponentId: to.compId, toPinIndex: to.pinIndex, rustId: w.id, netId: w.net_id });
      }

      useCircuitStore.setState({
        components: loadedComps,
        wires: loadedWires,
        nextId: loadedComps.length > 0 ? Math.max(...loadedComps.map((c) => c.id)) + 1 : 1,
        nextWireId: loadedWires.length > 0 ? Math.max(...loadedWires.map((w) => w.id)) + 1 : 1,
        placingKind: null,
        placingWire: null,
        selectedId: null,
      });
      useSimStore.getState().clearSignals();
    } catch (e) { console.error('load failed:', e); }
  };

  const togglePanel = () => {
    const next = !panelOpen;
    setPanelOpen(next);
    toolbarSetSize(next ? 420 : 380, next ? 360 : 56).catch((e) => console.error(e));
  };

  const onPickComponent = (kind: ComponentKind) => {
    emit('overlay-pick-component', kind);
    setPanelOpen(false);
    toolbarSetSize(380, 56).catch((e) => console.error(e));
  };

  const canRun = simStatus === 'stopped' || simStatus === 'paused';
  const isActive = simStatus === 'running';

  return (
    <div className="toolbar-root">
      <div className="tb-row">
        <div className={'tb-grab' + (positionLocked ? ' disabled' : '')} onPointerDown={onGrabPointerDown} title={positionLocked ? '位置已锁定' : '拖动'}>⋮⋮</div>
        <button className={'tb-btn' + (positionLocked ? ' active' : '')} onClick={togglePositionLock} title={positionLocked ? '解锁' : '锁定'}>{positionLocked ? '📌' : '📍'}</button>
        <button className={'tb-btn' + (editMode ? ' active' : '')} onClick={() => { void toggleEditMode(); }} title={editMode ? '退出编辑 (Esc)' : '进入编辑'}>{editMode ? '✏️' : '🖱️'}</button>
        {editMode && (
          <>
            <div className="tb-sep" />
            <button className={'tb-btn' + (panelOpen ? ' active' : '')} onClick={togglePanel} title="元件库 Components">🧩</button>
            <button className="tb-btn" onClick={onSave} title="保存 Save">💾</button>
            <button className="tb-btn" onClick={onLoad} title="加载 Load">📂</button>
            <div className="tb-sep" />
            <button className={'tb-btn' + (isActive ? ' active' : '')} onClick={() => { if (canRun) { setSimStatus('running'); simStart().catch((e) => console.error(e)); } }} title="运行 Run" disabled={!canRun}>▶</button>
            <button className="tb-btn" onClick={() => { setSimStatus('paused'); simPause().catch((e) => console.error(e)); }} title="暂停 Pause" disabled={!isActive}>⏸</button>
            <button className="tb-btn" onClick={() => { simStep().then((r) => updateSignals(r)).catch((e) => console.error(e)); }} title="单步 Step" disabled={isActive}>⏭</button>
            <button className="tb-btn" onClick={() => { setSimStatus('stopped'); clearSignals(); simReset().catch((e) => console.error(e)); }} title="重置 Reset">↺</button>
          </>
        )}
        <span className="tb-title">{editMode ? (isActive ? 'Simulating' : 'Editing') : 'CircuitForge'}</span>
        <button className="tb-btn tb-exit" onClick={() => { void appExit(); }} title="Exit">✕</button>
      </div>
      {panelOpen && <ComponentPanel onPick={onPickComponent} />}
    </div>
  );
}
