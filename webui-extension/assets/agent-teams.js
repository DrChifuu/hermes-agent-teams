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
      escalatedBanner: 'Escalado al Capitán — se alcanzó el límite de rondas de revisión. Requiere intervención.',
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
      chatTeamDone: 'Equipo finalizado — listo para informe final',
      chatAskReport: 'Pedir informe al Capitán',
      chatDismiss: 'Ocultar aviso',
      fullAccessOn: 'Full Access: ON',
      fullAccessOff: 'Full Access',
      fullAccessTip: 'Ejecuta equipos nuevos automáticamente sin pedir aprobación manual ni bloquear herramientas',
      archiveListTitle: 'Equipos archivados',
      archiveEmpty: 'No hay equipos en el archivo.',
      archiveInspect: 'Ver',
      archiveBack: '← Volver a la lista',
      archiveClearAll: 'Vaciar todo',
      archiveConfirmClearAll: '¿Borrar todos?',
      archiveConfirmOne: '¿Borrar?',
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
      effortLabel: 'Esfuerzo de razonamiento',
      effortInherit: 'Esfuerzo: heredar',
      effortNone: 'Sin razonamiento',
      effortMinimal: 'Esfuerzo: mínimo',
      effortLow: 'Esfuerzo: bajo',
      effortMedium: 'Esfuerzo: medio',
      effortHigh: 'Esfuerzo: alto',
      effortXhigh: 'Esfuerzo: muy alto',
      effortMax: 'Esfuerzo: máximo',
      effortUnsupported: 'Sin ajuste de esfuerzo',
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
      escalatedBanner: 'Escalated to Captain — review round limit reached. Requires intervention.',
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
      chatTeamDone: 'Team finished — ready for final report',
      chatAskReport: 'Ask Captain for report',
      chatDismiss: 'Dismiss banner',
      fullAccessOn: 'Full Access: ON',
      fullAccessOff: 'Full Access',
      fullAccessTip: 'Auto-approve new teams and default members to full tool access without manual prompts',
      archiveListTitle: 'Archived teams',
      archiveEmpty: 'No archived teams.',
      archiveInspect: 'Inspect',
      archiveBack: '← Back to list',
      archiveClearAll: 'Clear all',
      archiveConfirmClearAll: 'Delete all?',
      archiveConfirmOne: 'Delete?',
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
      effortLabel: 'Reasoning effort',
      effortInherit: 'Effort: inherit',
      effortNone: 'Reasoning: off',
      effortMinimal: 'Effort: minimal',
      effortLow: 'Effort: low',
      effortMedium: 'Effort: medium',
      effortHigh: 'Effort: high',
      effortXhigh: 'Effort: xhigh',
      effortMax: 'Effort: max',
      effortUnsupported: 'No effort control',
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

  function formatDuration(ms) {
    const totalSec = Math.max(0, Math.floor((ms || 0) / 1000));
    const hrs = Math.floor(totalSec / 3600);
    const mins = Math.floor((totalSec % 3600) / 60);
    const secs = totalSec % 60;
    const pad = (n) => String(n).padStart(2, '0');
    if (hrs > 0) return `${hrs}h ${pad(mins)}m ${pad(secs)}s`;
    if (mins > 0) return `${mins}m ${pad(secs)}s`;
    return `${secs}s`;
  }

  function computeTeamElapsedMs(team) {
    if (!team || team.phase === 'staged') return 0;
    const start = team.approvedAt || team.createdAt || 0;
    if (!start) return 0;
    const active = !team.archived && !team.halted && !isTeamFinished(team);
    if (active) return Math.max(0, Date.now() - start);
    const taskMax = (team.tasks || []).reduce(
      (mx, t) => Math.max(mx, Number(t.updatedAt || 0)),
      0
    );
    const end = Math.max(start, taskMax || start);
    return Math.max(0, end - start);
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
    minimized: false,
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
    archiveDeleteArmedId: null,
    archiveClearAllArmed: false,
    fullAccess: false,
    fullAccessLoaded: false,
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
        const appErr = new Error(errBody.error || `HTTP ${res.status}`);
        appErr._httpAppError = true;
        throw appErr;
      } catch (err) {
        if (err && err._httpAppError) throw err;
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
      if (uiState.open && uiState.tab === 'live') {
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

  // Single persistent delegated click handler on `panel` so polling `innerHTML` refreshes
  // can NEVER swallow a click on ✕ (close), − (minimize), ❐ (float), or header buttons.
  panel.addEventListener('click', async (ev) => {
    const btn = ev.target && ev.target.closest ? ev.target.closest('[data-action], [data-tab]') : null;
    if (!btn || !panel.contains(btn)) return;

    const tab = btn.getAttribute('data-tab');
    if (tab) {
      ev.stopPropagation();
      uiState.tab = tab;
      if (uiState.tab !== 'live') uiState.selectedTeamId = null;
      uiState.selectedTaskId = null;
      uiState.editorDirty = false;
      uiState.stopArmed = false;
      uiState.deleteArmed = false;
      uiState.archiveDeleteArmedId = null;
      uiState.archiveClearAllArmed = false;
      render();
      if (uiState.tab === 'archive') {
        void refreshState(true, { includeArchived: true });
      } else if (uiState.tab === 'live') {
        void refreshActivity(currentTeam(), false);
      }
      return;
    }

    const action = btn.getAttribute('data-action');
    if (action === 'archive-inspect') {
      ev.stopPropagation();
      uiState.selectedTeamId = btn.getAttribute('data-team-id') || null;
      uiState.selectedTaskId = null;
      uiState.archiveDeleteArmedId = null;
      uiState.archiveClearAllArmed = false;
      render();
      return;
    }
    if (action === 'archive-back') {
      ev.stopPropagation();
      uiState.selectedTeamId = null;
      uiState.selectedTaskId = null;
      uiState.deleteArmed = false;
      render();
      return;
    }
    if (action === 'archive-delete-one') {
      ev.stopPropagation();
      const tid = btn.getAttribute('data-team-id') || '';
      const ws = btn.getAttribute('data-workspace') || '';
      if (!tid) return;
      if (uiState.archiveDeleteArmedId !== tid) {
        uiState.archiveDeleteArmedId = tid;
        uiState.archiveClearAllArmed = false;
        render();
        return;
      }
      uiState.archiveDeleteArmedId = null;
      await apiFetch('/api/v1/team/delete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ teamId: tid, workspace: ws || undefined, archived: true }),
      });
      await refreshState(true, { includeArchived: true });
      return;
    }
    if (action === 'archive-clear-all') {
      ev.stopPropagation();
      const archivedList = uiState.payload.archived || [];
      if (!archivedList.length) return;
      if (!uiState.archiveClearAllArmed) {
        uiState.archiveClearAllArmed = true;
        uiState.archiveDeleteArmedId = null;
        render();
        return;
      }
      uiState.archiveClearAllArmed = false;
      for (const t of archivedList) {
        try {
          await apiFetch('/api/v1/team/delete', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ teamId: t.id, workspace: t.workspace || undefined, archived: true }),
          });
        } catch (_e) {}
      }
      uiState.selectedTeamId = null;
      await refreshState(true, { includeArchived: true });
      return;
    }
    if (action === 'close') {
      ev.stopPropagation();
      uiState.open = false;
      uiState.minimized = false;
      uiState._userClosed = true;
      render();
      return;
    }
    if (action === 'minimize') {
      ev.stopPropagation();
      uiState.minimized = !uiState.minimized;
      render();
      return;
    }
    if (action === 'toggle-float') {
      ev.stopPropagation();
      uiState.floating = !uiState.floating;
      uiState.minimized = false;
      render();
      return;
    }
    if (action === 'toggle-full-access') {
      ev.stopPropagation();
      const next = !uiState.fullAccess;
      try {
        const res = await apiFetch('/api/v1/settings', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ fullAccess: next }),
        });
        uiState.fullAccess = !!(res && res.fullAccess);
        uiState.fullAccessLoaded = true;
        render();
      } catch (_e) {}
      return;
    }
    const team = currentTeam();
    if (action === 'stop' && team) {
      ev.stopPropagation();
      if (!uiState.stopArmed) {
        uiState.stopArmed = true;
        uiState.deleteArmed = false;
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
      return;
    }
    if (action === 'delete' && team) {
      ev.stopPropagation();
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
    }
  });

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

  function isTeamFinished(team) {
    if (!team || team.phase === 'staged' || team.halted) return false;
    const st = team.stats || {};
    if (!st.total || st.total <= 0) return false;
    const anyWorking = (team.members || []).some((m) => m.activity === 'working');
    return (
      !anyWorking &&
      (st.inProgress || 0) === 0 &&
      (st.waitingDeps || 0) === 0 &&
      (st.pendingReady || 0) === 0
    );
  }

  function triggerCaptainReport(team) {
    if (!team) return;
    const prompt =
      getLang() === 'es'
        ? `El equipo ${team.name} terminó todas sus tareas. Revisa agent_teams_status y entrégame el informe final consolidado (sin archivar hasta que te confirme).`
        : `Team ${team.name} has finished all tasks. Inspect agent_teams_status and deliver the consolidated final report.`;
    const input =
      document.getElementById('msgInput') ||
      document.querySelector('#composerWrap textarea, textarea[data-testid="chat-input"], textarea');
    const sendBtn =
      document.getElementById('btnSend') ||
      document.querySelector('#composerWrap button[type="submit"], button.send-btn');
    if (input) {
      input.value = prompt;
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.focus();
      if (sendBtn && !sendBtn.disabled) {
        sendBtn.click();
      }
    }
  }

  const DISMISSED_KEY = 'hat_dismissed_banners_v1';

  function getDismissedBanners() {
    try {
      const raw = localStorage.getItem(DISMISSED_KEY);
      const arr = raw ? JSON.parse(raw) : [];
      return new Set(Array.isArray(arr) ? arr : []);
    } catch (_e) {
      return new Set();
    }
  }

  function dismissBannerKey(key) {
    try {
      const set = getDismissedBanners();
      set.add(key);
      const list = Array.from(set).slice(-40);
      localStorage.setItem(DISMISSED_KEY, JSON.stringify(list));
    } catch (_e) {}
  }

  async function refreshSettings() {
    try {
      const data = await apiFetch('/api/v1/settings');
      if (data && typeof data.fullAccess === 'boolean') {
        const changed = data.fullAccess !== uiState.fullAccess || !uiState.fullAccessLoaded;
        uiState.fullAccess = data.fullAccess;
        uiState.fullAccessLoaded = true;
        if (changed && uiState.open) render();
      }
    } catch (_e) {}
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
    const doneTeam = sessionTeams.find((t) => isTeamFinished(t));
    const runningTeam = sessionTeams.find((t) => !t.halted && t.phase !== 'staged' && !isTeamFinished(t));
    const target = stagedTeam || doneTeam || (!uiState.open ? runningTeam : null);
    if (!target) {
      chatBanner.hidden = true;
      return;
    }

    const isStaged = target.phase === 'staged';
    const isDone = isTeamFinished(target);
    const bannerKey = `${target.id}:${isStaged ? 'staged' : isDone ? 'done' : 'running'}`;
    if (getDismissedBanners().has(bannerKey)) {
      chatBanner.hidden = true;
      return;
    }

    const art = window.HermesTeamsArtwork;
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

    const titleText = isStaged
      ? tr('chatPlanReady')
      : isDone
        ? tr('chatTeamDone')
        : tr('chatTeamRunning');

    chatBanner.hidden = false;
    chatBanner.className = `hat-chat-banner${isStaged ? ' is-staged' : isDone ? ' is-done' : ' is-running'}`;
    chatBanner.innerHTML = `
      <div class="hat-chat-banner-main" role="button" tabindex="0" data-action="open-chat-team">
        <img class="hat-chat-banner-img" src="${esc(art.LEAD_ART)}" alt="Captain Ferris" />
        <div class="hat-chat-banner-body">
          <div class="hat-chat-banner-top">
            <span class="hat-chat-banner-title">${esc(titleText)}</span>
            <span class="hat-chat-banner-name">${esc(target.name)}</span>
            <span class="hat-status-pill ${meta.cls}">${esc(meta.label)}</span>
            <span class="hat-chat-banner-tasks">${stats.completed || 0}/${stats.total || (target.tasks || []).length} ${esc(tr('tasks'))}${!isStaged ? ` · <span class="hat-timer" data-hat-timer-team="${esc(target.id)}">⏱ ${esc(formatDuration(computeTeamElapsedMs(target)))}</span>` : ''}</span>
          </div>
          <div class="hat-chat-banner-models">${modelChips}</div>
        </div>
        ${
          isDone
            ? `<button type="button" class="hat-btn-primary hat-chat-banner-cta" data-action="ask-captain-report">${esc(tr('chatAskReport'))} 🦀</button>`
            : `<button type="button" class="${isStaged ? 'hat-btn-primary' : 'hat-btn-secondary'} hat-chat-banner-cta" data-action="open-chat-team">${esc(isStaged ? tr('chatReviewApprove') : tr('chatOpenTeam'))} 🦀</button>`
        }
        <button type="button" class="hat-chat-dismiss" data-action="dismiss-chat-banner" title="${esc(tr('chatDismiss'))}" aria-label="${esc(tr('chatDismiss'))}">✕</button>
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
    const dismissBtn = chatBanner.querySelector('[data-action="dismiss-chat-banner"]');
    if (dismissBtn) {
      dismissBtn.addEventListener('click', (ev) => {
        ev.stopPropagation();
        dismissBannerKey(bannerKey);
        chatBanner.hidden = true;
      });
    }
    const reportBtn = chatBanner.querySelector('[data-action="ask-captain-report"]');
    if (reportBtn) {
      reportBtn.addEventListener('click', (ev) => {
        ev.stopPropagation();
        dismissBannerKey(bannerKey);
        chatBanner.hidden = true;
        triggerCaptainReport(target);
      });
    }
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
    if (e.key === 'Escape') {
      if (uiState.selectedTaskId) {
        uiState.selectedTaskId = null;
        render();
      } else if (uiState.open) {
        uiState.open = false;
        uiState._userClosed = true;
        render();
      }
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
    if (isTeamFinished(team)) return { cls: 'is-done', label: tr('statusDone') };
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
    panel.classList.toggle('is-minimized', uiState.minimized);
    renderChatBanner();
    if (!uiState.open) return;

    const team = currentTeam();
    if (team) void ensureCaptainRoute(team);

    // Preserve scroll positions across polling re-renders on the same tab & team
    const viewKey = `${uiState.tab}:${team ? team.id : ''}`;
    const sameView = uiState._lastRenderViewKey === viewKey;
    const savedScroll = sameView
      ? {
          bodyTop: (panel.querySelector('#hat-body') || {}).scrollTop || 0,
          dagTop: (panel.querySelector('.hat-dag-scroll') || {}).scrollTop || 0,
          dagLeft: (panel.querySelector('.hat-dag-scroll') || {}).scrollLeft || 0,
          liveTop: (panel.querySelector('.hat-live-feed') || {}).scrollTop || 0,
          detailTop: (panel.querySelector('.hat-detail-output') || {}).scrollTop || 0,
        }
      : null;
    uiState._lastRenderViewKey = viewKey;

    const restoreScroll = () => {
      if (!savedScroll) return;
      const b = panel.querySelector('#hat-body');
      if (b && savedScroll.bodyTop) b.scrollTop = savedScroll.bodyTop;
      const d = panel.querySelector('.hat-dag-scroll');
      if (d) {
        if (savedScroll.dagTop) d.scrollTop = savedScroll.dagTop;
        if (savedScroll.dagLeft) d.scrollLeft = savedScroll.dagLeft;
      }
      const l = panel.querySelector('.hat-live-feed');
      if (l && savedScroll.liveTop) l.scrollTop = savedScroll.liveTop;
      const o = panel.querySelector('.hat-detail-output');
      if (o && savedScroll.detailTop) o.scrollTop = savedScroll.detailTop;
    };
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

    const showStop = team && !team.archived && !team.halted && team.phase !== 'staged' && !isTeamFinished(team);
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
          <div class="hat-head-win-controls">
            <button type="button" class="hat-icon-btn" data-action="minimize" title="${uiState.minimized ? 'Restore' : 'Minimize'}" aria-label="Minimize panel">${
              uiState.minimized ? '＋' : '−'
            }</button>
            <button type="button" class="hat-icon-btn" data-action="toggle-float" title="Dock / Float" aria-label="Dock / Float">${
              uiState.floating ? '◧' : '❐'
            }</button>
            <button type="button" class="hat-icon-btn hat-close-btn" data-action="close" title="Close" aria-label="Close panel">✕</button>
          </div>
        </div>
        <div class="hat-head-subrow">
          ${
            team
              ? `<div class="hat-head-meta">${stats.memberCount} ${esc(tr('members'))} · ${stats.completed}/${stats.total} ${esc(tr('done'))} · ${stats.messageCount} ${esc(tr('messages'))}${
                  team.phase !== 'staged'
                    ? ` · <span class="hat-timer${!team.archived && !team.halted && !isTeamFinished(team) ? ' is-live' : ''}" data-hat-timer-team="${esc(team.id)}">⏱ ${esc(formatDuration(computeTeamElapsedMs(team)))}</span>`
                    : ''
                }</div>`
              : '<div class="hat-head-meta"></div>'
          }
          <div class="hat-head-controls">
            <button type="button" class="hat-icon-btn hat-fullaccess-btn${uiState.fullAccess ? ' is-on' : ''}" data-action="toggle-full-access" title="${esc(tr('fullAccessTip'))}">
              ⚡ ${esc(uiState.fullAccess ? tr('fullAccessOn') : tr('fullAccessOff'))}
            </button>
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
          </div>
        </div>
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
      restoreScroll();
      return;
    }

    if (uiState.tab === 'archive' && !uiState.selectedTeamId) {
      renderArchiveList(body, archivedTeams);
      restoreScroll();
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
        captainEffort: team.captainReasoningEffort || '',
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

    if (uiState.tab === 'archive' && team) {
      const backBar = document.createElement('div');
      backBar.className = 'hat-archive-backbar';
      backBar.innerHTML = `<button type="button" class="hat-btn-ghost" data-action="archive-back">${esc(tr('archiveBack'))}</button>`;
      body.appendChild(backBar);
      const wrap = document.createElement('div');
      body.appendChild(wrap);
      renderActiveWorkspace(wrap, team);
      restoreScroll();
      return;
    }

    renderActiveWorkspace(body, team);
    restoreScroll();
  }

  function renderArchiveList(body, archivedTeams) {
    if (!archivedTeams || !archivedTeams.length) {
      body.innerHTML = `<div class="hat-empty-state"><p>${esc(tr('archiveEmpty'))}</p></div>`;
      return;
    }
    const fmtDate = (ms) => {
      if (!ms) return '';
      try {
        return new Date(ms).toLocaleString([], { month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit' });
      } catch (_e) {
        return '';
      }
    };
    const rows = archivedTeams
      .map((t) => {
        const st = t.stats || {};
        const dur = formatDuration(computeTeamElapsedMs(t));
        const armed = uiState.archiveDeleteArmedId === t.id;
        return `
          <div class="hat-archive-row">
            <div class="hat-archive-row-main">
              <div class="hat-archive-row-top">
                <span class="hat-archive-name">${esc(t.name)}</span>
                <span class="hat-archive-date">${esc(fmtDate(t.createdAt))}</span>
              </div>
              <div class="hat-archive-meta">
                ${st.memberCount || (t.members || []).length} ${esc(tr('members'))} · ${st.completed || 0}/${st.total || (t.tasks || []).length} ${esc(tr('done'))} · ⏱ ${esc(dur)}
              </div>
            </div>
            <div class="hat-archive-row-actions">
              <button type="button" class="hat-btn-secondary hat-archive-btn" data-action="archive-inspect" data-team-id="${esc(t.id)}">${esc(tr('archiveInspect'))}</button>
              <button type="button" class="hat-icon-btn is-danger" data-action="archive-delete-one" data-team-id="${esc(t.id)}" data-workspace="${esc(t.workspace || '')}" title="${esc(tr('deleteTeam'))}">
                🗑${armed ? ` ${esc(tr('archiveConfirmOne'))}` : ''}
              </button>
            </div>
          </div>`;
      })
      .join('');

    body.innerHTML = `
      <div class="hat-archive-shell">
        <div class="hat-archive-head">
          <span class="hat-dag-title">${esc(tr('archiveListTitle'))} (${archivedTeams.length})</span>
          <button type="button" class="hat-icon-btn is-danger" data-action="archive-clear-all">
            🗑 ${esc(uiState.archiveClearAllArmed ? tr('archiveConfirmClearAll') : tr('archiveClearAll'))}
          </button>
        </div>
        <div class="hat-archive-list">${rows}</div>
      </div>`;
  }

  function renderActiveWorkspace(body, team) {
    const art = window.HermesTeamsArtwork;
    const stats = team.stats || {};
    const tasks = team.tasks || [];
    const members = team.members || [];
    const allDone = isTeamFinished(team);
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
                ${route ? `<span class="hat-model-pill" title="${esc(route)}${m.reasoningEffort ? ` · ${esc(m.reasoningEffort)}` : ''}">${esc(window.HermesTeamsDag.shortModel(route))}${m.reasoningEffort ? ` · ${esc(m.reasoningEffort)}` : ''}</span>` : ''}
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
                    ? `<span class="hat-model-pill hat-captain-model" title="${esc(captainRoute)}${team.captainReasoningEffort ? ` · ${esc(team.captainReasoningEffort)}` : ''}">${esc(window.HermesTeamsDag.shortModel(captainRoute))}${team.captainReasoningEffort ? ` · ${esc(team.captainReasoningEffort)}` : ''}</span>`
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
            ${team.escalated ? `<div class="hat-escalated-banner" role="alert">⚠ ${esc(tr('escalatedBanner'))}</div>` : ''}
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
                  ${route ? `<span class="hat-model-pill" title="${esc(route)}${m.reasoningEffort ? ` · ${esc(m.reasoningEffort)}` : ''}">${esc(window.HermesTeamsDag.shortModel(route))}${m.reasoningEffort ? ` · ${esc(m.reasoningEffort)}` : ''}</span>` : ''}
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
              ${
                rt && rt.startedAt
                  ? `<span class="hat-timer${effectiveState === 'running' ? ' is-live' : ''}"${effectiveState === 'running' ? ` data-hat-timer-start="${esc(rt.startedAt)}"` : ''}>⏱ ${esc(formatDuration((effectiveState === 'running' ? Date.now() : (rt.updatedAt || Date.now())) - rt.startedAt))}</span>`
                  : updated
                    ? `<span>${esc(formatRelative(updated))}</span>`
                    : ''
              }
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

  function wireHeaderControls(_team) {
    // Button clicks (close, minimize, float, full-access, stop, delete, tabs) are handled via
    // persistent event delegation on `panel` so polling refreshes never swallow clicks.
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
      (t) => !t.halted && t.phase !== 'staged' && !isTeamFinished(t)
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

  // Lightweight 1s timer tick (updates only textContent of timer spans — zero re-renders or scroll jumps)
  window.setInterval(() => {
    if (document.hidden) return;
    const allTeams = (uiState.payload.teams || []).concat(uiState.payload.archived || []);
    document.querySelectorAll('[data-hat-timer-team]').forEach((el) => {
      const tid = el.getAttribute('data-hat-timer-team');
      const t = allTeams.find((x) => x.id === tid);
      if (t && !t.archived && !t.halted && t.phase !== 'staged' && !isTeamFinished(t)) {
        el.textContent = `⏱ ${formatDuration(computeTeamElapsedMs(t))}`;
      }
    });
    document.querySelectorAll('[data-hat-timer-start]').forEach((el) => {
      const start = Number(el.getAttribute('data-hat-timer-start') || 0);
      if (start > 0) {
        el.textContent = `⏱ ${formatDuration(Date.now() - start)}`;
      }
    });
  }, 1000);

  refreshState().then(() => {
    void refreshSettings();
    connectSse();
    scheduleNextPoll();
  });
})();
