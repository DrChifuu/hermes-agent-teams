(function () {
  'use strict';

  if (window.__HERMES_AGENT_TEAMS_MOUNTED__) return;
  window.__HERMES_AGENT_TEAMS_MOUNTED__ = true;

  const EXT_ID = 'hermes-agent-teams';
  const DIRECT_BASE = 'http://127.0.0.1:17890';

  function getAppBasePath() {
    // Strip `/session/<id>` suffix while preserving any reverse-proxy subpath like `/hermes/`.
    const path = (window.location.pathname || '/').replace(/\/session\/[^/?#]+.*$/, '');
    return path.endsWith('/') ? path : `${path}/`;
  }

  function getProxyBase() {
    return `${getAppBasePath()}api/extensions/${EXT_ID}/sidecar`;
  }

  const STRINGS = {
    es: {
      title: 'Agent Teams',
      activeTab: 'Equipo activo',
      archiveTab: 'Archivo',
      liveTab: 'En vivo',
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
      dagHintHorizontal: 'De izquierda a derecha · clic para inspeccionar',
      dagHintVertical: 'De arriba a abajo · clic para inspeccionar',
      dagAuto: 'Auto',
      dagVertical: 'Vertical',
      dagHorizontal: 'Horizontal',
      dagExpandHeight: 'Extender',
      dagCompactHeight: 'Compactar',
      noTasks: 'Aún no hay tareas planificadas.',
      unassigned: 'sin asignar',
      stopTeam: 'Detener equipo',
      confirmStop: '¿Confirmar detención?',
      deleteTeam: 'Eliminar',
      confirmDelete: '¿Confirmar borrado?',
      modelDetecting: 'detectando modelo…',
      chatPlanReady: 'Esquema de equipo listo para aprobar',
      chatReviewApprove: 'Revisar y aprobar',
      chatOpenTeam: 'Abrir panel',
      chatTeamRunning: 'Equipo activo en esta sesión',
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
      liveTitle: 'Agentes en segundo plano',
      liveSubtitle: 'Actividad real de herramientas, subagentes y buzón P2P',
      liveAll: 'Todos',
      liveTools: 'herramientas',
      liveRunning: 'Ejecutando',
      liveIdle: 'En espera',
      liveSucceeded: 'Turno listo',
      liveFailed: 'Error',
      liveCancelled: 'Detenido',
      liveNoEvents: 'Sin eventos en segundo plano todavía. En cuanto un subagente ejecute herramientas o envíe mensajes aparecerán aquí en tiempo real.',
      liveWorkingOn: 'Ejecutando ahora',
      liveLastActivity: 'Última actividad',
      liveNoTeam: 'Selecciona o inicia un equipo para observar a los agentes en segundo plano.',
    },
    en: {
      title: 'Agent Teams',
      activeTab: 'Active Team',
      archiveTab: 'Archive',
      liveTab: 'Live',
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
      dagHintHorizontal: 'Left to right · click a node to inspect',
      dagHintVertical: 'Top to bottom · click a node to inspect',
      dagAuto: 'Auto',
      dagVertical: 'Vertical',
      dagHorizontal: 'Horizontal',
      dagExpandHeight: 'Expand',
      dagCompactHeight: 'Compact',
      noTasks: 'No tasks planned yet.',
      unassigned: 'unassigned',
      stopTeam: 'Stop team',
      confirmStop: 'Confirm stop?',
      deleteTeam: 'Delete',
      confirmDelete: 'Confirm delete?',
      modelDetecting: 'detecting model…',
      chatPlanReady: 'Team plan ready for approval',
      chatReviewApprove: 'Review & Approve',
      chatOpenTeam: 'Open panel',
      chatTeamRunning: 'Active team in this session',
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
      liveTitle: 'Background Agents',
      liveSubtitle: 'Real-time tool calls, subagent status, and P2P mailbox stream',
      liveAll: 'All',
      liveTools: 'tools',
      liveRunning: 'Running',
      liveIdle: 'Idle',
      liveSucceeded: 'Turn finished',
      liveFailed: 'Failed',
      liveCancelled: 'Cancelled',
      liveNoEvents: 'No background events recorded yet. Tool calls, lifecycle transitions, and P2P messages will appear here live.',
      liveWorkingOn: 'Running now',
      liveLastActivity: 'Last activity',
      liveNoTeam: 'Select or start a team to watch agents working in the background.',
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

  function formatTime(ms) {
    if (!ms) return '';
    try {
      const d = new Date(ms);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    } catch (_e) {
      return '';
    }
  }

  function formatRelative(ms) {
    if (!ms) return '';
    const diff = Math.max(0, Math.floor((Date.now() - ms) / 1000));
    if (diff < 5) return getLang() === 'es' ? 'ahora' : 'now';
    if (diff < 60) return `${diff}s`;
    const mins = Math.floor(diff / 60);
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    return `${hrs}h`;
  }

  function formatRoute(provider, model) {
    const m = String(model || '').trim();
    const p = String(provider || '').trim();
    if (!m) return '';
    if (m.startsWith('@') && m.includes(':')) {
      const idx = m.indexOf(':');
      return `${m.slice(1, idx)}/${m.slice(idx + 1)}`;
    }
    if (m.includes('/')) return m;
    return p ? `${p}/${m}` : m;
  }

  const uiState = {
    open: false,
    floating: false,
    tab: 'active', // 'active' | 'archive' | 'live'
    selectedTeamId: null,
    membersCollapsed: false,
    selectedTaskId: null,
    hoveredTaskId: null,
    dagOrientation: 'auto', // 'auto' | 'vertical' | 'horizontal'
    dagExpanded: false,
    liveMemberFilter: 'all',
    stopArmed: false,
    deleteArmed: false,
    editorDirty: false,
    lastSessionId: '',
    lastPayloadSig: '',
    lastActivitySig: '',
    apiBase: 'proxy',
    payload: { teams: [], archived: [] },
    archivedCount: 0,
    archivedLoaded: false,
    activity: { teamId: '', members: [], events: [] },
    captainRoutes: {}, // sessionId -> { provider, model, fetched }
    sse: null,
    pollTimer: null,
  };

  function detectCurrentSessionId() {
    try {
      // `S` in Hermes WebUI static/ui.js is a top-level `const S`, which lives in global lexical
      // scope rather than on `window.S`, so check bare `S` first.
      const globalS = typeof S !== 'undefined' ? S : window.S;
      if (globalS && globalS.session && globalS.session.session_id) {
        return String(globalS.session.session_id).trim();
      }
    } catch (_e) {}
    try {
      if (typeof _sessionIdFromLocation === 'function') {
        const locSid = _sessionIdFromLocation();
        if (locSid && String(locSid).trim()) return String(locSid).trim();
      }
    } catch (_e) {}
    try {
      const sp = new URLSearchParams(window.location.search || '');
      const fromQuery = sp.get('session') || sp.get('session_id');
      if (fromQuery && fromQuery.trim()) return fromQuery.trim();
    } catch (_e) {}
    const match = (window.location.pathname || '').match(/\/session\/([^/?#]+)/);
    if (match && match[1]) return decodeURIComponent(match[1]);
    const activeRow = document.querySelector('.session-item.active[data-session-id], [data-active-session-id]');
    if (activeRow) {
      return activeRow.getAttribute('data-session-id') || activeRow.getAttribute('data-active-session-id') || '';
    }
    return '';
  }

  async function apiFetch(path, options) {
    const proxyBase = getProxyBase();
    const bases = uiState.apiBase === 'direct' ? [DIRECT_BASE, proxyBase] : [proxyBase, DIRECT_BASE];
    let lastErr = null;
    for (const base of bases) {
      try {
        const res = await fetch(`${base}${path}`, Object.assign({ credentials: 'same-origin' }, options));
        if (res.ok) {
          uiState.apiBase = base === DIRECT_BASE ? 'direct' : 'proxy';
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

  function readWebUiSessionRoute(sid) {
    try {
      const globalS = typeof S !== 'undefined' ? S : window.S;
      if (!globalS) return null;
      const cur = globalS.session;
      if (cur && (!sid || String(cur.session_id || '').trim() === sid)) {
        const m = String(cur.model || cur.resolved_model || cur.last_used_model || '').trim();
        const p = String(cur.model_provider || cur.provider || '').trim();
        if (m) return { provider: p, model: m };
      }
      if (sid && Array.isArray(globalS.sessions)) {
        const match = globalS.sessions.find((x) => x && String(x.session_id || '').trim() === sid);
        if (match) {
          const m = String(match.model || match.last_used_model || '').trim();
          const p = String(match.model_provider || match.provider || '').trim();
          if (m) return { provider: p, model: m };
        }
      }
    } catch (_e) {}
    return null;
  }

  async function ensureCaptainRoute(team) {
    if (!team) return;
    if (team.captainModel) return;
    const sid = String(team.captainSessionId || detectCurrentSessionId() || '').trim();
    if (!sid) return;
    const existing = uiState.captainRoutes[sid];
    if (existing && (existing.model || existing.inFlight || (existing.fetchedAt && Date.now() - existing.fetchedAt < 5000))) {
      return;
    }
    const fromMem = readWebUiSessionRoute(sid);
    if (fromMem && fromMem.model) {
      uiState.captainRoutes[sid] = { provider: fromMem.provider, model: fromMem.model, inFlight: false, fetchedAt: Date.now() };
      return;
    }
    uiState.captainRoutes[sid] = { provider: '', model: '', inFlight: true, fetchedAt: Date.now() };
    try {
      const res = await fetch(
        `${getAppBasePath()}api/session?session_id=${encodeURIComponent(sid)}&messages=0&resolve_model=1`,
        { credentials: 'same-origin' }
      );
      if (!res.ok) {
        uiState.captainRoutes[sid] = { provider: '', model: '', inFlight: false, fetchedAt: Date.now() };
        return;
      }
      const body = await res.json();
      const sess = (body && body.session) || body || {};
      const model = String(sess.model || sess.resolved_model || sess.last_used_model || '').trim();
      const provider = String(sess.model_provider || sess.provider || '').trim();
      uiState.captainRoutes[sid] = { provider, model, inFlight: false, fetchedAt: Date.now() };
      if (model) {
        render();
      }
    } catch (_err) {
      uiState.captainRoutes[sid] = { provider: '', model: '', inFlight: false, fetchedAt: Date.now() };
    }
  }

  function resolveCaptainRoute(team) {
    if (!team) return '';
    if (team.captainModel) {
      return formatRoute(team.captainProvider, team.captainModel);
    }
    const sid = String(team.captainSessionId || detectCurrentSessionId() || '').trim();
    const cached = sid && uiState.captainRoutes[sid];
    if (cached && cached.model) {
      return formatRoute(cached.provider, cached.model);
    }
    const fromMem = readWebUiSessionRoute(sid);
    if (fromMem && fromMem.model) {
      return formatRoute(fromMem.provider, fromMem.model);
    }
    return '';
  }

  async function refreshActivity(team, forceRender) {
    if (!team) {
      uiState.activity = { teamId: '', members: [], events: [] };
      return false;
    }
    try {
      const params = new URLSearchParams({ team_id: team.id });
      if (team.workspace) params.set('workspace', team.workspace);
      if (team.archived) params.set('archived', 'true');
      const data = await apiFetch(`/api/v1/activity?${params.toString()}`);
      const nextAct = {
        teamId: team.id,
        members: Array.isArray(data && data.members) ? data.members : [],
        events: Array.isArray(data && data.events) ? data.events : [],
      };
      const sig = JSON.stringify(nextAct);
      const changed = sig !== uiState.lastActivitySig;
      uiState.activity = nextAct;
      uiState.lastActivitySig = sig;
      if (changed && uiState.tab === 'live' && uiState.open && !forceRender) {
        render();
      }
      return changed;
    } catch (_err) {
      return false;
    }
  }

  async function refreshState(forceRender, opts) {
    try {
      const sid = detectCurrentSessionId();
      if (sid && sid !== uiState.lastSessionId) {
        uiState.lastSessionId = sid;
        uiState.selectedTeamId = null;
        uiState.selectedTaskId = null;
        uiState.editorDirty = false;
        forceRender = true;
      }
      const wantArchived = Boolean(
        (opts && opts.includeArchived) || uiState.tab === 'archive'
      );
      // Lightweight polling (`include_archived=0`) avoids re-serializing all archived teams every 1.5s.
      const data = await apiFetch(
        wantArchived ? '/api/v1/state?include_archived=1' : '/api/v1/state?include_archived=0'
      );
      const nextTeams = (data && data.teams) || [];
      const nextArchived = wantArchived
        ? (data && data.archived) || []
        : uiState.payload.archived || [];
      const nextArchivedCount =
        data && typeof data.archivedCount === 'number'
          ? data.archivedCount
          : nextArchived.length;
      if (wantArchived) uiState.archivedLoaded = true;
      const nextPayload = { teams: nextTeams, archived: nextArchived };
      const nextSig = JSON.stringify({ teams: nextTeams, archivedCount: nextArchivedCount, archLen: nextArchived.length, archSig: wantArchived ? nextArchived : null });
      const changed = nextSig !== uiState.lastPayloadSig;
      uiState.payload = nextPayload;
      uiState.archivedCount = nextArchivedCount;
      uiState.lastPayloadSig = nextSig;

      const autoOpen = extHandle && extHandle.settings ? extHandle.settings.get('auto_open_panel') : true;
      if (autoOpen && !uiState._userClosed && uiState.payload.teams && uiState.payload.teams.length > 0 && !uiState.open) {
        uiState.open = true;
        forceRender = true;
      }

      const activeTeam = currentTeam();
      (uiState.payload.teams || []).forEach((t) => {
        void ensureCaptainRoute(t);
      });
      if (activeTeam) {
        void ensureCaptainRoute(activeTeam);
      }

      let actChanged = false;
      if (uiState.open && (uiState.tab === 'live' || (activeTeam && activeTeam.phase === 'running'))) {
        actChanged = await refreshActivity(activeTeam, true);
      }

      if (!forceRender && !changed && !actChanged) return;

      const editingStaged =
        uiState.tab === 'active' &&
        activeTeam &&
        activeTeam.phase === 'staged' &&
        !activeTeam.archived &&
        (uiState.editorDirty ||
          (panel.contains(document.activeElement) &&
            document.activeElement &&
            document.activeElement.closest('.hat-plan-shell')));
      if (!forceRender && editingStaged) return;

      render();
    } catch (_err) {
      if (forceRender) render();
    }
  }

  function connectSse() {
    if (uiState.sse) {
      uiState.sse.close();
      uiState.sse = null;
    }
    if (uiState.apiBase !== 'direct') {
      return;
    }
    try {
      const url = `${DIRECT_BASE}/api/v1/events`;
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
    if (uiState.open) void refreshState(true);
  });

  const panel = document.createElement('aside');
  panel.className = 'hat-panel';
  panel.id = 'hermes-agent-teams-panel';
  panel.hidden = true;

  const chatBanner = document.createElement('div');
  chatBanner.className = 'hat-chat-banner';
  chatBanner.id = 'hermes-agent-teams-chat-banner';
  chatBanner.hidden = true;

  document.body.appendChild(badge);
  document.body.appendChild(panel);

  function mountChatBanner() {
    const mainChat = document.getElementById('mainChat');
    const composerWrap = document.getElementById('composerWrap');
    if (mainChat && composerWrap && composerWrap.parentNode === mainChat) {
      if (chatBanner.parentNode !== mainChat || chatBanner.nextSibling !== composerWrap) {
        mainChat.insertBefore(chatBanner, composerWrap);
      }
      return true;
    }
    return false;
  }

  function renderChatBanner() {
    if (!mountChatBanner()) {
      chatBanner.hidden = true;
      return;
    }
    const sid = detectCurrentSessionId();
    const activeTeams = uiState.payload.teams || [];
    const sessionTeams = sid
      ? activeTeams.filter((t) => !t.archived && String(t.captainSessionId || '').trim() === sid)
      : [];
    const stagedTeam = sessionTeams.find((t) => t.phase === 'staged');
    const runningTeam = sessionTeams.find((t) => !t.halted && t.phase !== 'staged');
    const target = stagedTeam || (!uiState.open ? runningTeam : null);
    if (!target) {
      chatBanner.hidden = true;
      return;
    }

    const art = window.HermesTeamsArtwork;
    const isStaged = target.phase === 'staged';
    const meta = statusMeta(target);
    const stats = target.stats || {};
    const captainRoute = resolveCaptainRoute(target);
    const shortCap = captainRoute ? window.HermesTeamsDag.shortModel(captainRoute) : tr('modelDetecting');
    const modelChips = [
      `<span class="hat-chat-chip is-captain" title="${esc(captainRoute || '')}"><strong>${esc(tr('captain'))}:</strong> ${esc(shortCap)}</span>`,
      ...(target.members || []).map((m) => {
        const effRoute = formatRoute(m.provider, m.model) || captainRoute;
        const shortM = effRoute ? window.HermesTeamsDag.shortModel(effRoute) : tr('modelDetecting');
        return `<span class="hat-chat-chip" title="${esc(effRoute || '')}"><strong>${esc(m.name)}:</strong> ${esc(shortM)}</span>`;
      }),
    ].join('');

    chatBanner.hidden = false;
    chatBanner.className = `hat-chat-banner${isStaged ? ' is-staged' : ' is-running'}`;
    chatBanner.innerHTML = `
      <div class="hat-chat-banner-main" role="button" tabindex="0" data-action="open-chat-team">
        <img class="hat-chat-banner-img" src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
        <div class="hat-chat-banner-body">
          <div class="hat-chat-banner-top">
            <span class="hat-chat-banner-title">${esc(isStaged ? tr('chatPlanReady') : tr('chatTeamRunning'))}</span>
            <span class="hat-chat-banner-name">${esc(target.name)}</span>
            <span class="hat-status-pill ${meta.cls}">${esc(meta.label)}</span>
            <span class="hat-chat-banner-tasks">${stats.completed || 0}/${stats.total || (target.tasks || []).length} ${esc(tr('tasks'))}</span>
          </div>
          <div class="hat-chat-banner-models">${modelChips}</div>
        </div>
        <button type="button" class="${isStaged ? 'hat-btn-primary' : 'hat-btn-secondary'} hat-chat-banner-cta" data-action="open-chat-team">
          ${esc(isStaged ? tr('chatReviewApprove') : tr('chatOpenTeam'))} 🦀
        </button>
      </div>`;

    const openTarget = () => {
      uiState.open = true;
      uiState._userClosed = false;
      uiState.tab = 'active';
      uiState.selectedTeamId = target.id;
      uiState.deleteArmed = false;
      uiState.stopArmed = false;
      render();
      void refreshState(true);
    };
    chatBanner.querySelectorAll('[data-action="open-chat-team"]').forEach((el) => {
      el.addEventListener('click', (ev) => {
        ev.stopPropagation();
        openTarget();
      });
      if (el.getAttribute('role') === 'button') {
        el.addEventListener('keydown', (ev) => {
          if (ev.key === 'Enter' || ev.key === ' ') {
            ev.preventDefault();
            openTarget();
          }
        });
      }
    });
  }

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && uiState.selectedTaskId) {
      uiState.selectedTaskId = null;
      render();
    }
  });

  // Re-evaluate automatic DAG vertical/horizontal layout on viewport resize
  let resizeTimer = null;
  window.addEventListener('resize', () => {
    if (!uiState.open || uiState.dagOrientation !== 'auto') return;
    if (resizeTimer) window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => {
      if (uiState.open && uiState.tab !== 'live') render();
    }, 120);
  });

  function currentTeam() {
    const activeTeams = uiState.payload.teams || [];
    const archivedTeams = uiState.payload.archived || [];
    const list =
      uiState.tab === 'archive'
        ? archivedTeams
        : uiState.tab === 'live'
          ? activeTeams.length
            ? activeTeams
            : archivedTeams
          : activeTeams;
    if (!Array.isArray(list) || !list.length) return null;
    if (uiState.selectedTeamId) {
      const found = list.find((t) => t.id === uiState.selectedTeamId);
      if (found) return found;
    }
    if (uiState.tab !== 'archive') {
      const sid = detectCurrentSessionId();
      if (sid) {
        const ownActive = list.find((t) => t.captainSessionId === sid && !t.halted);
        if (ownActive) return ownActive;
      }
      // Prefer an active/staged (non-halted) team over a halted one when auto-selecting.
      const nonHalted = list.find((t) => !t.halted);
      if (nonHalted) return nonHalted;
      if (sid) {
        const ownAny = list.find((t) => t.captainSessionId === sid);
        if (ownAny) return ownAny;
      }
    }
    return list[0];
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
    renderChatBanner();
    if (!uiState.open) return;

    const team = currentTeam();
    if (team) void ensureCaptainRoute(team);
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
    const switcherList =
      uiState.tab === 'archive'
        ? archivedTeams
        : uiState.tab === 'live'
          ? activeTeams.concat(archivedTeams)
          : activeTeams;
    const liveActiveCount = activeTeams.reduce(
      (acc, t) => acc + (t.members || []).filter((m) => m.activity === 'working').length,
      0
    );

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
            ${
              team
                ? `<button type="button" class="hat-icon-btn is-danger" data-action="delete" title="${esc(tr('deleteTeam'))}">🗑 ${esc(
                    uiState.deleteArmed ? tr('confirmDelete') : tr('deleteTeam')
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
          ${esc(tr('archiveTab'))} (${uiState.archivedCount || archivedTeams.length})
        </button>
        <button type="button" class="hat-tab hat-tab-live${uiState.tab === 'live' ? ' is-active' : ''}" data-tab="live">
          <span class="hat-live-dot${liveActiveCount > 0 ? ' is-busy' : ''}"></span>
          ${esc(tr('liveTab'))}${liveActiveCount > 0 ? ` (${liveActiveCount})` : ''}
        </button>
        ${
          switcherList.length > 1
            ? `<select class="hat-team-switcher" data-action="switch-team" aria-label="Select team">
                ${switcherList
                  .map(
                    (t) =>
                      `<option value="${esc(t.id)}"${team && t.id === team.id ? ' selected' : ''}>${esc(t.name)} · ${esc(statusMeta(t).label)} (${(t.stats && t.stats.completed) || 0}/${(t.stats && t.stats.total) || 0})</option>`
                  )
                  .join('')}
              </select>`
            : ''
        }
      </div>
      <div class="hat-body" id="hat-body"></div>
    `;

    wireHeaderControls(team);

    const body = panel.querySelector('#hat-body');

    if (uiState.tab === 'live') {
      renderLiveWorkspace(body, team);
      return;
    }

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
        captainRoute: resolveCaptainRoute(team),
        appBasePath: getAppBasePath(),
        onDirtyChange: (isDirty) => {
          uiState.editorDirty = !!isDirty;
        },
        onSaveDraft: async (teamId, draft) => {
          await apiFetch('/api/v1/plan/edit', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(Object.assign({ teamId, workspace: team.workspace }, draft)),
          });
          uiState.editorDirty = false;
          await refreshState(true);
        },
        onApprove: async (teamId) => {
          await apiFetch('/api/v1/plan/approve', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId, workspace: team.workspace }),
          });
          uiState.editorDirty = false;
          await refreshState(true);
        },
        onFeedback: async (teamId) => {
          await apiFetch('/api/v1/plan/feedback', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId, workspace: team.workspace }),
          });
          uiState.editorDirty = false;
          await refreshState(true);
        },
        onDiscard: async (teamId) => {
          await apiFetch('/api/v1/plan/discard', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId, workspace: team.workspace }),
          });
          uiState.editorDirty = false;
          await refreshState(true);
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
    const captainRoute = resolveCaptainRoute(team);

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
        const route = formatRoute(m.provider, m.model) || captainRoute;
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
              <div class="hat-member-state${m.spawnError ? ' is-error' : ''}">${esc(m.currentTask || m.spawnError || (allDone ? tr('statusDone') : m.status))}</div>
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
              <div class="hat-captain-model-row">
                ${
                  captainRoute
                    ? `<span class="hat-model-pill hat-captain-model" title="${esc(captainRoute)}">${esc(window.HermesTeamsDag.shortModel(captainRoute))}</span>`
                    : `<span class="hat-model-pill hat-model-pending">${esc(tr('modelDetecting'))}</span>`
                }
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
            <div class="hat-dag-header-left">
              <span class="hat-dag-title">⎇ ${esc(tr('taskDeps'))}</span>
              <span class="hat-dag-hint" id="hat-dag-hint">${esc(tr('dagHintHorizontal'))}</span>
            </div>
            <div class="hat-dag-controls">
              <div class="hat-seg-toggle" role="group" aria-label="DAG layout">
                <button type="button" class="hat-seg-btn${uiState.dagOrientation === 'auto' ? ' is-active' : ''}" data-dag-orient="auto" title="${esc(tr('dagAuto'))}">${esc(tr('dagAuto'))}</button>
                <button type="button" class="hat-seg-btn${uiState.dagOrientation === 'vertical' ? ' is-active' : ''}" data-dag-orient="vertical" title="${esc(tr('dagVertical'))}">⇅</button>
                <button type="button" class="hat-seg-btn${uiState.dagOrientation === 'horizontal' ? ' is-active' : ''}" data-dag-orient="horizontal" title="${esc(tr('dagHorizontal'))}">⇄</button>
              </div>
              <button type="button" class="hat-seg-btn hat-dag-expand-btn${uiState.dagExpanded ? ' is-active' : ''}" data-action="toggle-dag-expand" title="${esc(uiState.dagExpanded ? tr('dagCompactHeight') : tr('dagExpandHeight'))}">
                ${uiState.dagExpanded ? '⇕ −' : '⇕ +'}
              </button>
            </div>
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

    body.querySelectorAll('[data-dag-orient]').forEach((btn) => {
      btn.addEventListener('click', () => {
        uiState.dagOrientation = btn.getAttribute('data-dag-orient') || 'auto';
        render();
      });
    });

    const expandDagBtn = body.querySelector('[data-action="toggle-dag-expand"]');
    if (expandDagBtn) {
      expandDagBtn.addEventListener('click', () => {
        uiState.dagExpanded = !uiState.dagExpanded;
        render();
      });
    }

    const dagMount = body.querySelector('#hat-dag-mount');
    const dagHintEl = body.querySelector('#hat-dag-hint');
    const mountOpts = {
      selectedTaskId: uiState.selectedTaskId || (selectedTask && selectedTask.id),
      hoveredTaskId: uiState.hoveredTaskId,
      orientation: uiState.dagOrientation,
      expanded: uiState.dagExpanded,
      tr,
      onSelectTask: (tid) => {
        uiState.selectedTaskId = tid;
        render();
      },
      onHoverTask: (tid) => {
        if (uiState.hoveredTaskId !== tid) {
          uiState.hoveredTaskId = tid;
          window.HermesTeamsDag.renderDag(
            dagMount,
            team,
            Object.assign({}, mountOpts, { hoveredTaskId: tid, onHoverTask: () => {} })
          );
        }
      },
    };
    const res = window.HermesTeamsDag.renderDag(dagMount, team, mountOpts);
    if (dagHintEl && res && res.orientation) {
      dagHintEl.textContent =
        res.orientation === 'vertical' ? tr('dagHintVertical') : tr('dagHintHorizontal');
    }
  }

  function renderLiveWorkspace(body, team) {
    const art = window.HermesTeamsArtwork;
    if (!team) {
      body.innerHTML = `
        <div class="hat-empty-state">
          <img src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
          <div><strong>${esc(tr('liveTitle'))}</strong></div>
          <p>${esc(tr('liveNoTeam'))}</p>
        </div>`;
      return;
    }

    const act = uiState.activity && uiState.activity.teamId === team.id ? uiState.activity : { members: [], events: [] };
    const runtimeByName = new Map((act.members || []).map((r) => [r.member, r]));
    const captainRoute = resolveCaptainRoute(team);
    const members = team.members || [];

    const stateLabel = (st) => {
      switch (st) {
        case 'running':
        case 'working':
          return { cls: 'is-running', text: tr('liveRunning') };
        case 'succeeded':
        case 'completed':
          return { cls: 'is-done', text: tr('liveSucceeded') };
        case 'failed':
          return { cls: 'is-halted', text: tr('liveFailed') };
        case 'cancelled':
        case 'interrupted':
          return { cls: 'is-staged', text: tr('liveCancelled') };
        default:
          return { cls: '', text: tr('liveIdle') };
      }
    };

    const cardsHtml = [
      // Captain live card
      (() => {
        const capRt = runtimeByName.get('captain');
        const capBusy = team.phase === 'running' && !team.halted && (team.stats && team.stats.completed < team.stats.total);
        const capPill = stateLabel(capRt ? capRt.state : capBusy ? 'running' : 'idle');
        return `
          <div class="hat-live-card${capBusy ? ' is-active' : ''}">
            <div class="hat-live-card-top">
              <div class="hat-avatar-wrap">
                <img class="hat-avatar${capBusy ? ' is-working' : ' is-idle'}" src="${esc(art.LEAD_ART)}" alt="Captain" />
              </div>
              <div class="hat-live-card-id">
                <div class="hat-member-top">
                  <span class="hat-member-name">${esc(tr('captain'))}</span>
                  ${
                    captainRoute
                      ? `<span class="hat-model-pill" title="${esc(captainRoute)}">${esc(window.HermesTeamsDag.shortModel(captainRoute))}</span>`
                      : `<span class="hat-model-pill hat-model-pending">${esc(tr('modelDetecting'))}</span>`
                  }
                </div>
                <div class="hat-live-sub-id">${esc(team.captainSessionId || 'session')}</div>
              </div>
              <span class="hat-status-pill ${capPill.cls}">${esc(capPill.text)}</span>
            </div>
            <div class="hat-live-card-now">
              ${esc((capRt && (capRt.currentTool || capRt.lastSummary)) || tr('captainSub'))}
            </div>
          </div>`;
      })(),
      ...members.map((m) => {
        const rt = runtimeByName.get(m.name);
        const route = formatRoute(m.provider, m.model) || captainRoute;
        const effectiveState = (rt && rt.state) || (m.spawnError ? 'failed' : m.activity === 'working' ? 'running' : 'idle');
        const pill = stateLabel(effectiveState);
        const imgUrl = art.memberArtUrl(m.name, m.role);
        const actionImg = art.actionArtUrl(effectiveState === 'running' ? 'working' : effectiveState === 'succeeded' ? 'celebrating' : 'idle');
        const currentLine =
          (rt && rt.currentTool
            ? `⚡ ${rt.currentTool}${rt.currentToolDetail ? `: ${rt.currentToolDetail}` : ''}`
            : '') ||
          (rt && rt.error) ||
          m.spawnError ||
          (rt && rt.lastSummary) ||
          m.currentTask ||
          tr('liveIdle');
        const toolCount = (rt && rt.toolCount) || 0;
        const updated = (rt && rt.updatedAt) || 0;

        return `
          <div class="hat-live-card${effectiveState === 'running' ? ' is-active' : ''}${effectiveState === 'failed' ? ' is-failed' : ''}">
            <div class="hat-live-card-top">
              <div class="hat-avatar-wrap${m.unread > 0 ? ' has-unread' : ''}">
                <img class="hat-avatar is-${effectiveState === 'running' ? 'working' : 'idle'}" src="${esc(imgUrl)}" alt="${esc(m.name)}" />
                <img class="hat-action-chip" src="${esc(actionImg)}" alt="${esc(effectiveState)}" />
              </div>
              <div class="hat-live-card-id">
                <div class="hat-member-top">
                  <span class="hat-member-name">${esc(m.name)}</span>
                  <span class="hat-member-role">${esc(m.role)}</span>
                  ${route ? `<span class="hat-model-pill" title="${esc(route)}">${esc(window.HermesTeamsDag.shortModel(route))}</span>` : ''}
                </div>
                <div class="hat-live-sub-id">${esc((rt && rt.subagentId) || m.id || 'pendiente')}</div>
              </div>
              <span class="hat-status-pill ${pill.cls}">${esc(pill.text)}</span>
            </div>
            <div class="hat-live-card-now${effectiveState === 'failed' ? ' is-error' : ''}" title="${esc(currentLine)}">
              ${esc(currentLine)}
            </div>
            <div class="hat-live-card-foot">
              <span>${toolCount} ${esc(tr('liveTools'))} · ${m.completedTasks}/${m.totalTasks} ${esc(tr('done'))}</span>
              ${updated ? `<span>${esc(formatRelative(updated))}</span>` : ''}
            </div>
          </div>`;
      }),
    ].join('');

    const filterNames = ['all', 'captain'].concat(members.map((m) => m.name));
    const filterHtml = filterNames
      .map((name) => {
        const label = name === 'all' ? tr('liveAll') : name === 'captain' ? tr('captain') : name;
        return `<button type="button" class="hat-filter-chip${uiState.liveMemberFilter === name ? ' is-active' : ''}" data-live-filter="${esc(name)}">${esc(label)}</button>`;
      })
      .join('');

    const filteredEvents = (act.events || []).filter(
      (ev) => uiState.liveMemberFilter === 'all' || ev.member === uiState.liveMemberFilter
    );

    const kindBadge = (ev) => {
      if (ev.kind === 'tool_start') return { cls: 'is-running', text: ev.tool || 'tool' };
      if (ev.kind === 'tool_end') return { cls: ev.status === 'error' ? 'is-halted' : 'is-done', text: ev.tool || 'tool' };
      if (ev.kind === 'blocked') return { cls: 'is-halted', text: `⛔ ${ev.tool || 'blocked'}` };
      if (ev.kind === 'failed') return { cls: 'is-halted', text: 'error' };
      if (ev.kind === 'message') return { cls: 'is-staged', text: ev.tool || 'msg' };
      if (ev.kind === 'spawned') return { cls: 'is-running', text: 'spawn' };
      if (ev.kind === 'completed') return { cls: 'is-done', text: 'done' };
      return { cls: '', text: ev.tool || ev.kind || 'event' };
    };

    const feedHtml = filteredEvents.length
      ? filteredEvents
          .map((ev) => {
            const kb = kindBadge(ev);
            const dur = Number.isFinite(ev.durationMs) ? `${ev.durationMs}ms` : '';
            return `
              <div class="hat-live-row kind-${esc(ev.kind || 'event')}${ev.status === 'error' ? ' is-error' : ''}">
                <div class="hat-live-row-head">
                  <span class="hat-live-time">${esc(formatTime(ev.ts))}</span>
                  <span class="hat-live-actor">${esc(ev.member === 'captain' ? tr('captain') : ev.member)}</span>
                  <span class="hat-status-pill hat-live-kind ${kb.cls}">${esc(kb.text)}</span>
                  ${dur ? `<span class="hat-live-dur">${esc(dur)}</span>` : ''}
                </div>
                <div class="hat-live-summary">${esc(ev.summary)}</div>
                ${ev.detail && ev.detail !== ev.summary ? `<div class="hat-live-detail">${esc(ev.detail)}</div>` : ''}
              </div>`;
          })
          .join('')
      : `<div class="hat-empty-dag">${esc(tr('liveNoEvents'))}</div>`;

    body.innerHTML = `
      <div class="hat-live-shell">
        <div class="hat-live-header">
          <div>
            <div class="hat-dag-title">⚡ ${esc(tr('liveTitle'))}</div>
            <div class="hat-dag-hint">${esc(tr('liveSubtitle'))}</div>
          </div>
          <span class="hat-live-count">${filteredEvents.length} eventos</span>
        </div>
        <div class="hat-live-cards">${cardsHtml}</div>
        <div class="hat-live-filters">${filterHtml}</div>
        <div class="hat-live-feed">${feedHtml}</div>
      </div>
    `;

    body.querySelectorAll('[data-live-filter]').forEach((btn) => {
      btn.addEventListener('click', () => {
        uiState.liveMemberFilter = btn.getAttribute('data-live-filter') || 'all';
        render();
      });
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
        if (uiState.tab !== 'live') {
          uiState.selectedTeamId = null;
        }
        uiState.selectedTaskId = null;
        uiState.editorDirty = false;
        uiState.stopArmed = false;
        uiState.deleteArmed = false;
        render();
        if (uiState.tab === 'archive') {
          void refreshState(true, { includeArchived: true });
        } else if (uiState.tab === 'live') {
          void refreshActivity(currentTeam(), false);
        }
      });
    });
    const switcher = panel.querySelector('[data-action="switch-team"]');
    if (switcher) {
      switcher.addEventListener('change', () => {
        uiState.selectedTeamId = switcher.value || null;
        uiState.selectedTaskId = null;
        uiState.editorDirty = false;
        uiState.stopArmed = false;
        uiState.deleteArmed = false;
        render();
        if (uiState.tab === 'archive') {
          void refreshState(true, { includeArchived: true });
        } else if (uiState.tab === 'live') {
          void refreshActivity(currentTeam(), false);
        }
      });
    }
    const stopBtn = panel.querySelector('[data-action="stop"]');
    if (stopBtn && team) {
      stopBtn.addEventListener('click', async () => {
        if (!uiState.stopArmed) {
          uiState.stopArmed = true;
          render();
          return;
        }
        uiState.stopArmed = false;
        uiState.deleteArmed = false;
        await apiFetch('/api/v1/team/halt', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            teamId: team.id,
            workspace: team.workspace,
            reason: 'Stopped from Hermes WebUI',
          }),
        });
        await refreshState(true);
      });
    }
    const deleteBtn = panel.querySelector('[data-action="delete"]');
    if (deleteBtn && team) {
      deleteBtn.addEventListener('click', async () => {
        if (!uiState.deleteArmed) {
          uiState.deleteArmed = true;
          uiState.stopArmed = false;
          render();
          return;
        }
        uiState.deleteArmed = false;
        await apiFetch('/api/v1/team/delete', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            teamId: team.id,
            workspace: team.workspace,
            archived: !!team.archived,
          }),
        });
        uiState.selectedTeamId = null;
        uiState.selectedTaskId = null;
        await refreshState(true);
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
          if (uiState.dagOrientation === 'auto' && uiState.tab !== 'live') {
            render();
          }
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

  // Initial load + adaptive polling:
  // Always keep syncing while tab is visible so staged plans created in another tab/session
  // or session switches via sidebar appear automatically even when the panel starts collapsed.
  function scheduleNextPoll() {
    if (uiState.pollTimer) window.clearTimeout(uiState.pollTimer);
    const hasRunning = (uiState.payload.teams || []).some(
      (t) => !t.halted && t.phase !== 'staged' && (t.stats && t.stats.completed < t.stats.total)
    );
    const fast = hasRunning || (uiState.open && uiState.tab === 'live');
    const delay = document.hidden ? 10000 : fast ? 1500 : uiState.open ? 4000 : 3000;
    uiState.pollTimer = window.setTimeout(async () => {
      if (!document.hidden) {
        await refreshState();
      }
      scheduleNextPoll();
    }, delay);
  }

  // React quickly when user switches chat sessions inside the Hermes SPA
  window.addEventListener('popstate', () => void refreshState(true));
  document.addEventListener('click', (ev) => {
    if (ev.target && ev.target.closest && ev.target.closest('.session-item, [data-session-id]')) {
      window.setTimeout(() => void refreshState(true), 250);
    }
  });

  refreshState().then(() => {
    connectSse();
    scheduleNextPoll();
  });
})();
