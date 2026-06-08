use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::entities::{EntitySearchResult, EntityType};
use crate::store::memory::{GraphData, Memory, MemoryStats, ProjectSummary, SearchQuery, Session, UserPrompt};

pub struct EntityGraphData {
    pub frequent_entities: Vec<(String, EntityType, u32)>,
    pub selected_memories: Vec<EntitySearchResult>,
    pub selected: usize,
}

pub struct TemporalData {
    pub memories: Vec<Memory>,
    pub reference_time: DateTime<Utc>,
    pub display_mode: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetailTab {
    Content, Structured, Entities, Temporal, Relations,
}
impl DetailTab {
    pub fn label(&self) -> &'static str {
        match self { DetailTab::Content=>" Content ", DetailTab::Structured=>" Fields ", DetailTab::Entities=>" Entities ", DetailTab::Temporal=>" Temporal ", DetailTab::Relations=>" Graph " }
    }
    pub fn prev(&self) -> Self {
        match self { DetailTab::Content=>DetailTab::Relations, DetailTab::Structured=>DetailTab::Content, DetailTab::Entities=>DetailTab::Structured, DetailTab::Temporal=>DetailTab::Entities, DetailTab::Relations=>DetailTab::Temporal }
    }
    pub fn next(&self) -> Self {
        match self { DetailTab::Content=>DetailTab::Structured, DetailTab::Structured=>DetailTab::Entities, DetailTab::Entities=>DetailTab::Temporal, DetailTab::Temporal=>DetailTab::Relations, DetailTab::Relations=>DetailTab::Content }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Screen {
    Dashboard,
    Memories,
    Sessions,
    SessionDetail,
    Prompts,
    Projects,
    Search,
    AgentSetup,
    Graph,
    EntityGraph,
    Temporal,
}

#[derive(Debug, Clone)]
pub enum Action {
    Search,
    RecentObservations,
    BrowseSessions,
    ViewPrompts,
    Projects,
    AgentPlugin,
    Quit,
}

pub struct App {
    pub screen: Screen,
    pub prev_screen: Screen,
    pub project: String,
    pub memories: Vec<Memory>,
    pub sessions: Vec<Session>,
    pub prompts: Vec<UserPrompt>,
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
    pub selected_session: Option<Session>,
    pub db: Arc<Database>,
    #[allow(dead_code)]
    settings: Arc<Settings>,
}

impl App {
    pub fn new(db: Arc<Database>, settings: Arc<Settings>) -> crate::error::Result<Self> {
        let project = Settings::infer_project();
        Ok(Self {
            screen: Screen::Dashboard,
            prev_screen: Screen::Dashboard,
            project,
            memories: Vec::new(),
            sessions: Vec::new(),
            prompts: Vec::new(),
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
            selected_session: None,
            db,
            settings,
        })
    }

    pub fn navigate(&mut self, screen: Screen) {
        self.prev_screen = self.screen;
        self.screen = screen;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn back(&mut self) {
        let prev = self.prev_screen;
        self.prev_screen = self.screen;
        self.screen = prev;
    }

    // ── DATA LOADING ──
    pub fn load_all(&mut self) { let _ = self.load_memories(); let _ = self.load_sessions(); let _ = self.load_prompts(); }
    pub fn load_memories(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        self.memories = if self.search_query.is_empty() {
            store.list(&self.project, None, None, None, 500, 0)?
        } else {
            let q = SearchQuery {
                text: self.search_query.clone(),
                project: Some(self.project.clone()),
                scope: None, memory_type: None, importance: None, tags: vec![], limit: 200, include_snippet: false, all_projects: false,
            };
            store.search(&q, &crate::store::search::SearchWeights::default(), None)?.into_iter().map(|r| r.memory).collect()
        };
        self.selected = self.selected.min(self.memories.len().saturating_sub(1));
        self.detail_scroll = 0;
        self.stats = store.stats(&self.project).ok();
        self.projects = store.list_projects().unwrap_or_default();
        self.total_memory_count = self.stats.as_ref().map(|s| s.total_memories).unwrap_or(0);
        Ok(())
    }
    pub fn load_sessions(&mut self) {
        self.sessions = self.db.sessions().list(&self.project, 100).unwrap_or_default();
    }
    pub fn load_prompts(&mut self) {
        // Load prompts from sessions
        self.prompts = self.db.memories().list(&self.project, None, None, None, 200, 0).into_iter().flatten().map(|_| UserPrompt {
            id: uuid::Uuid::new_v4(),
            session_id: None,
            content: String::new(),
            project: self.project.clone(),
            created_at: Utc::now(),
        }).collect();
        // If we can't get real prompts, show placeholder
        self.prompts.clear();
    }

    // ── DASHBOARD ACTIONS ──
    pub fn execute_action(&mut self, action: Action) -> crate::error::Result<()> {
        match action {
            Action::Search => { self.search_query.clear(); self.navigate(Screen::Search); }
            Action::RecentObservations => { self.search_query.clear(); self.load_memories()?; self.navigate(Screen::Memories); }
            Action::BrowseSessions => { self.load_sessions(); self.navigate(Screen::Sessions); }
            Action::ViewPrompts => { self.load_prompts(); self.navigate(Screen::Prompts); }
            Action::Projects => { self.navigate(Screen::Projects); }
            Action::AgentPlugin => { self.navigate(Screen::AgentSetup); }
            Action::Quit => self.should_quit = true,
        }
        Ok(())
    }

    // ── SESSION DETAIL ──
    pub fn view_session(&mut self, session: Session) {
        self.selected_session = Some(session);
        self.navigate(Screen::SessionDetail);
    }

    // ── NAVEGACIÓN ──
    pub fn select_next(&mut self) {
        let len = match self.screen {
            Screen::Memories => self.memories.len(),
            Screen::Sessions => self.sessions.len(),
            Screen::Prompts => self.prompts.len(),
            Screen::Projects => self.projects.len(),
            _ => 0,
        };
        if len > 0 { self.selected = (self.selected + 1).min(len - 1); self.ensure_visible(18); }
    }
    pub fn select_prev(&mut self) { self.selected = self.selected.saturating_sub(1); self.ensure_visible(18); }
    pub fn select_first(&mut self) { self.selected = 0; self.scroll_offset = 0; }
    pub fn select_last(&mut self) {
        let len = match self.screen {
            Screen::Memories => self.memories.len(),
            Screen::Sessions => self.sessions.len(),
            _ => 0,
        };
        if len > 0 { self.selected = len - 1; self.ensure_visible(18); }
    }
    pub fn page_down(&mut self) { let p = 18; self.selected = (self.selected + p).min(self.memories.len().saturating_sub(1)); self.ensure_visible(p); }
    pub fn page_up(&mut self) { self.selected = self.selected.saturating_sub(18); self.ensure_visible(18); }
    pub fn detail_scroll_down(&mut self) { self.detail_scroll = self.detail_scroll.saturating_add(3); }
    pub fn detail_scroll_up(&mut self) { self.detail_scroll = self.detail_scroll.saturating_sub(3); }
    pub fn detail_next_tab(&mut self) { self.detail_tab = self.detail_tab.next(); self.detail_scroll = 0; }
    pub fn detail_prev_tab(&mut self) { self.detail_tab = self.detail_tab.prev(); self.detail_scroll = 0; }
    fn ensure_visible(&mut self, visible: usize) {
        if self.selected < self.scroll_offset { self.scroll_offset = self.selected; }
        else if self.selected >= self.scroll_offset + visible { self.scroll_offset = self.selected.saturating_sub(visible - 1); }
    }

    pub fn selected_memory(&self) -> Option<&Memory> { self.memories.get(self.selected) }
    pub fn start_search(&mut self) { self.screen = Screen::Search; self.search_query.clear(); }
    pub fn confirm_search(&mut self) -> crate::error::Result<()> { self.load_memories()?; self.navigate(Screen::Memories); Ok(()) }
    pub fn cancel_search(&mut self) { self.search_query.clear(); let _ = self.load_memories(); self.screen = Screen::Dashboard; }
    pub fn push_search_char(&mut self, c: char) { self.search_query.push(c); }
    pub fn pop_search_char(&mut self) { self.search_query.pop(); }
    pub fn delete_selected(&mut self) -> crate::error::Result<()> {
        if let Some(mem) = self.selected_memory() {
            let id = mem.id;
            self.db.memories().delete(id, false)?;
            self.status_message = Some("Memoria eliminada".to_string());
            self.load_memories()?;
        }
        Ok(())
    }
    pub fn quit(&mut self) { self.should_quit = true; }

    // ── OTHER VIEWS ──
    pub fn load_graph(&mut self) -> crate::error::Result<()> { self.graph_data = Some(self.db.memories().get_graph(&self.project)?); self.graph_selected = 0; Ok(()) }
    pub fn graph_next(&mut self) { if let Some(ref data) = self.graph_data { if !data.nodes.is_empty() { self.graph_selected = (self.graph_selected + 1) % data.nodes.len(); } } }
    pub fn graph_prev(&mut self) { if let Some(ref data) = self.graph_data { if !data.nodes.is_empty() { self.graph_selected = self.graph_selected.checked_sub(1).unwrap_or(data.nodes.len() - 1); } } }
    pub fn load_entity_graph(&mut self) -> crate::error::Result<()> {
        let frequent = self.db.entities().frequent_entities(&self.project, 30)?;
        self.entity_graph_data = Some(EntityGraphData { frequent_entities: frequent, selected_memories: Vec::new(), selected: 0 });
        Ok(())
    }
    pub fn load_temporal(&mut self) -> crate::error::Result<()> {
        let memories = self.db.memories().list(&self.project, None, None, None, 500, 0)?;
        self.temporal_data = Some(TemporalData { memories, reference_time: Utc::now(), display_mode: 0 });
        Ok(())
    }
    pub fn temporal_cycle_mode(&mut self) { if let Some(ref mut data) = self.temporal_data { data.display_mode = (data.display_mode + 1) % 3; } }
}
