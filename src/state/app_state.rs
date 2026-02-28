use std::collections::HashSet;

use super::{
    focus::Focus,
    mode::Mode,
    workspace::{RequestTab, WorkspaceState},
};

// ─── Request/Response tab enums ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Headers,
    Body,
    Auth,
    Params,
    Scripts,
}

impl ActiveTab {
    pub fn next(&self) -> ActiveTab {
        match self {
            ActiveTab::Headers => ActiveTab::Body,
            ActiveTab::Body => ActiveTab::Auth,
            ActiveTab::Auth => ActiveTab::Params,
            ActiveTab::Params => ActiveTab::Scripts,
            ActiveTab::Scripts => ActiveTab::Headers,
        }
    }

    pub fn prev(&self) -> ActiveTab {
        match self {
            ActiveTab::Headers => ActiveTab::Scripts,
            ActiveTab::Body => ActiveTab::Headers,
            ActiveTab::Auth => ActiveTab::Body,
            ActiveTab::Params => ActiveTab::Auth,
            ActiveTab::Scripts => ActiveTab::Params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
    Cookies,
    Timing,
}

#[derive(Debug, Clone, Default)]
pub enum RequestStatus {
    #[default]
    Idle,
    Loading { spinner_tick: u8 },
    Error(String),
}

// ─── Popup discriminant ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActivePopup {
    #[default]
    None,
    EnvSwitcher,
    EnvEditor,
    WorkspaceSwitcher,
    CollectionNaming,
    ConfirmDelete,
}

// ─── Env popup state (unchanged from Round 2) ─────────────────────────────────

#[derive(Debug, Clone)]
pub struct EnvSwitcherState {
    pub selected: usize,
    pub search: String,
    pub search_cursor: usize,
    pub naming: bool,
    pub new_name: String,
    pub new_name_cursor: usize,
}

impl Default for EnvSwitcherState {
    fn default() -> Self {
        Self {
            selected: 0,
            search: String::new(),
            search_cursor: 0,
            naming: false,
            new_name: String::new(),
            new_name_cursor: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnvEditorState {
    pub env_idx: usize,
    pub row: usize,
    pub col: u8,
    pub cursor: usize,
    pub show_secret: bool,
    pub editing: bool,
    pub editing_name: bool,
    pub name_cursor: usize,
}

impl Default for EnvEditorState {
    fn default() -> Self {
        Self {
            env_idx: 0,
            row: 0,
            col: 0,
            cursor: 0,
            show_secret: false,
            editing: false,
            editing_name: false,
            name_cursor: 0,
        }
    }
}

// ─── Round 3: Sidebar state ───────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SidebarState {
    pub cursor: usize,
    pub collapsed_ids: HashSet<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub scroll_offset: usize,
}

// ─── Round 3: Workspace switcher popup ───────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct WorkspaceSwitcherState {
    pub selected: usize,
    pub search: String,
    pub search_cursor: usize,
    pub naming: bool,
    pub new_name: String,
    pub new_name_cursor: usize,
}

// ─── Round 3: Collection/folder/request naming popup ─────────────────────────

#[derive(Debug, Clone)]
pub enum NamingTarget {
    NewCollection,
    NewFolder { collection_id: String },
    NewRequest { collection_id: String, folder_id: Option<String> },
    Rename { id: String, old_name: String },
}

impl Default for NamingTarget {
    fn default() -> Self {
        Self::NewCollection
    }
}

#[derive(Debug, Clone)]
pub struct NamingState {
    pub target: NamingTarget,
    pub input: String,
    pub cursor: usize,
    pub method: String,
}

impl Default for NamingState {
    fn default() -> Self {
        Self {
            target: NamingTarget::default(),
            input: String::new(),
            cursor: 0,
            method: "GET".to_string(),
        }
    }
}

// ─── Round 3: Delete confirmation popup ──────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ConfirmDeleteState {
    pub message: String,
    pub target_id: String,
}

// ─── AppState ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub mode: Mode,
    pub focus: Focus,
    pub sidebar_visible: bool,
    pub should_quit: bool,
    /// Set to `true` whenever visible state changes. The render loop skips
    /// `terminal.draw()` when `false`, avoiding redundant work on idle ticks.
    pub dirty: bool,

    pub active_popup: ActivePopup,
    pub env_editor: EnvEditorState,
    pub env_switcher: EnvSwitcherState,

    // Round 3
    pub workspace: WorkspaceState,
    pub all_workspaces: Vec<String>,
    pub sidebar: SidebarState,
    pub naming: NamingState,
    pub confirm_delete: ConfirmDeleteState,
    pub ws_switcher: WorkspaceSwitcherState,
}

impl AppState {
    /// Returns a reference to the currently active request tab, if any.
    pub fn active_tab(&self) -> Option<&RequestTab> {
        self.workspace.open_tabs.get(self.workspace.active_tab_idx)
    }

    /// Returns a mutable reference to the currently active request tab, if any.
    pub fn active_tab_mut(&mut self) -> Option<&mut RequestTab> {
        self.workspace.open_tabs.get_mut(self.workspace.active_tab_idx)
    }
}
