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
          const id = typeof m === 'string' ? m : m && (m.id || m.name);
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
    const draft = {
      description: team.description || '',
      members: (team.members || []).map((m) => ({
        name: m.name,
        role: m.role || '',
        provider: m.provider || '',
        model: m.model || '',
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
    let dirty = false;
    let busy = false;
    let errorMsg = '';

    function setDirty(val) {
      dirty = val;
      if (typeof opts.onDirtyChange === 'function') {
        opts.onDirtyChange(val);
      }
    }

    loadHermesModelCatalog(opts.appBasePath).then((catalog) => {
      function paint() {
        const captainRoute = (opts.captainRoute || '').trim();
        const shortCap = captainRoute && window.HermesTeamsDag
          ? window.HermesTeamsDag.shortModel(captainRoute)
          : captainRoute;
        const inheritLabel = shortCap
          ? `${tr('inheritCaptainModel')} (${shortCap})`
          : tr('inheritCaptainModel');

        const captainRowHtml = `
          <div class="hat-plan-captain">
            <img class="hat-plan-avatar" src="${esc(art.LEAD_ART)}" alt="${esc(tr('captain'))}" />
            <div class="hat-plan-captain-info">
              <div class="hat-member-top">
                <span class="hat-member-name">${esc(tr('captain'))}</span>
                <span class="hat-member-role">${esc(tr('captainSub'))}</span>
                ${
                  shortCap
                    ? `<span class="hat-model-pill hat-captain-model" title="${esc(captainRoute)}">${esc(shortCap)}</span>`
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
                  <select class="hat-select hat-select-model" data-field="route">
                    ${options.join('')}
                  </select>
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
                <button type="button" class="hat-btn-ghost" data-action="add-member">+ ${esc(tr('addMember'))}</button>
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
              setDirty(true);
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
              } else {
                draft.members[idx][f] = el.value;
              }
            };
            el.addEventListener('change', syncMember);
            if (el.tagName === 'INPUT') el.addEventListener('input', syncMember);
          });
          row.querySelector('.hat-remove-member').addEventListener('click', () => {
            setDirty(true);
            draft.members.splice(idx, 1);
            paint();
          });
        });

        container.querySelectorAll('.hat-plan-task').forEach((row) => {
          const idx = Number(row.getAttribute('data-tidx'));
          row.querySelectorAll('[data-field]').forEach((el) => {
            const syncTask = () => {
              setDirty(true);
              const f = el.getAttribute('data-field');
              if (f === 'dependencies') {
                draft.tasks[idx].dependencies = el.value
                  .split(',')
                  .map((s) => s.trim())
                  .filter(Boolean);
              } else {
                draft.tasks[idx][f] = el.value;
              }
            };
            el.addEventListener('change', syncTask);
            if (el.tagName === 'INPUT') el.addEventListener('input', syncTask);
          });
          row.querySelector('.hat-remove-task').addEventListener('click', () => {
            setDirty(true);
            draft.tasks.splice(idx, 1);
            paint();
          });
        });

        container.querySelector('[data-action="add-member"]').addEventListener('click', () => {
          setDirty(true);
          draft.members.push({
            name: `member-${draft.members.length + 1}`,
            role: 'engineer',
            provider: '',
            model: '',
            reasoningEffort: '',
          });
          paint();
        });

        container.querySelector('[data-action="add-task"]').addEventListener('click', () => {
          setDirty(true);
          draft.tasks.push({
            id: `t${draft.tasks.length + 1}`,
            subject: '',
            assignee: draft.members[0] ? draft.members[0].name : '',
            dependencies: [],
          });
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
          model: m.model.trim() || undefined,
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
  };
})();
