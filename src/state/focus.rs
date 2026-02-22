#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Focus {
    Sidebar,
    #[default]
    UrlBar,
    TabBar,
    Editor,
    ResponseViewer,
}

impl Focus {
    /// Cycle order: Sidebar → UrlBar → Editor → ResponseViewer → Sidebar
    pub fn next(&self) -> Focus {
        match self {
            Focus::Sidebar => Focus::UrlBar,
            Focus::UrlBar => Focus::Editor,
            Focus::TabBar => Focus::Editor,
            Focus::Editor => Focus::ResponseViewer,
            Focus::ResponseViewer => Focus::Sidebar,
        }
    }

    pub fn prev(&self) -> Focus {
        match self {
            Focus::Sidebar => Focus::ResponseViewer,
            Focus::UrlBar => Focus::Sidebar,
            Focus::TabBar => Focus::UrlBar,
            Focus::Editor => Focus::UrlBar,
            Focus::ResponseViewer => Focus::Editor,
        }
    }
}
