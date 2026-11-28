(function () {
  'use strict';

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  let cachedCatalog = null;
  const reasoningCapsCache = new Map();
  const FALLBACK_CAPS = {
    supported_efforts: ['minimal', 'low', 'medium', 'high', 'xhigh', 'max'],
    supports_reasoning_effort: true,
    supports_thinking_toggle: true,
  };
  const draftCache = new Map();
  const DRAFT_STORAGE_PREFIX = 'hat_staged_draft_v1_';

  function parseRouteString(route) {
    const raw = String(route || '').trim();
    if (!raw) return { provider: '', model: '' };
    if (raw.includes('::')) {
      const idx = raw.indexOf('::');
      return { provider: raw.slice(0, idx).trim(), model: raw.slice(idx + 2).trim() };
    }
    if (raw.includes('/')) {
      const idx = raw.indexOf('/');
      return { provider: raw.slice(0, idx).trim(), model: raw.slice(idx + 1).trim() };
    }
    return { provider: '', model: raw };
  }

  function normalizeModelId(model) {
    const raw = String(model || '').trim();
    // Strip `@provider:` prefix that /api/models returns on some codex/custom entries
    if (raw.startsWith('@')) {
      const lastColon = raw.lastIndexOf(':');
      if (lastColon > 0) return raw.slice(lastColon + 1).trim();
    }
    return raw;
  }

  async function loadModelReasoningCaps(basePath, provider, model) {
    const prov = String(provider || '').trim();
    const mod = normalizeModelId(model);
    if (!mod) return FALLBACK_CAPS;
    const key = `${prov}::${mod}`;
    if (reasoningCapsCache.has(key)) return reasoningCapsCache.get(key);
    try {
      const base = basePath || getAppBasePath();
      const qs = new URLSearchParams();
      qs.set('model', mod);
      if (prov) qs.set('provider', prov);
      const resp = await fetch(`${base}api/reasoning?${qs.toString()}`, { credentials: 'same-origin' });
      if (!resp.ok) return FALLBACK_CAPS;
      const data = await resp.json();
      const caps = {
        supported_efforts: Array.isArray(data.supported_efforts)
          ? data.supported_efforts.map((s) => String(s || '').trim().toLowerCase()).filter(Boolean)
          : [],
        supports_reasoning_effort: Boolean(data.supports_reasoning_effort),
        supports_thinking_toggle: Boolean(data.supports_thinking_toggle),
      };
      reasoningCapsCache.set(key, caps);
      return caps;
    } catch (_err) {
      return FALLBACK_CAPS;
    }
  }

  function getCachedReasoningCaps(provider, model) {
    const prov = String(provider || '').trim();
    const mod = normalizeModelId(model);
    if (!mod) return FALLBACK_CAPS;
    return reasoningCapsCache.get(`${prov}::${mod}`) || null;
  }

  function formatEffortLabel(level, tr) {
    const map = {
      none: tr('effortNone'),
      minimal: tr('effortMinimal'),
      low: tr('effortLow'),
      medium: tr('effortMedium'),
      high: tr('effortHigh'),
      xhigh: tr('effortXhigh'),
      max: tr('effortMax'),
    };
    return map[level] || `${tr('effortLabel')}: ${level}`;
  }

  function loadCachedDraft(teamId, serverUpdatedAt) {
    if (!teamId) return null;
    let entry = draftCache.get(teamId) || null;
    if (!entry) {
      try {
        const raw = window.sessionStorage && window.sessionStorage.getItem(DRAFT_STORAGE_PREFIX + teamId);
        if (raw) {
          const parsed = JSON.parse(raw);
          if (parsed && parsed.draft && Array.isArray(parsed.draft.members) && Array.isArray(parsed.draft.tasks)) {
            entry = parsed;
            draftCache.set(teamId, entry);
          }
        }
      } catch (_e) {
        // ignore storage errors
      }
    }
    if (!entry) return null;
    const srvTs = Number(serverUpdatedAt || 0);
    const baseTs = Number(entry.baseUpdatedAt || 0);
    // If the server plan was updated by Captain in chat after our local edits, prefer the fresh server plan.
    if (srvTs && baseTs && srvTs > baseTs) {
      clearCachedDraft(teamId);
      return null;
    }
    return entry.draft;
  }

  function saveCachedDraft(teamId, serverUpdatedAt, draft) {
    if (!teamId || !draft) return;
    const entry = {
      baseUpdatedAt: Number(serverUpdatedAt || 0),
      draft: JSON.parse(JSON.stringify(draft)),
    };
    draftCache.set(teamId, entry);
    try {
      if (window.sessionStorage) {
        window.sessionStorage.setItem(DRAFT_STORAGE_PREFIX + teamId, JSON.stringify(entry));
      }
    } catch (_e) {
      // ignore storage quota/privacy errors
    }
  }

  function clearCachedDraft(teamId) {
    if (!teamId) return;
    draftCache.delete(teamId);
    try {
      if (window.sessionStorage) {
        window.sessionStorage.removeItem(DRAFT_STORAGE_PREFIX + teamId);
      }
    } catch (_e) {
      // ignore
    }
  }

  function getAppBasePath() {
    const path = (window.location.pathname || '/').replace(/\/session\/[^/?#]+.*$/, '');
    return path.endsWith('/') ? path : `${path}/`;
  }

  async function loadHermesModelCatalog(basePath) {
    if (cachedCatalog) return cachedCatalog;
    try {
      const base = basePath || getAppBasePath();
      const resp = await fetch(`${base}api/models`, { credentials: 'same-origin' });
      if (!resp.ok) return [];
      const data = await resp.json();
      const out = [];
      const groups = Array.isArray(data.groups) ? data.groups : [];
      for (const g of groups) {
        const provider = g.provider_id || g.provider || '';
        const list = [].concat(g.models || [], g.extra_models || []);
        for (const m of list) {
          const rawId = typeof m === 'string' ? m : m && (m.id || m.name);
          const id = normalizeModelId(rawId);
          if (!id) continue;
          out.push({
            provider,
            model: id,
            label: provider ? `${provider} / ${id}` : id,
            value: provider ? `${provider}::${id}` : `::${id}`,
          });
        }
      }
      cachedCatalog = out;
      return out;
    } catch (_err) {
      return [];
    }
  }

  function renderPlanEditor(container, team, opts) {
    const tr = opts.tr;
    const art = window.HermesTeamsArtwork;
    const restored = loadCachedDraft(team.id, team.updatedAt);
    const draft = restored
      ? JSON.parse(JSON.stringify(restored))
      : {
          description: team.description || '',
          members: (team.members || []).map((m) => ({
            name: m.name,
            role: m.role || '',
            provider: m.provider || '',
            model: normalizeModelId(m.model || ''),
            reasoningEffort: m.reasoningEffort || '',
            executionPrompt: m.executionPrompt || undefined,
            toolPolicy: m.toolPolicy || undefined,
          })),
          tasks: (team.tasks || []).map((t) => ({
            id: t.id,
            subject: t.subject,
            description: t.description || undefined,
            assignee: t.assignee || '',
            dependencies: Array.isArray(t.dependencies) ? t.dependencies.slice() : [],
            kind: t.kind || undefined,
            objective: t.objective || undefined,
            inScope: t.inScope || undefined,
            outOfScope: t.outOfScope || undefined,
            acceptance: t.acceptance || undefined,
            verify: t.verify || undefined,
            reviewedTaskId: t.reviewedTaskId || undefined,
            coverageOf: t.coverageOf || undefined,
          })),
        };

    let discardArmed = false;
    let dirty = !!restored;
    let busy = false;
    let errorMsg = '';

    function persistLocalDraft() {
      if (dirty) {
        saveCachedDraft(team.id, team.updatedAt, draft);
      } else {
        clearCachedDraft(team.id);
      }
    }

    function setDirty(val) {
      dirty = val;
      if (typeof opts.onDirtyChange === 'function') {
        opts.onDirtyChange(val);
      }
      persistLocalDraft();
    }

    if (restored && typeof opts.onDirtyChange === 'function') {
      opts.onDirtyChange(true);
    }

    function effectiveMemberRoute(m) {
      if (m && m.model) {
        return { provider: m.provider || '', model: m.model };
      }
      return parseRouteString(opts.captainRoute || '');
    }

    async function prefetchAllMemberCaps() {
      const seen = new Set();
      const jobs = [];
      const cap = parseRouteString(opts.captainRoute || '');
      if (cap.model) {
        const k = `${cap.provider}::${normalizeModelId(cap.model)}`;
        seen.add(k);
        jobs.push(loadModelReasoningCaps(opts.appBasePath, cap.provider, cap.model));
      }
      for (const m of draft.members) {
        const r = effectiveMemberRoute(m);
        if (!r.model) continue;
        const k = `${r.provider}::${normalizeModelId(r.model)}`;
        if (seen.has(k)) continue;
        seen.add(k);
        jobs.push(loadModelReasoningCaps(opts.appBasePath, r.provider, r.model));
      }
      if (jobs.length) await Promise.all(jobs);
    }

    Promise.all([loadHermesModelCatalog(opts.appBasePath), prefetchAllMemberCaps()]).then(([catalog]) => {
      function paint() {
        const captainRoute = (opts.captainRoute || '').trim();
        const captainEffort = (opts.captainEffort || '').trim();
        const shortCap = captainRoute && window.HermesTeamsDag
          ? window.HermesTeamsDag.shortModel(captainRoute)
          : captainRoute;
        const inheritLabel = shortCap
          ? `${tr('inheritCaptainModel')} (${shortCap})`
          : tr('inheritCaptainModel');
        const inheritEffortLabel = captainEffort
          ? `${tr('effortInherit')} (${captainEffort})`
          : tr('effortInherit');

        const captainRowHtml = `
          <div class="hat-plan-captain">
            <img class="hat-plan-avatar" src="${esc(art.LEAD_ART)}" alt="${esc(tr('captain'))}" />
            <div class="hat-plan-captain-info">
              <div class="hat-member-top">
                <span class="hat-member-name">${esc(tr('captain'))}</span>
                <span class="hat-member-role">${esc(tr('captainSub'))}</span>
                ${
                  shortCap
                    ? `<span class="hat-model-pill hat-captain-model" title="${esc(captainRoute)}">${esc(shortCap)}${captainEffort ? ` · ${esc(captainEffort)}` : ''}</span>`
                    : `<span class="hat-model-pill hat-model-pending">${esc(tr('modelDetecting'))}</span>`
                }
              </div>
            </div>
          </div>`;

        const memberRows = draft.members
          .map((m, idx) => {
            const currentVal = m.model ? `${m.provider || ''}::${m.model}` : '';
            const options = [
              `<option value="">${esc(inheritLabel)}</option>`,
              ...catalog.map(
                (c) =>
                  `<option value="${esc(c.value)}"${c.value === currentVal ? ' selected' : ''}>${esc(c.label)}</option>`
              ),
            ];
            if (currentVal && !catalog.some((c) => c.value === currentVal)) {
              options.push(
                `<option value="${esc(currentVal)}" selected>${esc(m.provider ? `${m.provider} / ${m.model}` : m.model)}</option>`
              );
            }
            const effRoute = effectiveMemberRoute(m);
            const caps = getCachedReasoningCaps(effRoute.provider, effRoute.model) || FALLBACK_CAPS;
            const allowedEfforts = Array.isArray(caps.supported_efforts) ? caps.supported_efforts : [];
            const supportsAnyEffort = allowedEfforts.length > 0 || caps.supports_thinking_toggle;
            const curEffort = (m.reasoningEffort || '').trim().toLowerCase();
            let effortOptionsHtml = '';
            if (!supportsAnyEffort) {
              effortOptionsHtml = `<option value="">${esc(tr('effortUnsupported'))}</option>`;
            } else {
              const optsList = [{ value: '', label: inheritEffortLabel }];
              if (caps.supports_thinking_toggle) {
                optsList.push({ value: 'none', label: formatEffortLabel('none', tr) });
              }
              for (const lvl of allowedEfforts) {
                if (lvl === 'none') continue;
                optsList.push({ value: lvl, label: formatEffortLabel(lvl, tr) });
              }
              if (curEffort && !optsList.some((o) => o.value === curEffort)) {
                optsList.push({ value: curEffort, label: formatEffortLabel(curEffort, tr) });
              }
              effortOptionsHtml = optsList
                .map(
                  (o) =>
                    `<option value="${esc(o.value)}"${o.value === curEffort ? ' selected' : ''}>${esc(o.label)}</option>`
                )
                .join('');
            }
            const imgUrl = art.memberArtUrl(m.name, m.role);
            return `
              <div class="hat-plan-member" data-midx="${idx}">
                <img class="hat-plan-avatar" src="${esc(imgUrl)}" alt="${esc(m.name)}" />
                <div class="hat-plan-member-fields">
                  <div class="hat-plan-row-top">
                    <input class="hat-input hat-input-name" data-field="name" value="${esc(m.name)}" placeholder="name" />
                    <input class="hat-input hat-input-role" data-field="role" value="${esc(m.role)}" placeholder="role (engineer, qa, security...)" />
                    <button type="button" class="hat-btn-icon hat-remove-member" title="Remove">✕</button>
                  </div>
                  <div class="hat-plan-row-model">
                    <select class="hat-select hat-select-model" data-field="route">
                      ${options.join('')}
                    </select>
                    <select class="hat-select hat-select-effort" data-field="reasoningEffort"${!supportsAnyEffort ? ' disabled' : ''} title="${esc(tr('effortLabel'))}" aria-label="${esc(tr('effortLabel'))}">
                      ${effortOptionsHtml}
                    </select>
                  </div>
                </div>
              </div>`;
          })
          .join('');

        const memberNames = draft.members.map((m) => m.name).filter(Boolean);
        const taskRows = draft.tasks
          .map((t, idx) => {
            const assigneeOpts = [
              `<option value="">${esc(tr('unassigned'))}</option>`,
              `<option value="captain"${t.assignee === 'captain' ? ' selected' : ''}>captain</option>`,
              ...memberNames.map(
                (n) => `<option value="${esc(n)}"${t.assignee === n ? ' selected' : ''}>${esc(n)}</option>`
              ),
            ].join('');
            return `
              <div class="hat-plan-task" data-tidx="${idx}">
                <span class="hat-task-badge">${esc(t.id || `t${idx + 1}`)}</span>
                <input class="hat-input hat-input-subject" data-field="subject" value="${esc(t.subject)}" placeholder="Task subject" />
                <select class="hat-select hat-select-assignee" data-field="assignee">${assigneeOpts}</select>
                <input class="hat-input hat-input-deps" data-field="dependencies" value="${esc(t.dependencies.join(', '))}" placeholder="deps: t1, t2" />
                <button type="button" class="hat-btn-icon hat-remove-task" title="Remove">✕</button>
              </div>`;
          })
          .join('');

        const isAwaitingFeedback = team.planReviewState === 'awaiting_feedback';

        container.innerHTML = `
          <div class="hat-plan-shell">
            <div class="hat-plan-banner">
              <strong>🦀 ${esc(tr('stagedBannerTitle'))}</strong>
              <span>${esc(isAwaitingFeedback ? tr('awaitingFeedbackHint') : tr('stagedBannerDesc'))}</span>
            </div>

            <div class="hat-plan-section">
              <div class="hat-plan-section-head">
                <span>${esc(tr('roster'))} (${draft.members.length + 1})</span>
                <div style="display:inline-flex;gap:4px;">
                  <button type="button" class="hat-btn-ghost" data-action="add-deployer" title="Add dedicated deploy worker">+ Deployer 🚀</button>
                  <button type="button" class="hat-btn-ghost" data-action="add-member">+ ${esc(tr('addMember'))}</button>
                </div>
              </div>
              <div class="hat-plan-list">${captainRowHtml}${memberRows}</div>
            </div>

            <div class="hat-plan-section">
              <div class="hat-plan-section-head">
                <span>${esc(tr('tasks'))} (${draft.tasks.length})</span>
                <button type="button" class="hat-btn-ghost" data-action="add-task">+ ${esc(tr('addTask'))}</button>
              </div>
              <div class="hat-plan-list">${taskRows}</div>
            </div>

            ${errorMsg ? `<div class="hat-plan-error" role="alert">${esc(errorMsg)}</div>` : ''}

            <div class="hat-plan-actions">
              <button type="button" class="hat-btn-primary" data-action="approve">
                ▶ ${esc(tr('approveAndRun'))}
              </button>
              <button type="button" class="hat-btn-secondary" data-action="save">
                ${esc(tr('saveDraft'))}
              </button>
              <button type="button" class="hat-btn-secondary" data-action="feedback">
                💬 ${esc(tr('returnToChat'))}
              </button>
              <button type="button" class="hat-btn-danger" data-action="discard">
                ${esc(discardArmed ? tr('confirmDiscard') : tr('discardPlan'))}
              </button>
            </div>
          </div>`;

        container.querySelectorAll('.hat-plan-member').forEach((row) => {
          const idx = Number(row.getAttribute('data-midx'));
          row.querySelectorAll('[data-field]').forEach((el) => {
            const syncMember = () => {
              const f = el.getAttribute('data-field');
              if (f === 'route') {
                const v = el.value || '';
                if (!v) {
                  draft.members[idx].provider = '';
                  draft.members[idx].model = '';
                } else {
                  const parts = v.split('::');
                  draft.members[idx].provider = parts[0] || '';
                  draft.members[idx].model = parts[1] || '';
                }
                setDirty(true);
                const effRoute = effectiveMemberRoute(draft.members[idx]);
                loadModelReasoningCaps(opts.appBasePath, effRoute.provider, effRoute.model).then((caps) => {
                  const cur = (draft.members[idx] && draft.members[idx].reasoningEffort || '').trim().toLowerCase();
                  if (cur) {
                    const allowed = new Set(caps.supported_efforts || []);
                    if (caps.supports_thinking_toggle) allowed.add('none');
                    if (!allowed.has(cur)) {
                      draft.members[idx].reasoningEffort = '';
                      setDirty(true);
                    }
                  }
                  paint();
                });
                return;
              } else {
                draft.members[idx][f] = el.value;
              }
              setDirty(true);
            };
            el.addEventListener('change', syncMember);
            if (el.tagName === 'INPUT') el.addEventListener('input', syncMember);
          });
          row.querySelector('.hat-remove-member').addEventListener('click', () => {
            draft.members.splice(idx, 1);
            setDirty(true);
            paint();
          });
        });

        container.querySelectorAll('.hat-plan-task').forEach((row) => {
          const idx = Number(row.getAttribute('data-tidx'));
          row.querySelectorAll('[data-field]').forEach((el) => {
            const syncTask = () => {
              const f = el.getAttribute('data-field');
              if (f === 'dependencies') {
                draft.tasks[idx].dependencies = el.value
                  .split(',')
                  .map((s) => s.trim())
                  .filter(Boolean);
              } else {
                draft.tasks[idx][f] = el.value;
              }
              setDirty(true);
            };
            el.addEventListener('change', syncTask);
            if (el.tagName === 'INPUT') el.addEventListener('input', syncTask);
          });
          row.querySelector('.hat-remove-task').addEventListener('click', () => {
            draft.tasks.splice(idx, 1);
            setDirty(true);
            paint();
          });
        });

        container.querySelector('[data-action="add-deployer"]').addEventListener('click', () => {
          const baseName = draft.members.some((m) => m.name === 'deployer')
            ? `deployer-${draft.members.length + 1}`
            : 'deployer';
          draft.members.push({
            name: baseName,
            role: 'deployer',
            provider: '',
            model: '',
            reasoningEffort: '',
            toolPolicy: 'deploy',
          });
          setDirty(true);
          paint();
        });

        container.querySelector('[data-action="add-member"]').addEventListener('click', () => {
          draft.members.push({
            name: `member-${draft.members.length + 1}`,
            role: 'engineer',
            provider: '',
            model: '',
            reasoningEffort: '',
          });
          setDirty(true);
          paint();
        });

        container.querySelector('[data-action="add-task"]').addEventListener('click', () => {
          draft.tasks.push({
            id: `t${draft.tasks.length + 1}`,
            subject: '',
            assignee: draft.members[0] ? draft.members[0].name : '',
            dependencies: [],
          });
          setDirty(true);
          paint();
        });

        container.querySelector('[data-action="save"]').addEventListener('click', async () => {
          if (busy) return;
          busy = true;
          errorMsg = '';
          try {
            await opts.onSaveDraft(team.id, cleanDraft(draft));
            setDirty(false);
          } catch (err) {
            errorMsg = (err && err.message) || 'Failed to save draft';
            paint();
          } finally {
            busy = false;
          }
        });

        container.querySelector('[data-action="approve"]').addEventListener('click', async (ev) => {
          if (busy) return;
          busy = true;
          errorMsg = '';
          const btn = ev.currentTarget;
          btn.disabled = true;
          try {
            if (dirty) {
              await opts.onSaveDraft(team.id, cleanDraft(draft));
              setDirty(false);
            }
            await opts.onApprove(team.id);
          } catch (err) {
            errorMsg = (err && err.message) || 'Failed to approve plan';
            btn.disabled = false;
            paint();
          } finally {
            busy = false;
          }
        });

        container.querySelector('[data-action="feedback"]').addEventListener('click', async () => {
          if (busy) return;
          busy = true;
          errorMsg = '';
          try {
            if (dirty) {
              await opts.onSaveDraft(team.id, cleanDraft(draft));
              setDirty(false);
            }
            await opts.onFeedback(team.id);
          } catch (err) {
            errorMsg = (err && err.message) || 'Failed to request feedback';
            paint();
          } finally {
            busy = false;
          }
        });

        container.querySelector('[data-action="discard"]').addEventListener('click', async () => {
          if (!discardArmed) {
            discardArmed = true;
            paint();
            return;
          }
          if (busy) return;
          busy = true;
          errorMsg = '';
          try {
            clearCachedDraft(team.id);
            await opts.onDiscard(team.id);
          } catch (err) {
            errorMsg = (err && err.message) || 'Failed to discard plan';
            discardArmed = false;
            paint();
          } finally {
            busy = false;
          }
        });
      }

      paint();
    });
  }

  function cleanDraft(draft) {
    return {
      description: draft.description,
      members: draft.members
        .filter((m) => m.name && m.name.trim())
        .map((m) => ({
          name: m.name.trim(),
          role: m.role.trim() || undefined,
          provider: m.provider.trim() || undefined,
          model: normalizeModelId(m.model) || undefined,
          reasoningEffort: m.reasoningEffort.trim() || undefined,
          executionPrompt: m.executionPrompt,
          toolPolicy: m.toolPolicy,
        })),
      tasks: draft.tasks
        .filter((t) => t.subject && t.subject.trim())
        .map((t, i) => ({
          id: t.id || `t${i + 1}`,
          subject: t.subject.trim(),
          description: t.description,
          assignee: t.assignee.trim() || undefined,
          dependencies: t.dependencies,
          kind: t.kind,
          objective: t.objective,
          inScope: t.inScope,
          outOfScope: t.outOfScope,
          acceptance: t.acceptance,
          verify: t.verify,
          reviewedTaskId: t.reviewedTaskId,
          coverageOf: t.coverageOf,
        })),
    };
  }

  window.HermesTeamsPlanEditor = {
    renderPlanEditor,
    loadHermesModelCatalog,
    loadModelReasoningCaps,
    getCachedReasoningCaps,
    normalizeModelId,
    parseRouteString,
    formatEffortLabel,
  };
})();
