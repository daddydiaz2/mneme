/// Web dashboard HTML for mneme.
/// Sirve una UI estática en `/` del servidor HTTP.
/// NO se abre automáticamente en el navegador (mneme es API + MCP, no una web app).
pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>mneme — Memory Dashboard</title>
    <link rel="icon" type="image/svg+xml" href="/favicon.ico">
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 24px;
            line-height: 1.5;
        }
        .container { max-width: 1400px; margin: 0 auto; }
        h1 { color: #58a6ff; margin-bottom: 4px; font-size: 28px; }
        h2 { color: #f0f6fc; font-size: 13px; margin-bottom: 10px; text-transform: uppercase; letter-spacing: 0.5px; }
        h3 { color: #f0f6fc; font-size: 12px; margin: 14px 0 6px 0; }
        .subtitle { color: #8b949e; margin-bottom: 20px; font-size: 13px; }

        .stats-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 16px; }
        .stat-card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 14px; }
        .stat-card .value { font-size: 26px; font-weight: 700; color: #f0f6fc; }
        .stat-card .label { color: #8b949e; font-size: 11px; margin-top: 4px; text-transform: uppercase; letter-spacing: 0.5px; }
        .stat-card .delta { font-size: 11px; color: #7ee787; margin-top: 4px; }
        .stat-card .delta.negative { color: #ff7b72; }

        .grid-2col { display: grid; grid-template-columns: 320px 1fr; gap: 16px; margin-bottom: 16px; }
        @media (max-width: 900px) { .grid-2col { grid-template-columns: 1fr; } }

        .panel { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; }
        .panel h2 { margin-bottom: 12px; padding-bottom: 8px; border-bottom: 1px solid #21262d; }

        .filter-group { margin-bottom: 14px; }
        .filter-group label { display: block; color: #8b949e; font-size: 11px; margin-bottom: 6px; text-transform: uppercase; }
        .filter-chip {
            display: inline-block; padding: 4px 10px; margin: 2px;
            background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
            border-radius: 14px; font-size: 11px; cursor: pointer; user-select: none;
            transition: all 0.1s;
        }
        .filter-chip:hover { border-color: #58a6ff; }
        .filter-chip.active { background: #1f6feb; border-color: #1f6feb; color: #fff; }
        .filter-chip .count { color: #8b949e; font-size: 10px; margin-left: 4px; }
        .filter-chip.active .count { color: rgba(255,255,255,0.7); }

        input[type="text"], select {
            width: 100%; padding: 8px 10px;
            background: #0d1117; border: 1px solid #30363d; color: #c9d1d9;
            border-radius: 6px; font-size: 13px; outline: none;
        }
        input[type="text"]:focus, select:focus { border-color: #58a6ff; }
        input::placeholder { color: #6e7681; }

        button {
            padding: 8px 14px; background: #238636; border: 0; color: #fff;
            border-radius: 6px; cursor: pointer; font-size: 13px; font-weight: 500;
        }
        button:hover { background: #2ea043; }
        button.secondary { background: #21262d; color: #c9d1d9; }
        button.secondary:hover { background: #30363d; }

        .memory-list { max-height: 600px; overflow-y: auto; }
        .memory-item {
            background: #0d1117; border: 1px solid #21262d; border-radius: 6px;
            padding: 12px 14px; margin-bottom: 8px; cursor: pointer;
            transition: border-color 0.1s;
        }
        .memory-item:hover { border-color: #58a6ff; }
        .memory-item.selected { border-color: #1f6feb; background: #0d1929; }
        .memory-title { color: #f0f6fc; font-size: 14px; margin-bottom: 6px; }
        .memory-content {
            color: #8b949e; font-size: 12px; line-height: 1.4;
            display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical;
            overflow: hidden;
        }
        .memory-meta { color: #6e7681; font-size: 10px; margin-top: 6px; display: flex; gap: 8px; align-items: center; }

        .pill { display: inline-block; padding: 2px 7px; border-radius: 3px; font-size: 10px; font-weight: 600; }
        .pill-critical { background: #da3633; color: #fff; }
        .pill-high { background: #d29922; color: #fff; }
        .pill-medium { background: #6e7681; color: #fff; }
        .pill-low { background: #21262d; color: #8b949e; }
        .tag { display: inline-block; background: #1f6feb33; color: #79c0ff; padding: 1px 6px; border-radius: 3px; font-size: 10px; margin-right: 3px; }
        .type-pill { display: inline-block; background: #21262d; color: #c9d1d9; padding: 2px 6px; border-radius: 3px; font-size: 10px; font-weight: 500; }

        .highlight { background: #f0f6fc; color: #0d1117; padding: 0 2px; border-radius: 2px; font-weight: 600; }

        .empty { color: #6e7681; font-style: italic; padding: 20px; text-align: center; }

        .detail-panel { max-height: 700px; overflow-y: auto; }
        .detail-row { margin-bottom: 8px; }
        .detail-label { color: #8b949e; font-size: 11px; text-transform: uppercase; margin-bottom: 2px; }
        .detail-value { color: #f0f6fc; font-size: 13px; word-break: break-word; }

        .entity-link { color: #79c0ff; cursor: pointer; }
        .entity-link:hover { text-decoration: underline; }

        .pagination { display: flex; gap: 8px; margin-top: 12px; align-items: center; }
        .pagination button { padding: 6px 12px; font-size: 12px; }
        .pagination .page-info { color: #8b949e; font-size: 12px; }

        .footer { color: #6e7681; font-size: 11px; text-align: center; margin-top: 32px; padding-top: 16px; border-top: 1px solid #21262d; }
        .footer a { color: #58a6ff; text-decoration: none; }

        .tabs { display: flex; gap: 4px; margin-bottom: 12px; border-bottom: 1px solid #21262d; }
        .tab { padding: 8px 14px; color: #8b949e; cursor: pointer; border-bottom: 2px solid transparent; font-size: 12px; }
        .tab:hover { color: #c9d1d9; }
        .tab.active { color: #f0f6fc; border-bottom-color: #58a6ff; }

        .skeleton { background: linear-gradient(90deg, #161b22 0%, #21262d 50%, #161b22 100%); background-size: 200% 100%; animation: pulse 1.5s infinite; border-radius: 4px; height: 14px; margin-bottom: 6px; }
        @keyframes pulse { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

        .toolbar { display: flex; gap: 8px; margin-bottom: 12px; }
        .toolbar input { flex: 1; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧠 mneme</h1>
        <p class="subtitle">Persistent memory for AI agents · v0.1.0 · <span id="project-name">__DEFAULT_PROJECT__</span></p>

        <!-- Stats overview -->
        <div class="stats-grid" id="stats-grid">
            <div class="stat-card"><div class="value" id="stat-total">—</div><div class="label">Total memorias</div></div>
            <div class="stat-card"><div class="value" id="stat-relations">—</div><div class="label">Relaciones</div></div>
            <div class="stat-card"><div class="value" id="stat-sessions">—</div><div class="label">Sesiones</div></div>
            <div class="stat-card"><div class="value" id="stat-conflicts">—</div><div class="label">Conflictos pendientes</div></div>
            <div class="stat-card"><div class="value" id="stat-entities">—</div><div class="label">Entidades</div></div>
        </div>

        <!-- Two-column layout: filters | content -->
        <div class="grid-2col">
            <!-- Filters panel -->
            <div class="panel">
                <h2>🎛 Filtros</h2>

                <div class="filter-group">
                    <label>Buscar</label>
                    <input type="text" id="search-input" placeholder="Buscar en title, content, tags...">
                </div>

                <div class="filter-group">
                    <label>Tipo de memoria</label>
                    <div id="filter-types" class="filter-chips">
                        <!-- populated by JS -->
                    </div>
                </div>

                <div class="filter-group">
                    <label>Importancia</label>
                    <div id="filter-importance" class="filter-chips">
                        <div class="filter-chip" data-imp="low">low <span class="count">·</span></div>
                        <div class="filter-chip" data-imp="medium">medium <span class="count">·</span></div>
                        <div class="filter-chip" data-imp="high">high <span class="count">·</span></div>
                        <div class="filter-chip" data-imp="critical">critical <span class="count">·</span></div>
                    </div>
                </div>

                <div class="filter-group">
                    <label>Tags</label>
                    <div id="filter-tags" class="filter-chips">
                        <!-- populated by JS -->
                    </div>
                </div>

                <div class="filter-group">
                    <label>Ordenar por</label>
                    <select id="sort-select">
                        <option value="updated_desc">Recientes primero</option>
                        <option value="updated_asc">Antiguas primero</option>
                        <option value="importance_desc">Más importantes</option>
                        <option value="access_desc">Más accedidas</option>
                        <option value="title_asc">Título (A-Z)</option>
                    </select>
                </div>

                <button class="secondary" onclick="resetFilters()" style="width:100%;">Limpiar filtros</button>
            </div>

            <!-- Memories panel -->
            <div class="panel">
                <div class="tabs">
                    <div class="tab active" data-tab="all">Todas</div>
                    <div class="tab" data-tab="entities">Entidades</div>
                    <div class="tab" data-tab="temporal">Temporal</div>
                </div>

                <div id="tab-content-all">
                    <div class="toolbar">
                        <span id="results-count" style="color:#8b949e; font-size:12px; align-self:center;">—</span>
                    </div>
                    <div class="memory-list" id="memories-list">
                        <div class="empty">Cargando memorias...</div>
                    </div>
                    <div class="pagination">
                        <button onclick="prevPage()" class="secondary">← Anterior</button>
                        <span class="page-info" id="page-info">—</span>
                        <button onclick="nextPage()" class="secondary">Siguiente →</button>
                    </div>
                </div>

                <div id="tab-content-entities" style="display:none;">
                    <h3>Entidades más frecuentes</h3>
                    <div id="entities-list"></div>
                </div>

                <div id="tab-content-temporal" style="display:none;">
                    <h3>Validez temporal</h3>
                    <div id="temporal-stats"></div>
                    <h3 style="margin-top:16px;">Memorias con ventana de validez</h3>
                    <div id="temporal-memories"></div>
                </div>
            </div>
        </div>

        <!-- Memory detail panel -->
        <div class="panel" id="detail-panel" style="display:none; margin-top:16px;">
            <h2 id="detail-title">Memoria</h2>
            <div class="detail-panel" id="detail-content"></div>
        </div>

        <p class="footer">
            mneme v0.1.0 ·
            <a href="https://github.com/daddydiaz2/mneme" target="_blank">GitHub</a> ·
            <a href="/api/v1/projects">API</a> ·
            64 MCP tools disponibles
        </p>
    </div>

    <script>
        const API = '';
        const PROJECT = '__DEFAULT_PROJECT__';
        const PAGE_SIZE = 20;
        let currentPage = 0;
        let currentFilters = { search: '', types: [], importance: [], tags: [], sort: 'updated_desc' };
        let currentTab = 'all';
        let memoryTypes = [];
        let importanceCounts = {};
        let allTags = [];

        // === TAB SWITCHING ===
        document.querySelectorAll('.tab').forEach(t => {
            t.addEventListener('click', () => switchTab(t.dataset.tab));
        });
        function switchTab(name) {
            currentTab = name;
            document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.tab === name));
            document.getElementById('tab-content-all').style.display = name === 'all' ? 'block' : 'none';
            document.getElementById('tab-content-entities').style.display = name === 'entities' ? 'block' : 'none';
            document.getElementById('tab-content-temporal').style.display = name === 'temporal' ? 'block' : 'none';
            if (name === 'entities') loadEntities();
            if (name === 'temporal') loadTemporal();
        }

        // === STATS ===
        async function loadStats() {
            try {
                const [stats, projects, peers, conflictResp] = await Promise.all([
                    fetch(`${API}/api/v1/stats?project=${PROJECT}`).then(r => r.json()).catch(() => null),
                    fetch(`${API}/api/v1/projects`).then(r => r.json()).catch(() => []),
                    fetch(`${API}/api/v1/cloud/status?project=${PROJECT}`).then(r => r.json()).catch(() => null)
                ]);

                if (stats) {
                    document.getElementById('stat-total').textContent = stats.total_memories ?? '0';
                    document.getElementById('stat-relations').textContent = stats.total_relations ?? '0';
                    document.getElementById('stat-sessions').textContent = stats.total_sessions ?? '0';
                }
                if (conflictResp) {
                    // Estimate pending conflicts from sync log
                    const conflicts = (conflictResp.recent_syncs || []).filter(s => s.status === 'partial').length;
                    document.getElementById('stat-conflicts').textContent = conflicts;
                } else {
                    document.getElementById('stat-conflicts').textContent = '0';
                }
                // Entities count: get from /api/v1/memories search results
                loadEntityCount();
            } catch (e) {
                console.error('loadStats failed', e);
            }
        }

        async function loadEntityCount() {
            try {
                const r = await fetch(`${API}/api/v1/memories?project=${PROJECT}&limit=100`);
                const j = await r.json();
                if (Array.isArray(j)) {
                    const entities = new Set();
                    j.forEach(m => (m.tags || []).forEach(t => entities.add(t)));
                    document.getElementById('stat-entities').textContent = entities.size;
                }
            } catch (e) {
                document.getElementById('stat-entities').textContent = '?';
            }
        }

        // === FILTERS LOADING ===
        async function loadFilterOptions() {
            try {
                const memories = await fetch(`${API}/api/v1/memories?project=${PROJECT}&limit=500`).then(r => r.json()).catch(() => []);
                if (!Array.isArray(memories)) return;

                memoryTypes = {};
                importanceCounts = { low: 0, medium: 0, high: 0, critical: 0 };
                allTags = new Map();
                memories.forEach(m => {
                    memoryTypes[m.memory_type] = (memoryTypes[m.memory_type] || 0) + 1;
                    importanceCounts[m.importance] = (importanceCounts[m.importance] || 0) + 1;
                    (m.tags || []).forEach(t => allTags.set(t, (allTags.get(t) || 0) + 1));
                });

                // Render type chips
                const typesEl = document.getElementById('filter-types');
                typesEl.innerHTML = Object.entries(memoryTypes).sort((a, b) => b[1] - a[1]).map(([t, c]) =>
                    `<div class="filter-chip" data-type="${t}">${t} <span class="count">${c}</span></div>`
                ).join('') || '<span class="empty" style="padding:0;">No hay memorias</span>';

                // Render importance counts
                document.querySelectorAll('#filter-importance .filter-chip').forEach(chip => {
                    const imp = chip.dataset.imp;
                    chip.querySelector('.count').textContent = `· ${importanceCounts[imp] || 0}`;
                });

                // Render top tags
                const tagsEl = document.getElementById('filter-tags');
                const topTags = [...allTags.entries()].sort((a, b) => b[1] - a[1]).slice(0, 30);
                tagsEl.innerHTML = topTags.map(([t, c]) =>
                    `<div class="filter-chip" data-tag="${t}">${t} <span class="count">${c}</span></div>`
                ).join('') || '<span class="empty" style="padding:0;">Sin tags</span>';

                // Attach filter handlers
                document.querySelectorAll('.filter-chip').forEach(chip => {
                    chip.addEventListener('click', () => toggleFilter(chip));
                });
            } catch (e) {
                console.error('loadFilterOptions failed', e);
            }
        }

        function toggleFilter(chip) {
            const t = chip.dataset.type;
            const i = chip.dataset.imp;
            const tg = chip.dataset.tag;
            if (t) {
                if (currentFilters.types.includes(t)) {
                    currentFilters.types = currentFilters.types.filter(x => x !== t);
                    chip.classList.remove('active');
                } else {
                    currentFilters.types.push(t);
                    chip.classList.add('active');
                }
            } else if (i) {
                if (currentFilters.importance.includes(i)) {
                    currentFilters.importance = currentFilters.importance.filter(x => x !== i);
                    chip.classList.remove('active');
                } else {
                    currentFilters.importance.push(i);
                    chip.classList.add('active');
                }
            } else if (tg) {
                if (currentFilters.tags.includes(tg)) {
                    currentFilters.tags = currentFilters.tags.filter(x => x !== tg);
                    chip.classList.remove('active');
                } else {
                    currentFilters.tags.push(tg);
                    chip.classList.add('active');
                }
            }
            currentPage = 0;
            loadMemories();
        }

        // === SEARCH with debounce + highlighting ===
        let searchTimer = null;
        document.getElementById('search-input').addEventListener('input', (e) => {
            currentFilters.search = e.target.value;
            currentPage = 0;
            clearTimeout(searchTimer);
            searchTimer = setTimeout(() => loadMemories(), 200);
        });
        document.getElementById('sort-select').addEventListener('change', (e) => {
            currentFilters.sort = e.target.value;
            loadMemories();
        });

        function resetFilters() {
            currentFilters = { search: '', types: [], importance: [], tags: [], sort: 'updated_desc' };
            document.getElementById('search-input').value = '';
            document.getElementById('sort-select').value = 'updated_desc';
            document.querySelectorAll('.filter-chip.active').forEach(c => c.classList.remove('active'));
            currentPage = 0;
            loadMemories();
        }

        // === MEMORIES LIST ===
        async function loadMemories() {
            const listEl = document.getElementById('memories-list');
            const isSearch = currentFilters.search || currentFilters.types.length || currentFilters.importance.length || currentFilters.tags.length;

            if (!isSearch && !currentFilters.sort.includes('importance') && !currentFilters.sort.includes('access') && !currentFilters.sort.includes('title')) {
                // Use simple list endpoint
                listEl.innerHTML = '<div class="skeleton" style="width:80%"></div><div class="skeleton" style="width:60%"></div><div class="skeleton" style="width:70%"></div>';
                try {
                    const url = new URL(`${API}/api/v1/memories`, window.location.origin);
                    url.searchParams.set('project', PROJECT);
                    url.searchParams.set('limit', String(PAGE_SIZE));
                    url.searchParams.set('offset', String(currentPage * PAGE_SIZE));
                    if (currentFilters.types.length === 1) url.searchParams.set('type', currentFilters.types[0]);
                    if (currentFilters.importance.length === 1) url.searchParams.set('importance', currentFilters.importance[0]);
                    const r = await fetch(url);
                    const j = await r.json();
                    renderMemories(Array.isArray(j) ? j : []);
                } catch (e) {
                    listEl.innerHTML = `<div class="empty">Error: ${e.message}</div>`;
                }
            } else {
                // Use search endpoint
                listEl.innerHTML = '<div class="skeleton" style="width:80%"></div><div class="skeleton" style="width:60%"></div>';
                try {
                    const r = await fetch(`${API}/api/v1/memories/search`, {
                        method: 'POST',
                        headers: {'Content-Type': 'application/json'},
                        body: JSON.stringify({
                            text: currentFilters.search || '',
                            project: PROJECT,
                            limit: PAGE_SIZE
                        })
                    });
                    const j = await r.json();
                    // /search returns {results: [...]} or directly [...]
                    const results = Array.isArray(j) ? j : (j.results || []);
                    renderMemories(results.map(s => ({ ...s.memory, _score: s.score, _snippet: s.snippet })));
                } catch (e) {
                    listEl.innerHTML = `<div class="empty">Error: ${e.message}</div>`;
                }
            }
        }

        function renderMemories(memories) {
            const listEl = document.getElementById('memories-list');
            document.getElementById('results-count').textContent = `${memories.length} resultado${memories.length === 1 ? '' : 's'}`;
            document.getElementById('page-info').textContent = `Página ${currentPage + 1}`;

            if (memories.length === 0) {
                listEl.innerHTML = '<div class="empty">No se encontraron memorias con los filtros actuales.</div>';
                return;
            }

            listEl.innerHTML = memories.map(m => {
                const imp = m.importance || 'medium';
                const impPill = `<span class="pill pill-${imp}">${imp.toUpperCase()}</span>`;
                const typePill = `<span class="type-pill">${m.memory_type || 'note'}</span>`;
                const tags = (m.tags || []).slice(0, 5).map(t => `<span class="tag">#${escapeHtml(t)}</span>`).join('');
                const moreTags = (m.tags || []).length > 5 ? `<span class="tag">+${(m.tags || []).length - 5}</span>` : '';
                const date = m.updated_at ? new Date(m.updated_at).toLocaleString() : '';
                const score = m._score ? `<span class="tag" style="background:#238636; color:#fff;">score: ${m._score.toFixed(2)}</span>` : '';
                const snippet = m._snippet ? `<div class="memory-content">${highlightTerms(m._snippet)}</div>` :
                    `<div class="memory-content">${highlightTerms(m.content || '').substring(0, 200)}</div>`;
                return `<div class="memory-item" onclick="showDetail('${m.id}')">
                    <div class="memory-title">${typePill} ${impPill} ${score} ${escapeHtml(m.title || 'untitled')}</div>
                    ${snippet}
                    <div class="memory-meta">${tags}${moreTags}<span style="margin-left:auto;">${date}</span></div>
                </div>`;
            }).join('');
        }

        function highlightTerms(text) {
            if (!currentFilters.search) return escapeHtml(text);
            const terms = currentFilters.search.split(/\s+/).filter(t => t.length > 2);
            let result = escapeHtml(text);
            terms.forEach(term => {
                const regex = new RegExp(`(${escapeRegex(term)})`, 'gi');
                result = result.replace(regex, '<span class="highlight">$1</span>');
            });
            return result;
        }

        function escapeRegex(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

        function prevPage() { if (currentPage > 0) { currentPage--; loadMemories(); } }
        function nextPage() { currentPage++; loadMemories(); }

        // === MEMORY DETAIL ===
        async function showDetail(id) {
            const panel = document.getElementById('detail-panel');
            const content = document.getElementById('detail-content');
            panel.style.display = 'block';
            content.innerHTML = '<div class="skeleton"></div><div class="skeleton"></div><div class="skeleton"></div>';
            panel.scrollIntoView({ behavior: 'smooth' });
            try {
                const [memResp, entitiesResp] = await Promise.all([
                    fetch(`${API}/api/v1/memories/${id}`).then(r => r.json()),
                    fetch(`${API}/api/v1/memories/${id}/entities`).then(r => r.ok ? r.json() : []).catch(() => [])
                ]);
                if (!memResp || memResp.error) {
                    content.innerHTML = `<div class="empty">No se pudo cargar la memoria.</div>`;
                    return;
                }
                document.getElementById('detail-title').textContent = memResp.title || 'untitled';
                const m = memResp;
                const what = m.what ? `<div class="detail-row"><div class="detail-label">What</div><div class="detail-value">${escapeHtml(m.what)}</div></div>` : '';
                const why = m.why ? `<div class="detail-row"><div class="detail-label">Why</div><div class="detail-value">${escapeHtml(m.why)}</div></div>` : '';
                const ctx = m.context ? `<div class="detail-row"><div class="detail-label">Context</div><div class="detail-value">${escapeHtml(m.context)}</div></div>` : '';
                const learned = m.learned ? `<div class="detail-row"><div class="detail-label">Learned</div><div class="detail-value">${escapeHtml(m.learned)}</div></div>` : '';
                const tags = (m.tags || []).map(t => `<span class="tag">#${escapeHtml(t)}</span>`).join('') || '<span class="empty">No tags</span>';
                const entities = (Array.isArray(entitiesResp) ? entitiesResp : []).map(e =>
                    `<span class="entity-link" onclick="searchByEntity('${escapeHtml(e.entity_name)}')">${escapeHtml(e.entity_name)} <small>(${e.entity_type})</small></span>`
                ).join(' | ') || '<span class="empty">No entities</span>';
                const valid = (m.valid_from || m.valid_until) ?
                    `<div class="detail-row"><div class="detail-label">Validez temporal</div><div class="detail-value">${m.valid_from || '∞'} → ${m.valid_until || '∞'}</div></div>` : '';

                content.innerHTML = `
                    <div class="detail-row"><div class="detail-label">Tipo / Importancia</div><div class="detail-value"><span class="type-pill">${m.memory_type}</span> <span class="pill pill-${m.importance}">${m.importance.toUpperCase()}</span></div></div>
                    <div class="detail-row"><div class="detail-label">Contenido</div><div class="detail-value" style="white-space:pre-wrap;">${escapeHtml(m.content)}</div></div>
                    ${what}${why}${ctx}${learned}
                    <div class="detail-row"><div class="detail-label">Tags</div><div class="detail-value">${tags}</div></div>
                    <div class="detail-row"><div class="detail-label">Entidades</div><div class="detail-value">${entities}</div></div>
                    <div class="detail-row"><div class="detail-label">Accesos / Revisiones</div><div class="detail-value">${m.access_count} accesos · ${m.revision_count} revisiones · ${m.duplicate_count} duplicados</div></div>
                    <div class="detail-row"><div class="detail-label">Fechas</div><div class="detail-value">Creado: ${m.created_at ? new Date(m.created_at).toLocaleString() : '?'}<br>Actualizado: ${m.updated_at ? new Date(m.updated_at).toLocaleString() : '?'}</div></div>
                    ${valid}
                    <div class="detail-row" style="margin-top:16px;">
                        <button onclick="deleteMemory('${m.id}')" style="background:#da3633;">Eliminar</button>
                    </div>
                `;
            } catch (e) {
                content.innerHTML = `<div class="empty">Error: ${e.message}</div>`;
            }
        }

        async function deleteMemory(id) {
            if (!confirm('¿Eliminar esta memoria?')) return;
            try {
                await fetch(`${API}/api/v1/memories/${id}?hard=false`, { method: 'DELETE' });
                document.getElementById('detail-panel').style.display = 'none';
                loadMemories();
                loadStats();
            } catch (e) {
                alert('Error: ' + e.message);
            }
        }

        function searchByEntity(entity) {
            currentFilters.search = entity;
            document.getElementById('search-input').value = entity;
            currentPage = 0;
            switchTab('all');
            loadMemories();
        }

        // === ENTITIES TAB ===
        async function loadEntities() {
            const el = document.getElementById('entities-list');
            el.innerHTML = '<div class="skeleton"></div><div class="skeleton"></div>';
            try {
                const memories = await fetch(`${API}/api/v1/memories?project=${PROJECT}&limit=500`).then(r => r.json());
                if (!Array.isArray(memories)) {
                    el.innerHTML = '<div class="empty">No se pudieron cargar entidades.</div>';
                    return;
                }
                const counts = new Map();
                memories.forEach(m => (m.tags || []).forEach(t => {
                    counts.set(t, (counts.get(t) || 0) + 1);
                }));
                const sorted = [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 50);
                el.innerHTML = sorted.map(([name, count]) =>
                    `<div class="memory-item" onclick="searchByEntity('${escapeHtml(name)}')" style="display:flex; justify-content:space-between;">
                        <span class="entity-link">${escapeHtml(name)}</span>
                        <span class="tag">${count} memorias</span>
                    </div>`
                ).join('') || '<div class="empty">Sin entidades extraídas</div>';
            } catch (e) {
                el.innerHTML = `<div class="empty">Error: ${e.message}</div>`;
            }
        }

        // === TEMPORAL TAB ===
        async function loadTemporal() {
            const statsEl = document.getElementById('temporal-stats');
            const memsEl = document.getElementById('temporal-memories');
            statsEl.innerHTML = '<div class="skeleton"></div>';
            memsEl.innerHTML = '';
            try {
                const memories = await fetch(`${API}/api/v1/memories?project=${PROJECT}&limit=500`).then(r => r.json());
                if (!Array.isArray(memories)) {
                    statsEl.innerHTML = '<div class="empty">No se pudieron cargar memorias.</div>';
                    return;
                }
                const now = new Date();
                let valid = 0, expired = 0, total_with_window = 0;
                const temporal = memories.filter(m => m.valid_from || m.valid_until);
                temporal.forEach(m => {
                    total_with_window++;
                    const inWindow = (!m.valid_from || new Date(m.valid_from) <= now) &&
                                     (!m.valid_until || new Date(m.valid_until) > now);
                    if (inWindow) valid++; else expired++;
                });
                statsEl.innerHTML = `
                    <div class="stats-grid">
                        <div class="stat-card"><div class="value">${total_with_window}</div><div class="label">Con ventana</div></div>
                        <div class="stat-card"><div class="value" style="color:#7ee787;">${valid}</div><div class="label">Válidas</div></div>
                        <div class="stat-card"><div class="value" style="color:#ff7b72;">${expired}</div><div class="label">Expiradas</div></div>
                    </div>
                `;
                memsEl.innerHTML = temporal.slice(0, 30).map(m => {
                    const vf = m.valid_from ? new Date(m.valid_from).toLocaleDateString() : '∞';
                    const vu = m.valid_until ? new Date(m.valid_until).toLocaleDateString() : '∞';
                    return `<div class="memory-item" onclick="showDetail('${m.id}')">
                        <div class="memory-title">${escapeHtml(m.title)}</div>
                        <div class="memory-meta">${vf} → ${vu}</div>
                    </div>`;
                }).join('') || '<div class="empty">No hay memorias con ventana de validez.</div>';
            } catch (e) {
                statsEl.innerHTML = `<div class="empty">Error: ${e.message}</div>`;
            }
        }

        // === UTILS ===
        function escapeHtml(s) {
            if (s == null) return '';
            return String(s).replace(/[&<>"']/g, c => ({
                '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
            }[c]));
        }

        // === INIT ===
        document.addEventListener('DOMContentLoaded', () => {
            loadStats();
            loadFilterOptions().then(loadMemories);
            setInterval(loadStats, 30000);
        });
    </script>
</body>
</html>"#;

/// Renderiza el HTML del dashboard con el proyecto por defecto inyectado.
pub fn render_dashboard(default_project: &str) -> String {
    DASHBOARD_HTML.replace("__DEFAULT_PROJECT__", default_project)
}
