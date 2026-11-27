(function () {
  'use strict';

  if (window.__HERMES_AGENT_TEAMS_MOUNTED__) return;
  window.__HERMES_AGENT_TEAMS_MOUNTED__ = true;

  const EXT_ID = 'hermes-agent-teams';
  const PROXY_BASE = `api/extensions/${EXT_ID}/sidecar`;
  const DIRECT_BASE = 'http://127.0.0.1:17890';

  const STRINGS = {
    es: {
      title: 'Agent Teams',
      activeTab: 'Equipo activo',
      archiveTab: 'Archivo',
      captain: 'Capitán',
      captainSub: 'Desglosa · Delega · Consolida',
      totalProgress: 'Progreso total',
      inProgress: 'En progreso',
      waitingDeps: 'Esperando deps',
      delivered: 'Entregadas',
      failed: 'Fallidas',
      allDelivered: 'Todas las tareas han sido entregadas',
      members: 'miembros',
      messages: 'mensajes',
      done: 'completadas',
      collapse: 'Contraer',
      expand: 'Expandir',
      taskDeps: 'Dependencias de tareas',
      dagHint: 'De izquierda a derecha · clic para inspeccionar',
      noTasks: 'Aún no hay tareas planificadas.',
      unassigned: 'sin asignar',
      stopTeam: 'Detener equipo',
      confirmStop: '¿Confirmar detención?',
      stagedBannerTitle: 'Plan en borrador — revisa antes de ejecutar',
      stagedBannerDesc: 'Ajusta roles, modelos de Hermes o dependencias y pulsa Aprobar y ejecutar.',
      awaitingFeedbackHint: 'Esperando tus cambios en el chat. El Capitán actualizará este mismo borrador.',
      roster: 'Integrantes',
      tasks: 'Tareas',
      addMember: 'Añadir miembro',
      addTask: 'Añadir tarea',
      approveAndRun: 'Aprobar y ejecutar',
      saveDraft: 'Guardar borrador',
      returnToChat: 'Volver al chat',
      discardPlan: 'Descartar',
      confirmDiscard: 'Confirmar descarte',
      inheritCaptainModel: 'Heredar modelo del Capitán',
      unlocks: 'Al completar desbloquea',
      waitingOn: 'Esperando a',
      emptyTitle: 'Sin equipos activos en esta sesión',
      emptyHint: 'Escribe `/agent-teams <objetivo>` en el chat para armar un equipo de cangrejos Ferris.',
      statusStaged: 'En borrador',
      statusRunning: 'En curso',
      statusHalted: 'Detenido',
      statusDone: 'Finalizado',
      statusArchived: 'Archivado',
      dispatched: 'Capitán asignó',
    },
    en: {
      title: 'Agent Teams',
      activeTab: 'Active Team',
      archiveTab: 'Archive',
      captain: 'Captain',
      captainSub: 'Plan · Delegate · Consolidate',
      totalProgress: 'Total progress',
      inProgress: 'In progress',
      waitingDeps: 'Waiting deps',
      delivered: 'Delivered',
      failed: 'Failed',
      allDelivered: 'All tasks delivered',
      members: 'members',
      messages: 'messages',
      done: 'done',
      collapse: 'Collapse',
      expand: 'Expand',
      taskDeps: 'Task Dependencies',
      dagHint: 'Left to right · click a node to inspect',
      noTasks: 'No tasks planned yet.',
      unassigned: 'unassigned',
      stopTeam: 'Stop team',
      confirmStop: 'Confirm stop?',
      stagedBannerTitle: 'Staged Plan — Review before running',
      stagedBannerDesc: 'Adjust roles, Hermes models, or dependencies and click Approve & Run.',
      awaitingFeedbackHint: 'Waiting for your feedback in chat. Captain will revise this same draft.',
      roster: 'Roster',
      tasks: 'Tasks',
      addMember: 'Add member',
      addTask: 'Add task',
      approveAndRun: 'Approve & Run',
      saveDraft: 'Save draft',
      returnToChat: 'Return to chat',
      discardPlan: 'Discard',
      confirmDiscard: 'Confirm discard',
      inheritCaptainModel: 'Inherit Captain model',
      unlocks: 'Unlocks',
      waitingOn: 'Waiting on',
      emptyTitle: 'No active team in this session',
      emptyHint: 'Type `/agent-teams <goal>` in chat to assemble a Ferris worker team.',
      statusStaged: 'Staged',
      statusRunning: 'Running',
      statusHalted: 'Halted',
      statusDone: 'Completed',
      statusArchived: 'Archived',
      dispatched: 'Assigned',
    },
  };

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  const extHandle = window.hermesExt && window.hermesExt.register ? window.hermesExt.register(EXT_ID) : null;

  function getLang() {
    const pref = extHandle && extHandle.settings ? extHandle.settings.get('language') : 'auto';
    if (pref === 'es' || pref === 'en') return pref;
    const nav = (document.documentElement.lang || navigator.language || 'es').toLowerCase();
    return nav.startsWith('es') ? 'es' : 'en';
  }

  function tr(key) {
    const dict = STRINGS[getLang()] || STRINGS.es;
    return dict[key] || STRINGS.en[key] || key;
  }

  const uiState = {
    open: false,
    floating: false,
    tab: 'active',
    membersCollapsed: false,
    selectedTaskId: null,
    hoveredTaskId: null,
    stopArmed: false,
    apiBase: PROXY_BASE,
    payload: { teams: [], archived: [] },
    sse: null,
    pollTimer: null,
  };

  function detectCurrentSessionId() {
    const match = window.location.pathname.match(/\/session\/([^/?#]+)/);
    if (match && match[1]) return decodeURIComponent(match[1]);
    const activeRow = document.querySelector('.session-item.active[data-session-id], [data-active-session-id]');
    if (activeRow) {
      return activeRow.getAttribute('data-session-id') || activeRow.getAttribute('data-active-session-id') || '';
    }
    return '';
  }

  async function apiFetch(path, options) {
    const bases = uiState.apiBase === PROXY_BASE ? [PROXY_BASE, DIRECT_BASE] : [DIRECT_BASE, PROXY_BASE];
    let lastErr = null;
    for (const base of bases) {
      try {
        const res = await fetch(`${base}${path}`, Object.assign({ credentials: 'same-origin' }, options));
        if (res.ok) {
          uiState.apiBase = base;
          return await res.json();
        }
        if (res.status === 403 || res.status === 404) {
          lastErr = new Error(`HTTP ${res.status}`);
          continue;
        }
        const errBody = await res.json().catch(() => ({}));
        throw new Error(errBody.error || `HTTP ${res.status}`);
      } catch (err) {
        lastErr = err;
      }
    }
    throw lastErr || new Error('hermes-teamsd unreachable');
  }

  async function refreshState() {
    try {
      const sid = detectCurrentSessionId();
      const qs = sid ? `?session_id=${encodeURIComponent(sid)}` : '';
      let data = await apiFetch(`/api/v1/state${qs}`);
      // If session-scoped filter is empty, fall back to workspace-wide active teams so the user always sees their team.
      if (sid && (!data.teams || !data.teams.length) && (!data.archived || !data.archived.length)) {
        data = await apiFetch('/api/v1/state');
      }
      uiState.payload = data || { teams: [], archived: [] };

      const autoOpen = extHandle && extHandle.settings ? extHandle.settings.get('auto_open_panel') : true;
      if (autoOpen && !uiState._userClosed && uiState.payload.teams && uiState.payload.teams.length > 0) {
        uiState.open = true;
      }
      render();
    } catch (_err) {
      render();
    }
  }

  function connectSse() {
    if (uiState.sse) {
      uiState.sse.close();
      uiState.sse = null;
    }
    try {
      const url = `${uiState.apiBase}/api/v1/events`;
      const es = new EventSource(url, { withCredentials: true });
      es.addEventListener('team_update', () => {
        void refreshState();
      });
      es.onerror = () => {
        es.close();
        uiState.sse = null;
      };
      uiState.sse = es;
    } catch (_err) {
      uiState.sse = null;
    }
  }

  // Mount root elements once
  const badge = document.createElement('button');
  badge.type = 'button';
  badge.className = 'hat-badge';
  badge.id = 'hermes-agent-teams-badge';
  badge.addEventListener('click', () => {
    uiState.open = !uiState.open;
    uiState._userClosed = !uiState.open;
    render();
  });

  const panel = document.createElement('aside');
  panel.className = 'hat-panel';
  panel.id = 'hermes-agent-teams-panel';
  panel.hidden = true;

  document.body.appendChild(badge);
  document.body.appendChild(panel);

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && uiState.selectedTaskId) {
      uiState.selectedTaskId = null;
      render();
    }
  });

  function currentTeam() {
    const list = uiState.tab === 'archive' ? uiState.payload.archived : uiState.payload.teams;
    return Array.isArray(list) && list.length ? list[0] : null;
  }

  function statusMeta(team) {
    if (team.archived) return { cls: 'is-done', label: tr('statusArchived') };
    if (team.phase === 'staged') return { cls: 'is-staged', label: tr('statusStaged') };
    if (team.halted) return { cls: 'is-halted', label: tr('statusHalted') };
    const st = team.stats || {};
    if (st.total > 0 && st.completed === st.total) return { cls: 'is-done', label: tr('statusDone') };
    return { cls: 'is-running', label: tr('statusRunning') };
  }

  function render() {
    const art = window.HermesTeamsArtwork;
    const activeTeams = uiState.payload.teams || [];
    const archivedTeams = uiState.payload.archived || [];
    const activeCount = activeTeams.length;
    const busy = activeTeams.some(
      (t) => !t.halted && (t.members || []).some((m) => m.activity === 'working')
    );

    badge.innerHTML = `
      <img class="hat-badge-img" src="${esc(art.LEAD_ART)}" alt="Ferris Captain" />
      <span>${esc(tr('title'))}</span>
      <span class="hat-badge-dot${busy ? ' is-busy' : ''}"></span>
      ${activeCount ? `<span>${activeCount}</span>` : ''}
    `;

    panel.hidden = !uiState.open;
    panel.classList.toggle('is-floating', uiState.floating);
    if (!uiState.open) return;

    const team = currentTeam();
    const meta = team ? statusMeta(team) : null;
    const stats = (team && team.stats) || {
      total: 0,
      completed: 0,
      inProgress: 0,
      waitingDeps: 0,
      failed: 0,
      memberCount: 0,
      messageCount: 0,
    };

    const showStop = team && !team.archived && !team.halted && team.phase !== 'staged';

    panel.innerHTML = `
      <div class="hat-resize-left" data-resize="left"></div>
      <div class="hat-panel-head" data-drag-handle="true">
        <div class="hat-head-top">
          <div class="hat-head-left">
            <img class="hat-badge-img" src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
            <span class="hat-head-title" title="${esc(team ? team.name : tr('title'))}">${esc(team ? team.name : tr('title'))}</span>
            ${meta ? `<span class="hat-status-pill ${meta.cls}">${esc(meta.label)}</span>` : ''}
          </div>
          <div class="hat-head-controls">
            ${
              showStop
                ? `<button type="button" class="hat-icon-btn is-danger" data-action="stop">${esc(
                    uiState.stopArmed ? tr('confirmStop') : tr('stopTeam')
                  )}</button>`
                : ''
            }
            <button type="button" class="hat-icon-btn" data-action="toggle-float" title="Dock / Float">${
              uiState.floating ? '◧' : '❐'
            }</button>
            <button type="button" class="hat-icon-btn" data-action="close" title="Collapse">✕</button>
          </div>
        </div>
        ${
          team
            ? `<div class="hat-head-meta">${stats.memberCount} ${esc(tr('members'))} · ${stats.completed}/${stats.total} ${esc(tr('done'))} · ${stats.messageCount} ${esc(tr('messages'))}</div>`
            : ''
        }
      </div>
      <div class="hat-tabs">
        <button type="button" class="hat-tab${uiState.tab === 'active' ? ' is-active' : ''}" data-tab="active">
          ${esc(tr('activeTab'))} (${activeTeams.length})
        </button>
        <button type="button" class="hat-tab${uiState.tab === 'archive' ? ' is-active' : ''}" data-tab="archive">
          ${esc(tr('archiveTab'))} (${archivedTeams.length})
        </button>
      </div>
      <div class="hat-body" id="hat-body"></div>
    `;

    wireHeaderControls(team);

    const body = panel.querySelector('#hat-body');
    if (!team) {
      body.innerHTML = `
        <div class="hat-empty-state">
          <img src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
          <div><strong>${esc(tr('emptyTitle'))}</strong></div>
          <p>${esc(tr('emptyHint'))}</p>
        </div>`;
      return;
    }

    if (team.phase === 'staged' && !team.archived) {
      window.HermesTeamsPlanEditor.renderPlanEditor(body, team, {
        tr,
        onSaveDraft: async (teamId, draft) => {
          await apiFetch('/api/v1/plan/edit', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(Object.assign({ teamId }, draft)),
          });
          await refreshState();
        },
        onApprove: async (teamId) => {
          await apiFetch('/api/v1/plan/approve', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId }),
          });
          await refreshState();
        },
        onFeedback: async (teamId) => {
          await apiFetch('/api/v1/plan/feedback', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId }),
          });
          await refreshState();
        },
        onDiscard: async (teamId) => {
          await apiFetch('/api/v1/plan/discard', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId }),
          });
          await refreshState();
        },
      });
      return;
    }

    renderActiveWorkspace(body, team);
  }

  function renderActiveWorkspace(body, team) {
    const art = window.HermesTeamsArtwork;
    const stats = team.stats || {};
    const tasks = team.tasks || [];
    const members = team.members || [];
    const allDone = stats.total > 0 && stats.completed === stats.total;

    const segments = tasks
      .map((t) => {
        const isWaiting = t.status === 'pending' && t.waitingOn && t.waitingOn.length > 0;
        const cls = isWaiting ? 'waiting' : t.status;
        return `<span class="hat-seg status-${esc(cls)}" title="${esc(t.id)}: ${esc(t.subject)}"></span>`;
      })
      .join('');

    const memberRows = members
      .map((m) => {
        const imgUrl = art.memberArtUrl(m.name, m.role);
        const actionImg = art.actionArtUrl(allDone ? 'celebrating' : m.activity || 'idle');
        const route = m.provider && m.model ? `${m.provider}/${m.model}` : m.model || '';
        const chips = (m.assignedTaskIds || [])
          .map((tid) => {
            const taskObj = tasks.find((x) => x.id === tid);
            const st = taskObj ? taskObj.status : 'pending';
            return `<button type="button" class="hat-task-chip status-${esc(st)}" data-select-task="${esc(tid)}">${esc(tid)}</button>`;
          })
          .join('');

        return `
          <div class="hat-member-row">
            <div class="hat-avatar-wrap${m.unread > 0 ? ' has-unread' : ''}">
              <img class="hat-avatar is-${esc(m.activity || 'idle')}" src="${esc(imgUrl)}" alt="${esc(m.name)}" />
              <img class="hat-action-chip" src="${esc(actionImg)}" alt="${esc(m.activity || 'idle')}" />
            </div>
            <div class="hat-member-main">
              <div class="hat-member-top">
                <span class="hat-member-name">${esc(m.name)}</span>
                <span class="hat-member-role">${esc(m.role)}</span>
                ${route ? `<span class="hat-model-pill" title="${esc(route)}">${esc(window.HermesTeamsDag.shortModel(route))}</span>` : ''}
              </div>
              <div class="hat-member-state">${esc(m.currentTask || m.spawnError || (allDone ? tr('statusDone') : m.status))}</div>
              ${chips ? `<div class="hat-member-tasks"><span class="hat-member-role">${esc(tr('dispatched'))}</span>${chips}</div>` : ''}
            </div>
            <div class="hat-member-ratio">${m.completedTasks}/${m.totalTasks}</div>
          </div>`;
      })
      .join('');

    const selectedTask =
      tasks.find((t) => t.id === uiState.selectedTaskId) || (tasks.length ? tasks[0] : null);

    body.innerHTML = `
      <div class="hat-grid">
        <div class="hat-left-col">
          <div class="hat-captain-card">
            <div class="hat-avatar-wrap">
              <img class="hat-avatar" src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
            </div>
            <div class="hat-captain-info">
              <div class="hat-captain-title">
                <span>${esc(tr('captain'))}</span>
                <span class="hat-member-role">${esc(tr('captainSub'))}</span>
              </div>
              <div class="hat-captain-sub">
                ${stats.total} ${esc(tr('tasks'))} · ${stats.memberCount} ${esc(tr('members'))}
              </div>
            </div>
          </div>

          <div class="hat-progress-block">
            <div class="hat-progress-label">
              <span>${esc(tr('totalProgress'))}</span>
              <span>${stats.completed}/${stats.total}</span>
            </div>
            <div class="hat-seg-bar">${segments || '<span class="hat-seg"></span>'}</div>
            <div class="hat-legend">
              <span class="hat-legend-item"><i class="hat-dot dot-in_progress"></i>${esc(tr('inProgress'))} ${stats.inProgress}</span>
              <span class="hat-legend-item"><i class="hat-dot dot-waiting"></i>${esc(tr('waitingDeps'))} ${stats.waitingDeps}</span>
              <span class="hat-legend-item"><i class="hat-dot dot-completed"></i>${esc(tr('delivered'))} ${stats.completed}</span>
              ${stats.failed ? `<span class="hat-legend-item"><i class="hat-dot dot-failed"></i>${esc(tr('failed'))} ${stats.failed}</span>` : ''}
            </div>
            ${allDone ? `<div class="hat-all-done-banner">● ${esc(tr('allDelivered'))}</div>` : ''}
          </div>

          <div class="hat-members-header">
            <span>${members.length} ${esc(tr('members'))}</span>
            <button type="button" class="hat-btn-ghost" data-action="toggle-members">
              ${esc(uiState.membersCollapsed ? tr('expand') : tr('collapse'))}
            </button>
          </div>
          ${uiState.membersCollapsed ? '' : `<div class="hat-members-tree">${memberRows}</div>`}
        </div>

        <div class="hat-dag-col">
          <div class="hat-dag-header">
            <span class="hat-dag-title">⎇ ${esc(tr('taskDeps'))}</span>
            <span class="hat-dag-hint">${esc(tr('dagHint'))}</span>
          </div>
          <div id="hat-dag-mount"></div>
          ${selectedTask ? renderTaskInspector(selectedTask) : ''}
        </div>
      </div>
    `;

    body.querySelector('[data-action="toggle-members"]').addEventListener('click', () => {
      uiState.membersCollapsed = !uiState.membersCollapsed;
      render();
    });

    body.querySelectorAll('[data-select-task]').forEach((btn) => {
      btn.addEventListener('click', () => {
        uiState.selectedTaskId = btn.getAttribute('data-select-task');
        render();
      });
    });

    const dagMount = body.querySelector('#hat-dag-mount');
    window.HermesTeamsDag.renderDag(dagMount, team, {
      selectedTaskId: uiState.selectedTaskId || (selectedTask && selectedTask.id),
      hoveredTaskId: uiState.hoveredTaskId,
      tr,
      onSelectTask: (tid) => {
        uiState.selectedTaskId = tid;
        render();
      },
      onHoverTask: (tid) => {
        if (uiState.hoveredTaskId !== tid) {
          uiState.hoveredTaskId = tid;
          window.HermesTeamsDag.renderDag(dagMount, team, {
            selectedTaskId: uiState.selectedTaskId || (selectedTask && selectedTask.id),
            hoveredTaskId: uiState.hoveredTaskId,
            tr,
            onSelectTask: (nextId) => {
              uiState.selectedTaskId = nextId;
              render();
            },
            onHoverTask: () => {},
          });
        }
      },
    });
  }

  function renderTaskInspector(task) {
    const unlocks = task.unlocks && task.unlocks.length ? `${tr('unlocks')}: ${task.unlocks.join(', ')}` : '';
    const waiting = task.waitingOn && task.waitingOn.length ? `${tr('waitingOn')}: ${task.waitingOn.join(', ')}` : '';
    return `
      <div class="hat-task-detail">
        <div class="hat-detail-head">
          <span class="hat-detail-id">${esc(task.id)}</span>
          <span>${esc(task.subject)}</span>
          <span class="hat-status-pill is-${task.status === 'completed' ? 'done' : 'running'}">${esc(task.status)}</span>
        </div>
        <div class="hat-detail-meta">
          <span>${esc(task.assignee || tr('unassigned'))}${task.model ? ` · ${esc(task.model)}` : ''}</span>
          ${waiting ? `<span>${esc(waiting)}</span>` : ''}
          ${unlocks ? `<span>${esc(unlocks)}</span>` : ''}
        </div>
        ${task.output ? `<div class="hat-detail-output">${esc(task.output)}</div>` : ''}
      </div>`;
  }

  function wireHeaderControls(team) {
    panel.querySelector('[data-action="close"]').addEventListener('click', () => {
      uiState.open = false;
      uiState._userClosed = true;
      render();
    });
    panel.querySelector('[data-action="toggle-float"]').addEventListener('click', () => {
      uiState.floating = !uiState.floating;
      render();
    });
    panel.querySelectorAll('[data-tab]').forEach((btn) => {
      btn.addEventListener('click', () => {
        uiState.tab = btn.getAttribute('data-tab');
        uiState.selectedTaskId = null;
        render();
      });
    });
    const stopBtn = panel.querySelector('[data-action="stop"]');
    if (stopBtn && team) {
      stopBtn.addEventListener('click', async () => {
        if (!uiState.stopArmed) {
          uiState.stopArmed = true;
          render();
          return;
        }
        uiState.stopArmed = false;
        await apiFetch('/api/v1/team/halt', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ teamId: team.id, reason: 'Stopped from Hermes WebUI' }),
        });
        await refreshState();
      });
    }

    // Left-edge resize handle
    const resizer = panel.querySelector('[data-resize="left"]');
    if (resizer) {
      resizer.addEventListener('pointerdown', (ev) => {
        ev.preventDefault();
        const startX = ev.clientX;
        const startW = panel.getBoundingClientRect().width;
        const onMove = (m) => {
          const nextW = Math.max(380, Math.min(window.innerWidth - 36, startW + (startX - m.clientX)));
          document.documentElement.style.setProperty('--hat-panel-width', `${Math.round(nextW)}px`);
        };
        const onUp = () => {
          window.removeEventListener('pointermove', onMove);
          window.removeEventListener('pointerup', onUp);
        };
        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', onUp);
      });
    }
  }

  // Subscribe to Hermes WebUI official turn lifecycle hooks
  if (extHandle && extHandle.events && extHandle.events.on) {
    extHandle.events.on('turn:start', () => void refreshState());
    extHandle.events.on('turn:complete', () => void refreshState());
  }

  // Initial load + SSE + gentle safety poll every 6s when panel is open
  refreshState().then(() => connectSse());
  uiState.pollTimer = window.setInterval(() => {
    if (uiState.open || !uiState.sse) void refreshState();
  }, 6000);
})();
