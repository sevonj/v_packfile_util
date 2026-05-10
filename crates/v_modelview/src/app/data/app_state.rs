use std::collections::VecDeque;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    #[default]
    View,
    Log,
}

impl std::fmt::Display for AppTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppTab::View => write!(f, "View"),
            AppTab::Log => write!(f, "Log"),
        }
    }
}

#[derive(Default)]
pub struct AppState {
    pub log: VecDeque<String>,
    pub tab: AppTab,
}
