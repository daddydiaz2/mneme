use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::entities::{EntitySearchResult, EntityType};
use crate::store::memory::{GraphData, Memory, SearchQuery};
use crate::store::memory::{MemoryStats, ProjectSummary};

/// Datos para vista de entidades.
pub struct EntityGraphData {
    pub frequent_entities: Vec<(String, EntityType, u32)>,
    pub selected_memories: Vec<EntitySearchResult>,
    pub selected: usize,
}

/// Datos para vista temporal.
pub struct TemporalData {
    pub memories: Vec<Memory>,
    pub reference_time: DateTime<Utc>,
    pub display_mode: u8,
}

/// Pestañas disponibles en el panel de detalle.
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
            DetailTab::Content => " Content ",
            DetailTab::Structured => " Fields ",
            DetailTab::Entities => " Entities ",
            DetailTab::Temporal => " Temporal ",
            DetailTab::Relations => " Graph ",
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            DetailTab::Content => DetailTab::Relations,
            DetailTab::Structured => DetailTab::Content,
            DetailTab::Entities => DetailTab::Structured,
            DetailTab::Temporal => DetailTab::Entities,
            DetailTab::Relations => DetailTab::Temporal,
        }
    }
    pub fn next(&self) -> Self {
        match self {
            DetailTab::Content => DetailTab::Structured,
            DetailTab::Structured => DetailTab::Entities,
            DetailTab::Entities => DetailTab::Temporal,
            DetailTab::Temporal => DetailTab::Relations,
            DetailTab::Relations => DetailTab::Content,
        }
    }
}

/// Modos de operación de la app.
pub enum AppMode {
    Normal,
    Searching,
    Confirming {
        action: String,
        memory_id: uuid::Uuid,
    },
    Help,
    Graph,
    EntityGraph,
    Temporal,
}

/// Estado global.
pub struct App {
    pub mode: AppMode,
    pub project: String,
    pub memories: Vec<Memory>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_query: String,
    pub status_message: Option<String>,
    pub should_quit: bool,
    pub graph_data: Option<GraphData>,
    pub graph_selected: usize,
    pub entity_graph_data: Option<EntityGraphData>,
    pub temporal_data: Option<TemporalData>,
    pub detail_scroll: usize,
    pub detail_tab: DetailTab,
    pub stats: Option<MemoryStats>,
    pub projects: Vec<ProjectSummary>,
    pub total_memory_count: u32,
    pub db: Arc<Database>,
    #[allow(dead_code)]
    pub settings: Arc<Settings>,
}

impl App {
    pub fn new(db: Arc<Database>, settings: Arc<Settings>) -> crate::error::Result<Self> {
        let project = Settings::infer_project();
        Ok(Self {
            mode: AppMode::Normal,
            project,
            memories: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            search_query: String::new(),
            status_message: None,
            should_quit: false,
            graph_data: None,
            graph_selected: 0,
            entity_graph_data: None,
            temporal_data: None,
            detail_scroll: 0,
            detail_tab: DetailTab::Content,
            stats: None,
            projects: Vec::new(),
            total_memory_count: 0,
            db,
            settings,
        })
    }

    /// Carga memorias + stats en paralelo.
    pub fn load_memories(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        let memories = if self.search_query.is_empty() {
            store.list(&self.project, None, None, None, 500, 0)?
        } else {
            let query = SearchQuery {
                text: self.search_query.clone(),
                project: Some(self.project.clone()),
                scope: None,
                memory_type: None,
                importance: None,
                tags: Vec::new(),
                limit: 200,
                include_snippet: false,
                all_projects: false,
            };
            let weights = crate::store::search::SearchWeights::default();
            store
                .search(&query, &weights, None)?
                .into_iter()
                .map(|r| r.memory)
                .collect()
        };
        self.memories = memories;
        self.selected = self.selected.min(self.memories.len().saturating_sub(1));
        self.detail_scroll = 0;

        // Siempre refrescar stats
        self.stats = store.stats(&self.project).ok();
        self.projects = store.list_projects().unwrap_or_default();
        self.total_memory_count = self.stats.as_ref().map(|s| s.total_memories).unwrap_or(0);
        Ok(())
    }

    // ===== GRAFO =====
    pub fn load_graph(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        self.graph_data = Some(store.get_graph(&self.project)?);
        self.graph_selected = 0;
        Ok(())
    }
    pub fn toggle_graph(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::Graph => self.mode = AppMode::Normal,
            _ => {
                self.load_graph()?;
                self.mode = AppMode::Graph;
            }
        }
        Ok(())
    }
    pub fn graph_next(&mut self) {
        if let Some(ref data) = self.graph_data {
            if !data.nodes.is_empty() {
                self.graph_selected = (self.graph_selected + 1) % data.nodes.len();
            }
        }
    }
    pub fn graph_prev(&mut self) {
        if let Some(ref data) = self.graph_data {
            if !data.nodes.is_empty() {
                self.graph_selected = self
                    .graph_selected
                    .checked_sub(1)
                    .unwrap_or(data.nodes.len() - 1);
            }
        }
    }

    // ===== ENTIDADES =====
    pub fn load_entity_graph(&mut self) -> crate::error::Result<()> {
        let entity_store = self.db.entities();
        let frequent = entity_store.frequent_entities(&self.project, 30)?;
        self.entity_graph_data = Some(EntityGraphData {
            frequent_entities: frequent,
            selected_memories: Vec::new(),
            selected: 0,
        });
        Ok(())
    }
    pub fn toggle_entity_graph(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::EntityGraph => self.mode = AppMode::Normal,
            _ => {
                self.load_entity_graph()?;
                self.mode = AppMode::EntityGraph;
            }
        }
        Ok(())
    }

    // ===== TEMPORAL =====
    pub fn load_temporal(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        let memories = store.list(&self.project, None, None, None, 500, 0)?;
        self.temporal_data = Some(TemporalData {
            memories,
            reference_time: Utc::now(),
            display_mode: 0,
        });
        Ok(())
    }
    pub fn toggle_temporal(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::Temporal => self.mode = AppMode::Normal,
            _ => {
                self.load_temporal()?;
                self.mode = AppMode::Temporal;
            }
        }
        Ok(())
    }
    pub fn temporal_cycle_mode(&mut self) {
        if let Some(ref mut data) = self.temporal_data {
            data.display_mode = (data.display_mode + 1) % 3;
        }
    }

    // ===== NAVEGACIÓN =====
    pub fn select_next(&mut self) {
        if !self.memories.is_empty() {
            self.selected = (self.selected + 1).min(self.memories.len() - 1);
            self.ensure_selected_visible(18);
        }
    }
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.ensure_selected_visible(18);
    }
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }
    pub fn select_last(&mut self) {
        if !self.memories.is_empty() {
            self.selected = self.memories.len() - 1;
            self.ensure_selected_visible(18);
        }
    }
    pub fn page_down(&mut self) {
        let p = 18usize;
        if !self.memories.is_empty() {
            self.selected = (self.selected + p).min(self.memories.len() - 1);
            self.ensure_selected_visible(p);
        }
    }
    pub fn page_up(&mut self) {
        let p = 18usize;
        self.selected = self.selected.saturating_sub(p);
        self.ensure_selected_visible(p);
    }

    /// Scroll en panel de detalle
    pub fn detail_scroll_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(3);
    }
    pub fn detail_scroll_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(3);
    }
    pub fn detail_next_tab(&mut self) {
        self.detail_tab = self.detail_tab.next();
        self.detail_scroll = 0;
    }
    pub fn detail_prev_tab(&mut self) {
        self.detail_tab = self.detail_tab.prev();
        self.detail_scroll = 0;
    }

    pub fn selected_memory(&self) -> Option<&Memory> {
        self.memories.get(self.selected)
    }

    // ===== SEARCH =====
    pub fn start_search(&mut self) {
        self.mode = AppMode::Searching;
        self.search_query.clear();
    }
    pub fn confirm_search(&mut self) -> crate::error::Result<()> {
        self.mode = AppMode::Normal;
        self.load_memories()
    }
    pub fn cancel_search(&mut self) {
        self.mode = AppMode::Normal;
        self.search_query.clear();
        let _ = self.load_memories();
    }
    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }
    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
    }

    // ===== DELETE =====
    pub fn delete_selected(&mut self) -> crate::error::Result<()> {
        if let Some(memory) = self.selected_memory() {
            self.mode = AppMode::Confirming {
                action: "delete".to_string(),
                memory_id: memory.id,
            };
        }
        Ok(())
    }
    pub fn confirm_action(&mut self) -> crate::error::Result<()> {
        if let AppMode::Confirming { action, memory_id } = &self.mode {
            if action == "delete" {
                let store = self.db.memories();
                store.delete(*memory_id, false)?;
                self.status_message = Some("🗑 Eliminada".to_string());
                self.load_memories()?;
            }
            self.mode = AppMode::Normal;
        }
        Ok(())
    }
    pub fn cancel_confirm(&mut self) {
        self.mode = AppMode::Normal;
    }

    // ===== HELP / QUIT =====
    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,
            _ => AppMode::Help,
        };
    }
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    fn ensure_selected_visible(&mut self, visible: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible {
            self.scroll_offset = self.selected.saturating_sub(visible - 1);
        }
    }
}
