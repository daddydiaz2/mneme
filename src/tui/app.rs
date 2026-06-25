use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::memory::{Memory, MemoryStats, ProjectSummary, SearchQuery, Session};
use std::sync::Arc;

pub struct App {
    pub db: Arc<Database>,
    pub screen: Screen,
    // Project list
    pub projects: Vec<ProjectSummary>,
    pub proj_sel: usize,
    // Memory list
    pub project: String,
    pub memories: Vec<Memory>,
    pub mem_sel: usize,
    pub mem_scroll: usize,
    pub stats: Option<MemoryStats>,
    // Detail
    pub detail: Option<Memory>,
    pub detail_scroll: usize,
    // Sessions
    pub sessions: Vec<Session>,
    // Search
    pub search: String,
    pub searching: bool,
    // UI
    pub quit: bool,
    pub msg: String,
    pub total_mems: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Projects,
    Memories,
    Detail,
    Sessions,
    Help,
}

impl App {
    pub fn new(db: Arc<Database>, _settings: Arc<Settings>) -> Self {
        let projects = db.memories().list_projects().unwrap_or_default();
        let total: u32 = projects.iter().map(|p| p.memory_count).sum();
        Self {
            db,
            screen: Screen::Projects,
            projects,
            proj_sel: 0,
            project: String::new(),
            memories: vec![],
            mem_sel: 0,
            mem_scroll: 0,
            stats: None,
            detail: None,
            detail_scroll: 0,
            sessions: vec![],
            search: String::new(),
            searching: false,
            quit: false,
            msg: String::new(),
            total_mems: total,
        }
    }
    pub fn load_projects(&mut self) {
        self.projects = self.db.memories().list_projects().unwrap_or_default();
        self.total_mems = self.projects.iter().map(|p| p.memory_count).sum();
    }
    pub fn load_memories(&mut self) {
        self.memories = self
            .db
            .memories()
            .list(&self.project, None, None, None, 500, 0)
            .unwrap_or_default();
        self.stats = self.db.memories().stats(&self.project).ok();
        self.mem_sel = 0;
        self.mem_scroll = 0;
    }
    pub fn load_sessions(&mut self) {
        self.sessions = self
            .db
            .sessions()
            .list(&self.project, 100)
            .unwrap_or_default();
    }
    pub fn search_all(&mut self) {
        if self.search.is_empty() {
            self.load_projects();
            return;
        }
        let q = SearchQuery {
            text: self.search.clone(),
            project: None,
            scope: None,
            memory_type: None,
            importance: None,
            tags: vec![],
            limit: 200,
            include_snippet: true,
            all_projects: true,
        };
        self.memories = self
            .db
            .memories()
            .search(&q, &crate::store::search::SearchWeights::default(), None)
            .unwrap_or_default()
            .into_iter()
            .map(|r| r.memory)
            .collect();
        self.screen = Screen::Memories;
        self.project = format!("search: {}", self.search);
    }
    pub fn load(&mut self) {
        self.load_projects();
    }
    pub fn delete_sel(&mut self) {
        if let Some(m) = self.detail.clone() {
            let _ = self.db.memories().delete(m.id, false);
            self.msg = format!("Deleted: {}", m.title);
            self.detail = None;
            self.screen = Screen::Memories;
            self.load_memories();
        }
    }
}
