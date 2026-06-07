use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::entities::{EntitySearchResult, EntityType};
use crate::store::memory::{GraphData, Memory, SearchQuery};

/// Datos para la vista de grafo de entidades.
pub struct EntityGraphData {
    /// Entidades frecuentes (top N).
    pub frequent_entities: Vec<(String, EntityType, u32)>,
    /// Memorias seleccionadas actualmente.
    pub selected_memories: Vec<EntitySearchResult>,
    /// Índice del nodo seleccionado.
    pub selected: usize,
}

/// Datos para la vista temporal de memorias.
pub struct TemporalData {
    /// Memorias con sus ventanas de validez.
    pub memories: Vec<Memory>,
    /// Timestamp de referencia para la consulta.
    pub reference_time: DateTime<Utc>,
    /// Modo de visualización: 0 = "all", 1 = "valid at", 2 = "expired".
    pub display_mode: u8,
}

/// Modos de operación de la aplicación TUI.
pub enum AppMode {
    /// Navegación normal.
    Normal,
    /// Escritura en la barra de búsqueda.
    Searching,
    /// Confirmación de una acción destructiva.
    Confirming {
        action: String,
        memory_id: uuid::Uuid,
    },
    /// Overlay de ayuda visible.
    Help,
    /// Vista de grafo de relaciones.
    Graph,
    /// Vista de grafo de entidades.
    EntityGraph,
    /// Vista temporal de memorias.
    Temporal,
}

/// Estado global de la aplicación TUI.
pub struct App {
    /// Modo actual.
    pub mode: AppMode,
    /// Proyecto activo.
    pub project: String,
    /// Memorias cargadas.
    pub memories: Vec<Memory>,
    /// Índice de la memoria seleccionada.
    pub selected: usize,
    /// Offset de scroll en la lista.
    pub scroll_offset: usize,
    /// Texto de búsqueda actual.
    pub search_query: String,
    /// Mensaje transitorio para la status bar.
    pub status_message: Option<String>,
    /// Flag para salir del loop.
    pub should_quit: bool,
    /// Datos del grafo de relaciones (cargados bajo demanda).
    pub graph_data: Option<GraphData>,
    /// Índice del nodo seleccionado en la vista de grafo.
    pub graph_selected: usize,
    /// Datos de entidades para entity graph.
    pub entity_graph_data: Option<EntityGraphData>,
    /// Estado de la vista temporal.
    pub temporal_data: Option<TemporalData>,
    db: Arc<Database>,
    #[allow(dead_code)]
    settings: Arc<Settings>,
}

impl App {
    /// Crea una nueva instancia de la app y carga el proyecto inferido.
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
            db,
            settings,
        })
    }

    /// Carga las memorias del proyecto actual aplicando búsqueda si hay query activa.
    pub fn load_memories(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        let memories = if self.search_query.is_empty() {
            store.list(&self.project, None, None, None, 200, 0)?
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
        Ok(())
    }

    /// Carga el grafo de relaciones para el proyecto actual.
    pub fn load_graph(&mut self) -> crate::error::Result<()> {
        let store = self.db.memories();
        let data = store.get_graph(&self.project)?;
        self.graph_selected = 0;
        self.graph_data = Some(data);
        Ok(())
    }

    /// Alterna la vista de grafo. Si se activa, carga los datos primero.
    pub fn toggle_graph(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::Graph => {
                self.mode = AppMode::Normal;
            }
            _ => {
                self.load_graph()?;
                self.mode = AppMode::Graph;
            }
        }
        Ok(())
    }

    /// Avanza la selección al siguiente nodo del grafo.
    pub fn graph_next(&mut self) {
        if let Some(ref data) = self.graph_data {
            if !data.nodes.is_empty() {
                self.graph_selected = (self.graph_selected + 1) % data.nodes.len();
            }
        }
    }

    /// Retrocede la selección al nodo anterior del grafo.
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

    /// Carga datos de entidades frecuentes y prepara entity graph.
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

    /// Activa la vista de grafo de entidades.
    pub fn toggle_entity_graph(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::EntityGraph => {
                self.mode = AppMode::Normal;
            }
            _ => {
                self.load_entity_graph()?;
                self.mode = AppMode::EntityGraph;
            }
        }
        Ok(())
    }

    /// Carga datos temporales (memorias + ventanas de validez).
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

    /// Activa la vista temporal.
    pub fn toggle_temporal(&mut self) -> crate::error::Result<()> {
        match self.mode {
            AppMode::Temporal => {
                self.mode = AppMode::Normal;
            }
            _ => {
                self.load_temporal()?;
                self.mode = AppMode::Temporal;
            }
        }
        Ok(())
    }

    /// Cicla el modo de display temporal: 0=All, 1=ValidAt, 2=Expired.
    pub fn temporal_cycle_mode(&mut self) {
        if let Some(ref mut data) = self.temporal_data {
            data.display_mode = (data.display_mode + 1) % 3;
        }
    }

    /// Mueve la selección hacia abajo.
    pub fn select_next(&mut self) {
        if !self.memories.is_empty() {
            self.selected = (self.selected + 1).min(self.memories.len() - 1);
            self.ensure_selected_visible(20);
        }
    }

    /// Mueve la selección hacia arriba.
    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.ensure_selected_visible(20);
    }

    /// Va al primer elemento.
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Va al último elemento.
    pub fn select_last(&mut self) {
        if !self.memories.is_empty() {
            self.selected = self.memories.len() - 1;
            self.ensure_selected_visible(20);
        }
    }

    /// Avanza una página.
    pub fn page_down(&mut self) {
        let page_size = 20usize;
        if !self.memories.is_empty() {
            self.selected = (self.selected + page_size).min(self.memories.len() - 1);
            self.ensure_selected_visible(page_size);
        }
    }

    /// Retrocede una página.
    pub fn page_up(&mut self) {
        let page_size = 20usize;
        self.selected = self.selected.saturating_sub(page_size);
        self.ensure_selected_visible(page_size);
    }

    /// Retorna la memoria seleccionada si existe.
    pub fn selected_memory(&self) -> Option<&Memory> {
        self.memories.get(self.selected)
    }

    /// Activa el modo de búsqueda.
    pub fn start_search(&mut self) {
        self.mode = AppMode::Searching;
        self.search_query.clear();
    }

    /// Confirma la búsqueda y recarga las memorias.
    pub fn confirm_search(&mut self) -> crate::error::Result<()> {
        self.mode = AppMode::Normal;
        self.load_memories()?;
        Ok(())
    }

    /// Cancela la búsqueda y vuelve a mostrar todas las memorias.
    pub fn cancel_search(&mut self) {
        self.mode = AppMode::Normal;
        self.search_query.clear();
        let _ = self.load_memories();
    }

    /// Agrega un carácter al query de búsqueda.
    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    /// Elimina el último carácter del query de búsqueda.
    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
    }

    /// Inicia la confirmación para eliminar la memoria seleccionada.
    pub fn delete_selected(&mut self) -> crate::error::Result<()> {
        if let Some(memory) = self.selected_memory() {
            let id = memory.id;
            self.mode = AppMode::Confirming {
                action: "delete".to_string(),
                memory_id: id,
            };
        }
        Ok(())
    }

    /// Confirma la acción pendiente.
    pub fn confirm_action(&mut self) -> crate::error::Result<()> {
        if let AppMode::Confirming { action, memory_id } = &self.mode {
            if action == "delete" {
                let store = self.db.memories();
                store.delete(*memory_id, false)?;
                self.status_message = Some("Memoria eliminada".to_string());
                self.load_memories()?;
            }
            self.mode = AppMode::Normal;
        }
        Ok(())
    }

    /// Cancela la acción pendiente.
    pub fn cancel_confirm(&mut self) {
        self.mode = AppMode::Normal;
    }

    /// Alterna la visibilidad del overlay de ayuda.
    pub fn toggle_help(&mut self) {
        self.mode = match self.mode {
            AppMode::Help => AppMode::Normal,
            _ => AppMode::Help,
        };
    }

    /// Marca la app para salir del loop.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    fn ensure_selected_visible(&mut self, visible_height: usize) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected.saturating_sub(visible_height - 1);
        }
    }
}
