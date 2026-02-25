use super::{
    environment::Environment,
    focus::Focus,
    mode::Mode,
    request_state::RequestState,
    response_state::ResponseState,
};

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

/// Which overlay popup (if any) is currently visible.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActivePopup {
    #[default]
    None,
    EnvSwitcher,
    EnvEditor,
}

/// State for the environment switcher popup.
#[derive(Debug, Clone)]
pub struct EnvSwitcherState {
    /// Currently highlighted row in the filtered list.
    pub selected: usize,
    pub search: String,
    pub search_cursor: usize,
}

impl Default for EnvSwitcherState {
    fn default() -> Self {
        Self { selected: 0, search: String::new(), search_cursor: 0 }
    }
}

/// State for the environment editor popup.
#[derive(Debug, Clone)]
pub struct EnvEditorState {
    /// Index into `AppState.environments` being edited.
    pub env_idx: usize,
    /// Selected row (variable index).
    pub row: usize,
    /// Selected column: 0=key, 1=value, 2=description, 3=type.
    pub col: u8,
    /// Byte cursor within the currently edited cell.
    pub cursor: usize,
    pub show_secret: bool,
    /// Whether we are currently editing the cell (Insert mode for the editor).
    pub editing: bool,
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
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub mode: Mode,
    pub focus: Focus,
    pub request: RequestState,
    pub response: Option<ResponseState>,
    pub active_tab: ActiveTab,
    pub response_tab: ResponseTab,
    pub request_status: RequestStatus,
    pub sidebar_visible: bool,
    pub should_quit: bool,
    /// Set to `true` whenever visible state changes. The render loop skips
    /// `terminal.draw()` when `false`, avoiding redundant work on idle ticks.
    pub dirty: bool,

    // Round 2: environments
    pub environments: Vec<Environment>,
    pub active_env_idx: Option<usize>,
    pub active_popup: ActivePopup,
    pub env_editor: EnvEditorState,
    pub env_switcher: EnvSwitcherState,
}
