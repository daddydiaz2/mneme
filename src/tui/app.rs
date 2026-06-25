use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::entities::{EntitySearchResult, EntityType};
use crate::store::memory::{GraphData, Memory, MemoryStats, ProjectSummary, SearchQuery, Session};

/// Tabs del panel de detalle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Content,
    Structured,
    Entities,
    Temporal,
    Relations,
}
impl DetailTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Content => "Content",
            Self::Structured => "Fields",
            Self::Entities => "Entities",
            Self::Temporal => "Temporal",
            Self::Relations => "Graph",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Content => Self::Structured,
            Self::Structured => Self::Entities,
            Self::Entities => Self::Temporal,
            Self::Temporal => Self::Relations,
            Self::Relations => Self::Content,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::Content => Self::Relations,
            Self::Structured => Self::Content,
            Self::Entities => Self::Structured,
            Self::Temporal => Self::Entities,
            Self::Relations => Self::Temporal,
        }
    }
}

/// Estado global de la app — estilo lazygit/lazydocker.
pub struct App {
    pub project: String,
    pub memories: Vec<Memory>,
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub scroll: usize,
    pub search: String,
    pub status_msg: Option<String>,
    pub quit: bool,
    pub show_help: bool,
    pub detail_tab: DetailTab,
    pub detail_scroll: usize,
    pub stats: Option<MemoryStats>,
    pub graph: Option<GraphData>,
    pub graph_sel: usize,
    pub entity_data: Option<Vec<(String, EntityType, u32)>>,
    pub temporal_data: Option<(Vec<Memory>, u8)>,
    pub active_panel: usize, // 0=lista, 1=detalle, 2=search
    pub total_mems: u32,
    pub db: Arc<Database>,
}

impl App {
    pub fn new(db: Arc<Database>, _settings: Arc<Settings>) -> Self {
        let project = Settings::infer_project();
        Self {
            project,
            memories: Vec::new(),
            sessions: Vec::new(),
            selected: 0,
            scroll: 0,
            search: String::new(),
            status_msg: None,
            quit: false,
            show_help: false,
            detail_tab: DetailTab::Content,
            detail_scroll: 0,
            stats: None,
            graph: None,
            graph_sel: 0,
            entity_data: None,
            temporal_data: None,
            active_panel: 0,
            total_mems: 0,
            db,
        }
    }

    // ── CARGA ──
    pub fn load(&mut self) {
        let s = self.db.memories();
        if self.search.is_empty() {
            self.memories = s
                .list(&self.project, None, None, None, 500, 0)
                .unwrap_or_default();
        } else {
            let q = SearchQuery {
                text: self.search.clone(),
                project: Some(self.project.clone()),
                scope: None,
                memory_type: None,
                importance: None,
                tags: vec![],
                limit: 200,
                include_snippet: false,
                all_projects: false,
            };
            self.memories = s
                .search(&q, &crate::store::search::SearchWeights::default(), None)
                .unwrap_or_default()
                .into_iter()
                .map(|r| r.memory)
                .collect();
        }
        self.selected = self.selected.min(self.memories.len().saturating_sub(1));
        self.scroll = 0;
        self.detail_scroll = 0;
        self.stats = s.stats(&self.project).ok();
        self.total_mems = self.stats.as_ref().map(|s| s.total_memories).unwrap_or(0);
        self.sessions = self
            .db
            .sessions()
            .list(&self.project, 50)
            .unwrap_or_default();
    }

    // ── NAV ──
    pub fn down(&mut self) {
        if !self.memories.is_empty() {
            self.selected = (self.selected + 1).min(self.memories.len() - 1);
            if self.selected >= self.scroll + 20 {
                self.scroll += 1;
            }
        }
    }
    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        if self.selected < self.scroll {
            self.scroll = self.scroll.saturating_sub(1);
        }
    }
    pub fn first(&mut self) {
        self.selected = 0;
        self.scroll = 0;
    }
    pub fn last(&mut self) {
        if !self.memories.is_empty() {
            self.selected = self.memories.len() - 1;
            self.scroll = self.selected.saturating_sub(19);
        }
    }
    pub fn pgdn(&mut self) {
        if !self.memories.is_empty() {
            self.selected = (self.selected + 20).min(self.memories.len() - 1);
            if self.selected >= self.scroll + 20 {
                self.scroll += 20;
            }
        }
    }
    pub fn pgup(&mut self) {
        self.selected = self.selected.saturating_sub(20);
        if self.selected < self.scroll {
            self.scroll = self.scroll.saturating_sub(20);
        }
    }
    pub fn sel(&self) -> Option<&Memory> {
        self.memories.get(self.selected)
    }

    // ── DETAIL ──
    pub fn tab_next(&mut self) {
        self.detail_tab = self.detail_tab.next();
        self.detail_scroll = 0;
    }
    pub fn tab_prev(&mut self) {
        self.detail_tab = self.detail_tab.prev();
        self.detail_scroll = 0;
    }
    pub fn dscroll_down(&mut self) {
        self.detail_scroll += 3;
    }
    pub fn dscroll_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(3);
    }

    // ── ACTIONS ──
    pub fn delete_sel(&mut self) {
        if let Some(m) = self.sel() {
            self.db.memories().delete(m.id, false).ok();
            self.status_msg = Some("🗑 Deleted".into());
            self.load();
        }
    }
    pub fn load_graph(&mut self) {
        self.graph = self.db.memories().get_graph(&self.project).ok();
        self.graph_sel = 0;
    }
    pub fn graph_next(&mut self) {
        if let Some(ref d) = self.graph {
            if !d.nodes.is_empty() {
                self.graph_sel = (self.graph_sel + 1) % d.nodes.len();
            }
        }
    }
    pub fn graph_prev(&mut self) {
        if let Some(ref d) = self.graph {
            if !d.nodes.is_empty() {
                self.graph_sel = self.graph_sel.checked_sub(1).unwrap_or(d.nodes.len() - 1);
            }
        }
    }
    pub fn load_entity(&mut self) {
        self.entity_data = self.db.entities().frequent_entities(&self.project, 30).ok();
    }
    pub fn load_temporal(&mut self) {
        self.temporal_data = Some((
            self.db
                .memories()
                .list(&self.project, None, None, None, 500, 0)
                .unwrap_or_default(),
            0,
        ));
    }
    pub fn temporal_cycle(&mut self) {
        if let Some(ref mut td) = self.temporal_data {
            td.1 = (td.1 + 1) % 3;
        }
    }
}
