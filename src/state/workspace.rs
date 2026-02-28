use serde::{Deserialize, Serialize};

use crate::state::app_state::{ActiveTab, RequestStatus, ResponseTab};
use crate::state::collection::Collection;
use crate::state::environment::Environment;
use crate::state::request_state::RequestState;
use crate::state::response_state::ResponseState;

/// Persisted workspace metadata (saved to `workspace.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceFile {
    pub name: String,
    pub active_environment_idx: Option<usize>,
}

/// A single open request tab (in-memory only).
#[derive(Debug, Clone)]
pub struct RequestTab {
    pub request: RequestState,
    pub response: Option<ResponseState>,
    pub active_tab: ActiveTab,
    pub response_tab: ResponseTab,
    pub is_dirty: bool,
    pub collection_id: Option<String>,
    pub request_status: RequestStatus,
}

impl Default for RequestTab {
    fn default() -> Self {
        Self {
            request: RequestState::default(),
            response: None,
            active_tab: ActiveTab::default(),
            response_tab: ResponseTab::default(),
            is_dirty: false,
            collection_id: None,
            request_status: RequestStatus::default(),
        }
    }
}

/// Full in-memory workspace state.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    pub name: String,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment_idx: Option<usize>,
    pub open_tabs: Vec<RequestTab>,
    pub active_tab_idx: usize,
}
