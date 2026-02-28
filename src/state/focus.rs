#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Focus {
    Sidebar,
    RequestTabs,
    #[default]
    UrlBar,
    TabBar,
    Editor,
    ResponseViewer,
}

impl Focus {
    /// Cycle order: Sidebar → RequestTabs → UrlBar → TabBar → Editor → ResponseViewer → Sidebar
    pub fn next(&self) -> Focus {
        match self {
            Focus::Sidebar => Focus::RequestTabs,
            Focus::RequestTabs => Focus::UrlBar,
            Focus::UrlBar => Focus::TabBar,
            Focus::TabBar => Focus::Editor,
            Focus::Editor => Focus::ResponseViewer,
            Focus::ResponseViewer => Focus::Sidebar,
        }
    }

    pub fn prev(&self) -> Focus {
        match self {
            Focus::Sidebar => Focus::ResponseViewer,
            Focus::RequestTabs => Focus::Sidebar,
            Focus::UrlBar => Focus::RequestTabs,
            Focus::TabBar => Focus::UrlBar,
            Focus::Editor => Focus::TabBar,
            Focus::ResponseViewer => Focus::Editor,
        }
    }
}
