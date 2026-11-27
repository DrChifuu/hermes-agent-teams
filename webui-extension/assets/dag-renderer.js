(function () {
  'use strict';

  const NODE_W = 196;
  const NODE_H = 74;
  const COL_GAP = 42;
  const ROW_GAP = 14;
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
    const s = String(route || '').trim();
    if (!s) return '';
    const idx = s.lastIndexOf('/');
    return idx >= 0 ? s.slice(idx + 1) : s;
  }

  function computeLayout(tasks) {
    const columns = new Map();
    for (const t of tasks) {
      const d = Number.isFinite(t.depth) ? t.depth : 0;
      if (!columns.has(d)) columns.set(d, []);
      columns.get(d).push(t);
    }
    const depths = Array.from(columns.keys()).sort((a, b) => a - b);
    const nodes = [];
    const posById = new Map();
    let maxRows = 1;

    depths.forEach((depth, colIdx) => {
      const colTasks = columns.get(depth);
      if (colTasks.length > maxRows) maxRows = colTasks.length;
      colTasks.forEach((task, rowIdx) => {
        const x = PAD_X + colIdx * (NODE_W + COL_GAP);
        const y = PAD_Y + rowIdx * (NODE_H + ROW_GAP);
        const entry = { task, x, y };
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
        const x1 = src.x + NODE_W;
        const y1 = src.y + NODE_H / 2;
        const x2 = n.x;
        const y2 = n.y + NODE_H / 2;
        const dx = Math.max(20, (x2 - x1) * 0.55);
        const path = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
        edges.push({ from: depId, to: n.task.id, path });
      }
    }

    const width = Math.max(
      NODE_W + PAD_X * 2,
      PAD_X * 2 + depths.length * NODE_W + Math.max(0, depths.length - 1) * COL_GAP
    );
    const height = Math.max(
      NODE_H + PAD_Y * 2,
      PAD_Y * 2 + maxRows * NODE_H + Math.max(0, maxRows - 1) * ROW_GAP
    );

    return { width, height, nodes, edges };
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
      return;
    }

    const layout = computeLayout(tasks);
    const svgEdges = layout.edges
      .map((e) => {
        const hi = related && related.has(e.from) && related.has(e.to);
        const dim = related && !hi;
        const cls = `hat-dag-edge${hi ? ' is-hi' : ''}${dim ? ' is-dim' : ''}`;
        return `<path class="${cls}" d="${e.path}" />`;
      })
      .join('');

    const nodeCards = layout.nodes
      .map(({ task, x, y }) => {
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
            style="left:${x}px;top:${y}px;width:${NODE_W}px;min-height:${NODE_H}px"
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

    container.innerHTML = `
      <div class="hat-dag-scroll">
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
  }

  window.HermesTeamsDag = {
    renderDag,
    shortModel,
  };
})();
