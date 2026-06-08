/// Web dashboard HTML for mneme.
/// Sirve una UI estática en `/` del servidor HTTP.
/// NO se abre automáticamente en el navegador (mneme es una API + MCP, no una web app).
pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>mneme — Memory Dashboard</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 24px;
            line-height: 1.5;
        }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: #58a6ff; margin-bottom: 8px; font-size: 28px; }
        .subtitle { color: #8b949e; margin-bottom: 24px; font-size: 14px; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 16px; margin-bottom: 24px; }
        .card {
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 8px;
            padding: 16px;
        }
        .card h2 { color: #f0f6fc; font-size: 14px; margin-bottom: 12px; text-transform: uppercase; letter-spacing: 0.5px; }
        .stat { font-size: 32px; font-weight: 600; color: #f0f6fc; }
        .stat-label { color: #8b949e; font-size: 12px; margin-top: 4px; }
        .endpoint {
            background: #0d1117;
            border: 1px solid #21262d;
            border-radius: 4px;
            padding: 8px 12px;
            margin-bottom: 6px;
            font-family: 'SF Mono', Monaco, Consolas, monospace;
            font-size: 12px;
        }
        .method-get { color: #79c0ff; }
        .method-post { color: #7ee787; }
        .method-put { color: #ffa657; }
        .method-delete { color: #ff7b72; }
        .memory-item {
            background: #0d1117;
            border: 1px solid #21262d;
            border-radius: 4px;
            padding: 10px 12px;
            margin-bottom: 6px;
        }
        .memory-title { color: #f0f6fc; font-size: 13px; margin-bottom: 4px; }
        .memory-meta { color: #8b949e; font-size: 11px; }
        .tag {
            display: inline-block;
            background: #1f6feb33;
            color: #79c0ff;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 10px;
            margin-right: 4px;
        }
        .pill {
            display: inline-block;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 10px;
            font-weight: 600;
        }
        .pill-critical { background: #da3633; color: #fff; }
        .pill-high { background: #d29922; color: #fff; }
        .pill-medium { background: #6e7681; color: #fff; }
        .pill-low { background: #21262d; color: #8b949e; }
        .footer { color: #6e7681; font-size: 12px; text-align: center; margin-top: 32px; padding-top: 16px; border-top: 1px solid #21262d; }
        .empty { color: #6e7681; font-style: italic; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧠 mneme</h1>
        <p class="subtitle">Persistent memory for AI agents · v0.1.0 · MCP + HTTP API + TUI</p>

        <div class="grid">
            <div class="card">
                <h2>📊 Stats</h2>
                <div class="stat" id="stat-memories">—</div>
                <div class="stat-label">memories in this project</div>
            </div>
            <div class="card">
                <h2>🔍 Search</h2>
                <input type="text" id="search-input" placeholder="Search memories..."
                       style="width:100%; padding:8px; background:#0d1117; border:1px solid #30363d; color:#c9d1d9; border-radius:4px; margin-bottom:8px;">
                <button onclick="searchMemories()"
                        style="width:100%; padding:8px; background:#238636; border:0; color:#fff; border-radius:4px; cursor:pointer;">
                    Search
                </button>
            </div>
            <div class="card">
                <h2>🛠 Tools</h2>
                <p class="empty" style="font-size:12px;">MCP tools available</p>
                <div class="endpoint" style="font-size:11px;">64 tools</div>
                <div class="endpoint" style="font-size:11px;">mcp://stdio</div>
            </div>
        </div>

        <div class="card">
            <h2>📚 Recent Memories</h2>
            <div id="memories-list">
                <p class="empty">Loading...</p>
            </div>
        </div>

        <div class="card" style="margin-top:16px;">
            <h2>🌐 API Endpoints</h2>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/memories — list memories</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/memories — create memory</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/memories/:id — get memory by id</div>
            <div class="endpoint"><span class="method-put">PUT</span> /api/v1/memories/:id — update memory</div>
            <div class="endpoint"><span class="method-delete">DELETE</span> /api/v1/memories/:id — delete memory</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/memories/search — hybrid search</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/memories/similar — semantic similar</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/memories/batch — batch save</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/stats — project stats</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/projects — list projects</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/context — recent context</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/graph — knowledge graph</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/audit — quality audit</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/knowledge-gaps — coverage analysis</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/cloud/enroll — cloud enrollment</div>
            <div class="endpoint"><span class="method-post">POST</span> /api/v1/cloud/sync — cloud sync</div>
            <div class="endpoint"><span class="method-get">GET</span> /api/v1/cloud/status — cloud status</div>
        </div>

        <p class="footer">
            mneme v0.1.0 · mneme.dev ·
            <a href="https://github.com/daddydiaz2/mneme" style="color:#58a6ff; text-decoration:none;">GitHub</a> ·
            <a href="https://headroom-docs.vercel.app/docs" style="color:#58a6ff; text-decoration:none;">Docs</a>
        </p>
    </div>

    <script>
        const API_BASE = '';
        const DEFAULT_PROJECT = '__DEFAULT_PROJECT__';

        async function loadStats() {
            try {
                const r = await fetch(`${API_BASE}/api/v1/stats?project=${DEFAULT_PROJECT}`);
                const j = await r.json();
                document.getElementById('stat-memories').textContent = j.total_memories ?? '0';
            } catch (e) {
                document.getElementById('stat-memories').textContent = '?';
            }
        }

        async function loadMemories() {
            const container = document.getElementById('memories-list');
            try {
                const r = await fetch(`${API_BASE}/api/v1/memories?project=${DEFAULT_PROJECT}&limit=20`);
                const j = await r.json();
                if (!j || j.length === 0) {
                    container.innerHTML = '<p class="empty">No memories yet. Save some via the MCP or API.</p>';
                    return;
                }
                container.innerHTML = j.map(m => {
                    const impPill = `<span class="pill pill-${m.importance}">${m.importance.toUpperCase()}</span>`;
                    const typePill = `<span class="tag">${m.memory_type}</span>`;
                    const tags = (m.tags || []).map(t => `<span class="tag">#${t}</span>`).join('');
                    return `<div class="memory-item">
                        <div class="memory-title">${typePill} ${impPill} ${escapeHtml(m.title)}</div>
                        <div class="memory-meta">${tags}<span style="margin-left:8px;">${new Date(m.updated_at).toLocaleString()}</span></div>
                    </div>`;
                }).join('');
            } catch (e) {
                container.innerHTML = '<p class="empty">Failed to load memories: ' + e.message + '</p>';
            }
        }

        async function searchMemories() {
            const q = document.getElementById('search-input').value;
            if (!q) { loadMemories(); return; }
            const container = document.getElementById('memories-list');
            try {
                const r = await fetch(`${API_BASE}/api/v1/memories/search`, {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({query: q, project: DEFAULT_PROJECT, limit: 20})
                });
                const j = await r.json();
                if (!j || j.length === 0) {
                    container.innerHTML = '<p class="empty">No results for "' + escapeHtml(q) + '".</p>';
                    return;
                }
                container.innerHTML = j.map(r => {
                    const impPill = `<span class="pill pill-${r.memory.importance}">${r.memory.importance.toUpperCase()}</span>`;
                    const typePill = `<span class="tag">${r.memory.memory_type}</span>`;
                    const tags = (r.memory.tags || []).map(t => `<span class="tag">#${t}</span>`).join('');
                    const snippet = r.snippet ? `<div class="memory-meta" style="margin-top:4px;">${escapeHtml(r.snippet)}</div>` : '';
                    return `<div class="memory-item">
                        <div class="memory-title">${typePill} ${impPill} ${escapeHtml(r.memory.title)} <span style="color:#8b949e; font-size:11px;">(score: ${r.score.toFixed(3)})</span></div>
                        <div class="memory-meta">${tags}${snippet}</div>
                    </div>`;
                }).join('');
            } catch (e) {
                container.innerHTML = '<p class="empty">Search failed: ' + e.message + '</p>';
            }
        }

        function escapeHtml(s) {
            if (s == null) return '';
            return String(s).replace(/[&<>"']/g, c => ({
                '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
            }[c]));
        }

        document.getElementById('search-input').addEventListener('keydown', e => {
            if (e.key === 'Enter') searchMemories();
        });

        loadStats();
        loadMemories();
        setInterval(loadStats, 30000);
    </script>
</body>
</html>"#;

/// Renderiza el HTML del dashboard con el proyecto por defecto inyectado.
pub fn render_dashboard(default_project: &str) -> String {
    DASHBOARD_HTML.replace("__DEFAULT_PROJECT__", default_project)
}
