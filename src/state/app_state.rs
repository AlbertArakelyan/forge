use super::{
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
}
