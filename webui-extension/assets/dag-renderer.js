(function () {
  'use strict';

  const H_NODE_W = 196;
  const H_NODE_H = 76;
  const H_COL_GAP = 42;
  const H_ROW_GAP = 14;

  const V_NODE_MIN_W = 190;
  const V_NODE_MAX_W = 360;
  const V_NODE_H = 74;
  const V_COL_GAP = 16;
  const V_ROW_GAP = 32;

  const PAD_X = 14;
  const PAD_Y = 14;

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function shortModel(route) {
    let s = String(route || '').trim();
    if (!s) return '';
    // Handle Hermes `@provider:model` syntax (e.g. `@openai-codex:gpt-6.2-sol`)
    if (s.startsWith('@') && s.includes(':')) {
      s = s.slice(s.indexOf(':') + 1);
    }
    const idx = s.lastIndexOf('/');
    return idx >= 0 ? s.slice(idx + 1) : s;
  }

  function groupByDepth(tasks) {
    const columns = new Map();
    for (const t of tasks) {
      const d = Number.isFinite(t.depth) ? t.depth : 0;
      if (!columns.has(d)) columns.set(d, []);
      columns.get(d).push(t);
    }
    const depths = Array.from(columns.keys()).sort((a, b) => a - b);
    return { columns, depths };
  }

  function computeHorizontalLayout(tasks) {
    const { columns, depths } = groupByDepth(tasks);
    const nodes = [];
    const posById = new Map();
    let maxRows = 1;

    depths.forEach((depth, colIdx) => {
      const colTasks = columns.get(depth);
      if (colTasks.length > maxRows) maxRows = colTasks.length;
      colTasks.forEach((task, rowIdx) => {
        const x = PAD_X + colIdx * (H_NODE_W + H_COL_GAP);
        const y = PAD_Y + rowIdx * (H_NODE_H + H_ROW_GAP);
        const entry = { task, x, y, w: H_NODE_W, h: H_NODE_H };
        nodes.push(entry);
        posById.set(task.id, entry);
      });
    });

    const edges = [];
    for (const n of nodes) {
      const deps = Array.isArray(n.task.dependencies) ? n.task.dependencies : [];
      for (const depId of deps) {
        const src = posById.get(depId);
        if (!src) continue;
        const x1 = src.x + src.w;
        const y1 = src.y + src.h / 2;
        const x2 = n.x;
        const y2 = n.y + n.h / 2;
        const dx = Math.max(20, (x2 - x1) * 0.55);
        const path = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
        edges.push({ from: depId, to: n.task.id, path });
      }
    }

    const width = Math.max(
      H_NODE_W + PAD_X * 2,
      PAD_X * 2 + depths.length * H_NODE_W + Math.max(0, depths.length - 1) * H_COL_GAP
    );
    const height = Math.max(
      H_NODE_H + PAD_Y * 2,
      PAD_Y * 2 + maxRows * H_NODE_H + Math.max(0, maxRows - 1) * H_ROW_GAP
    );

    return { width, height, nodes, edges, orientation: 'horizontal' };
  }

  function computeVerticalLayout(tasks, availableWidth) {
    const { columns, depths } = groupByDepth(tasks);
    const avail = Math.max(240, Number(availableWidth) || 440);
    let maxCols = 1;
    for (const d of depths) {
      const len = columns.get(d).length;
      if (len > maxCols) maxCols = len;
    }

    // Choose node width so single-lane chains expand cleanly and multi-branch rows stay legible.
    const innerAvail = Math.max(200, avail - PAD_X * 2 - 4);
    const nodeW =
      maxCols === 1
        ? Math.max(V_NODE_MIN_W, Math.min(V_NODE_MAX_W, innerAvail))
        : Math.max(
            V_NODE_MIN_W,
            Math.min(240, Math.floor((innerAvail - (maxCols - 1) * V_COL_GAP) / maxCols))
          );

    const canvasW = Math.max(
      innerAvail + PAD_X * 2,
      PAD_X * 2 + maxCols * nodeW + Math.max(0, maxCols - 1) * V_COL_GAP
    );

    const nodes = [];
    const posById = new Map();

    depths.forEach((depth, rowIdx) => {
      const rowTasks = columns.get(depth);
      const rowW = rowTasks.length * nodeW + Math.max(0, rowTasks.length - 1) * V_COL_GAP;
      const startX = Math.max(PAD_X, Math.round((canvasW - rowW) / 2));
      const y = PAD_Y + rowIdx * (V_NODE_H + V_ROW_GAP);

      rowTasks.forEach((task, colIdx) => {
        const x = startX + colIdx * (nodeW + V_COL_GAP);
        const entry = { task, x, y, w: nodeW, h: V_NODE_H };
        nodes.push(entry);
        posById.set(task.id, entry);
      });
    });

    const edges = [];
    for (const n of nodes) {
      const deps = Array.isArray(n.task.dependencies) ? n.task.dependencies : [];
      for (const depId of deps) {
        const src = posById.get(depId);
        if (!src) continue;
        const x1 = src.x + src.w / 2;
        const y1 = src.y + src.h;
        const x2 = n.x + n.w / 2;
        const y2 = n.y;
        const dy = Math.max(16, Math.abs(y2 - y1) * 0.55);
        const path = `M ${x1} ${y1} C ${x1} ${y1 + dy}, ${x2} ${y2 - dy}, ${x2} ${y2}`;
        edges.push({ from: depId, to: n.task.id, path });
      }
    }

    const height = Math.max(
      V_NODE_H + PAD_Y * 2,
      PAD_Y * 2 + depths.length * V_NODE_H + Math.max(0, depths.length - 1) * V_ROW_GAP
    );

    return { width: canvasW, height, nodes, edges, orientation: 'vertical' };
  }

  function resolveOrientation(tasks, requested, availableWidth) {
    if (requested === 'vertical' || requested === 'horizontal') return requested;
    const avail = Number(availableWidth) || 0;
    if (avail > 0 && avail < 520) return 'vertical';
    const { depths } = groupByDepth(tasks);
    const horizW = PAD_X * 2 + depths.length * H_NODE_W + Math.max(0, depths.length - 1) * H_COL_GAP;
    if (avail > 0 && horizW > avail + 20) return 'vertical';
    return 'horizontal';
  }

  function lineageOf(tasks, targetId) {
    const byId = new Map(tasks.map((t) => [t.id, t]));
    const related = new Set([targetId]);
    const up = [targetId];
    while (up.length) {
      const id = up.pop();
      const t = byId.get(id);
      if (!t || !Array.isArray(t.dependencies)) continue;
      for (const dep of t.dependencies) {
        if (!related.has(dep)) {
          related.add(dep);
          up.push(dep);
        }
      }
    }
    const down = [targetId];
    while (down.length) {
      const id = down.pop();
      for (const t of tasks) {
        if (Array.isArray(t.dependencies) && t.dependencies.includes(id) && !related.has(t.id)) {
          related.add(t.id);
          down.push(t.id);
        }
      }
    }
    return related;
  }

  function renderDag(container, team, opts) {
    const tasks = Array.isArray(team.tasks) ? team.tasks : [];
    const selectedId = opts.selectedTaskId || null;
    const hoveredId = opts.hoveredTaskId || null;
    const activeId = hoveredId || selectedId;
    const related = activeId ? lineageOf(tasks, activeId) : null;
    const tLabel = opts.tr;

    if (!tasks.length) {
      container.innerHTML = `<div class="hat-empty-dag">${esc(tLabel('noTasks'))}</div>`;
      return { orientation: 'horizontal' };
    }

    const availW = container.clientWidth || (container.parentElement && container.parentElement.clientWidth) || 500;
    const effectiveOrientation = resolveOrientation(tasks, opts.orientation || 'auto', availW);
    const layout =
      effectiveOrientation === 'vertical'
        ? computeVerticalLayout(tasks, availW)
        : computeHorizontalLayout(tasks);

    const svgEdges = layout.edges
      .map((e) => {
        const hi = related && related.has(e.from) && related.has(e.to);
        const dim = related && !hi;
        const cls = `hat-dag-edge${hi ? ' is-hi' : ''}${dim ? ' is-dim' : ''}`;
        return `<path class="${cls}" d="${e.path}" />`;
      })
      .join('');

    const nodeCards = layout.nodes
      .map(({ task, x, y, w, h }) => {
        const isSelected = task.id === selectedId;
        const isHi = related ? related.has(task.id) : false;
        const isDim = related ? !isHi : false;
        const isActive = task.status === 'claimed' || task.status === 'in_progress';
        const isWaiting = task.status === 'pending' && task.waitingOn && task.waitingOn.length > 0;
        const statusKey = isWaiting ? 'waiting' : task.status;
        const modelShort = isActive && task.model ? shortModel(task.model) : '';

        return `
          <button
            type="button"
            class="hat-dag-node status-${esc(statusKey)}${isSelected ? ' is-selected' : ''}${isHi ? ' is-hi' : ''}${isDim ? ' is-dim' : ''}"
            style="left:${x}px;top:${y}px;width:${w}px;min-height:${h}px"
            data-task-id="${esc(task.id)}"
            title="${esc(task.subject)}"
          >
            <div class="hat-dag-node-head">
              <span class="hat-dag-id"><i class="hat-dot dot-${esc(statusKey)}"></i>${esc(task.id)}</span>
              <span class="hat-dag-assignee">${esc(task.assignee || tLabel('unassigned'))}</span>
            </div>
            <div class="hat-dag-subject">${esc(task.subject)}</div>
            ${modelShort ? `<div class="hat-dag-model">${esc(modelShort)}</div>` : ''}
          </button>`;
      })
      .join('');

    const expandedCls = opts.expanded ? ' is-expanded' : '';
    const orientCls = effectiveOrientation === 'vertical' ? ' is-vertical' : ' is-horizontal';

    container.innerHTML = `
      <div class="hat-dag-scroll${orientCls}${expandedCls}">
        <div class="hat-dag-canvas" style="width:${layout.width}px;height:${layout.height}px">
          <svg class="hat-dag-svg" width="${layout.width}" height="${layout.height}" viewBox="0 0 ${layout.width} ${layout.height}">
            ${svgEdges}
          </svg>
          ${nodeCards}
        </div>
      </div>`;

    container.querySelectorAll('.hat-dag-node').forEach((btn) => {
      const tid = btn.getAttribute('data-task-id');
      btn.addEventListener('click', () => opts.onSelectTask(tid === selectedId ? null : tid));
      btn.addEventListener('mouseenter', () => opts.onHoverTask(tid));
      btn.addEventListener('mouseleave', () => opts.onHoverTask(null));
    });

    return { orientation: effectiveOrientation };
  }

  window.HermesTeamsDag = {
    renderDag,
    shortModel,
    resolveOrientation,
  };
})();
