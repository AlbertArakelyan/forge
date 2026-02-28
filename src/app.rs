use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::event::Event;
use crate::http::{client::build_client, executor::execute};
use crate::state::app_state::{
    ActivePopup, ActiveTab, AppState, ConfirmDeleteState, NamingState, NamingTarget,
    RequestStatus, WorkspaceSwitcherState,
};
use crate::state::collection::{Collection, CollectionItem, CollectionRequest, Folder};
use crate::state::environment::{EnvVariable, Environment, VarType};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::request_state::KeyValuePair;
use crate::state::response_state::{ResponseBody, ResponseState};
use crate::state::workspace::RequestTab;
use crate::env::resolver::resolver_from_state;
use crate::storage::environment as env_storage;
use crate::storage::collection as col_storage;
use crate::storage::workspace as ws_storage;
use crate::ui::highlight::{detect_lang, highlight_text};
use crate::ui::sidebar::flatten_tree;

pub struct App {
    pub state: AppState,
    client: reqwest::Client,
    tx: UnboundedSender<Event>,
    cancel: Option<CancellationToken>,
}

impl App {
    pub fn new(tx: UnboundedSender<Event>) -> Self {
        let mut ws = ws_storage::load_workspace_full("default");
        let all_workspaces = ws_storage::list_workspaces();

        if ws.open_tabs.is_empty() {
            ws.open_tabs.push(RequestTab::default());
        }

        let active_env_idx = if ws.environments.is_empty() {
            None
        } else {
            ws.active_environment_idx.or(Some(0))
        };
        ws.active_environment_idx = active_env_idx;

        Self {
            state: AppState {
                sidebar_visible: true,
                dirty: true,
                workspace: ws,
                all_workspaces,
                ..Default::default()
            },
            client: build_client(),
            tx,
            cancel: None,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                self.state.dirty = true;

                // Ctrl+R fires globally regardless of mode or focus
                if key.code == KeyCode::Char('r')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    self.send_request();
                    return;
                }

                // Ctrl+E: toggle environment switcher popup
                if key.code == KeyCode::Char('e')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    match self.state.active_popup {
                        ActivePopup::None => {
                            self.state.active_popup = ActivePopup::EnvSwitcher;
                            self.state.env_switcher.selected = 0;
                            self.state.env_switcher.search.clear();
                            self.state.env_switcher.search_cursor = 0;
                        }
                        ActivePopup::EnvSwitcher | ActivePopup::EnvEditor => {
                            self.state.active_popup = ActivePopup::None;
                        }
                        _ => {
                            self.state.active_popup = ActivePopup::None;
                        }
                    }
                    return;
                }

                // Ctrl+W: workspace switcher
                if key.code == KeyCode::Char('w')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    match self.state.active_popup {
                        ActivePopup::None => {
                            self.state.active_popup = ActivePopup::WorkspaceSwitcher;
                            self.state.ws_switcher = WorkspaceSwitcherState::default();
                        }
                        ActivePopup::WorkspaceSwitcher => {
                            self.state.active_popup = ActivePopup::None;
                        }
                        _ => {
                            self.state.active_popup = ActivePopup::None;
                        }
                    }
                    return;
                }

                // If a popup is open, route all keys to it
                if self.state.active_popup != ActivePopup::None {
                    self.handle_popup_key(key);
                    return;
                }
                match self.state.mode {
                    Mode::Normal => self.handle_normal_key(key),
                    Mode::Insert => self.handle_insert_key(key),
                    Mode::Command | Mode::Visual => {}
                }
            }
            Event::Key(_) => {}
            Event::Response(result) => {
                self.state.dirty = true;
                self.handle_response(result);
            }
            // Tick: only dirty when the spinner is visible; otherwise a no-op.
            Event::Tick => self.handle_tick(),
            Event::Mouse(mouse) => {
                self.state.dirty = true;
                self.handle_mouse(mouse);
            }
            // Terminal resize always requires a full redraw.
            Event::Resize(_, _) => self.state.dirty = true,
        }
    }

    // -------------------------------------------------------------------------
    // Popup key handling
    // -------------------------------------------------------------------------

    fn handle_popup_key(&mut self, key: KeyEvent) {
        match self.state.active_popup.clone() {
            ActivePopup::EnvSwitcher => self.handle_env_switcher_key(key),
            ActivePopup::EnvEditor => self.handle_env_editor_key(key),
            ActivePopup::WorkspaceSwitcher => self.handle_workspace_switcher_key(key),
            ActivePopup::CollectionNaming => self.handle_naming_key(key),
            ActivePopup::ConfirmDelete => self.handle_confirm_delete_key(key),
            ActivePopup::None => {}
        }
    }

    // ─── Env switcher ─────────────────────────────────────────────────────────

    fn handle_env_switcher_key(&mut self, key: KeyEvent) {
        if self.state.env_switcher.naming {
            self.handle_env_switcher_naming_key(key);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Enter => {
                // Activate the selected environment
                let filter = self.state.env_switcher.search.to_lowercase();
                let selected = self.state.env_switcher.selected;
                let idx = self
                    .state
                    .workspace
                    .environments
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                    .nth(selected)
                    .map(|(i, _)| i);
                if let Some(i) = idx {
                    self.state.workspace.active_environment_idx = Some(i);
                }
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::ALT) => {
                // Open editor for selected environment
                let filter = self.state.env_switcher.search.to_lowercase();
                let selected = self.state.env_switcher.selected;
                let idx = self
                    .state
                    .workspace
                    .environments
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                    .nth(selected)
                    .map(|(i, _)| i);
                if let Some(i) = idx {
                    self.state.env_editor.env_idx = i;
                    self.state.env_editor.row = 0;
                    self.state.env_editor.col = 0;
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = false;
                    self.state.env_editor.show_secret = false;
                    self.state.active_popup = ActivePopup::EnvEditor;
                } else if self.state.workspace.environments.is_empty() {
                    let new_env = Environment::default();
                    self.state.workspace.environments.push(new_env);
                    let i = self.state.workspace.environments.len() - 1;
                    self.state.env_editor.env_idx = i;
                    self.state.env_editor.row = 0;
                    self.state.env_editor.col = 0;
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = false;
                    self.state.env_editor.show_secret = false;
                    self.state.active_popup = ActivePopup::EnvEditor;
                }
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.state.env_switcher.naming = true;
                self.state.env_switcher.new_name = String::new();
                self.state.env_switcher.new_name_cursor = 0;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::ALT) => {
                let filter = self.state.env_switcher.search.to_lowercase();
                let selected = self.state.env_switcher.selected;
                let idx = self
                    .state
                    .workspace
                    .environments
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                    .nth(selected)
                    .map(|(i, _)| i);
                if let Some(i) = idx {
                    let env_id = self.state.workspace.environments[i].id.clone();
                    let ws_name = self.state.workspace.name.clone();
                    let _ = env_storage::delete_ws(&ws_name, &env_id);
                    self.state.workspace.environments.remove(i);
                    match self.state.workspace.active_environment_idx {
                        Some(ai) if ai == i => self.state.workspace.active_environment_idx = None,
                        Some(ai) if ai > i => {
                            self.state.workspace.active_environment_idx = Some(ai - 1)
                        }
                        _ => {}
                    }
                    let count = self.filtered_env_count();
                    self.state.env_switcher.selected =
                        self.state.env_switcher.selected.min(count.saturating_sub(1));
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.filtered_env_count();
                if count > 0 {
                    self.state.env_switcher.selected =
                        (self.state.env_switcher.selected + 1).min(count - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.env_switcher.selected =
                    self.state.env_switcher.selected.saturating_sub(1);
            }
            KeyCode::Backspace => {
                let cursor = self.state.env_switcher.search_cursor;
                if cursor > 0 {
                    let search = self.state.env_switcher.search.clone();
                    let prev = Self::prev_char_boundary_of(&search, cursor);
                    self.state.env_switcher.search.drain(prev..cursor);
                    self.state.env_switcher.search_cursor = prev;
                    self.state.env_switcher.selected = 0;
                }
            }
            KeyCode::Char(c) => {
                let cursor = self.state.env_switcher.search_cursor;
                self.state.env_switcher.search.insert(cursor, c);
                self.state.env_switcher.search_cursor += c.len_utf8();
                self.state.env_switcher.selected = 0;
            }
            _ => {}
        }
    }

    fn filtered_env_count(&self) -> usize {
        let filter = self.state.env_switcher.search.to_lowercase();
        self.state
            .workspace
            .environments
            .iter()
            .filter(|e| filter.is_empty() || e.name.to_lowercase().contains(&filter))
            .count()
    }

    fn handle_env_switcher_naming_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.env_switcher.naming = false;
                self.state.env_switcher.new_name = String::new();
            }
            KeyCode::Enter => {
                let name = if self.state.env_switcher.new_name.trim().is_empty() {
                    "New Environment".to_string()
                } else {
                    self.state.env_switcher.new_name.trim().to_string()
                };
                let mut new_env = Environment::default();
                new_env.name = name;
                let ws_name = self.state.workspace.name.clone();
                let _ = env_storage::save_ws(&ws_name, &new_env);
                self.state.workspace.environments.push(new_env);
                let i = self.state.workspace.environments.len() - 1;
                self.state.env_switcher.selected = i;
                self.state.workspace.active_environment_idx = Some(i);
                self.state.env_switcher.naming = false;
                self.state.env_switcher.new_name = String::new();
                self.state.env_switcher.new_name_cursor = 0;
            }
            KeyCode::Char(c) => {
                let cursor = self.state.env_switcher.new_name_cursor;
                self.state.env_switcher.new_name.insert(cursor, c);
                self.state.env_switcher.new_name_cursor = cursor + c.len_utf8();
            }
            KeyCode::Backspace => {
                let cursor = self.state.env_switcher.new_name_cursor;
                if cursor > 0 {
                    let s = self.state.env_switcher.new_name.clone();
                    let prev = Self::prev_char_boundary_of(&s, cursor);
                    self.state.env_switcher.new_name.drain(prev..cursor);
                    self.state.env_switcher.new_name_cursor = prev;
                }
            }
            KeyCode::Delete => {
                let cursor = self.state.env_switcher.new_name_cursor;
                let len = self.state.env_switcher.new_name.len();
                if cursor < len {
                    let s = self.state.env_switcher.new_name.clone();
                    let next = Self::next_char_boundary_of(&s, cursor);
                    self.state.env_switcher.new_name.drain(cursor..next);
                }
            }
            KeyCode::Left => {
                let cursor = self.state.env_switcher.new_name_cursor;
                let s = self.state.env_switcher.new_name.clone();
                self.state.env_switcher.new_name_cursor = Self::prev_char_boundary_of(&s, cursor);
            }
            KeyCode::Right => {
                let cursor = self.state.env_switcher.new_name_cursor;
                let s = self.state.env_switcher.new_name.clone();
                self.state.env_switcher.new_name_cursor = Self::next_char_boundary_of(&s, cursor);
            }
            KeyCode::Home => {
                self.state.env_switcher.new_name_cursor = 0;
            }
            KeyCode::End => {
                self.state.env_switcher.new_name_cursor = self.state.env_switcher.new_name.len();
            }
            _ => {}
        }
    }

    // ─── Env editor ───────────────────────────────────────────────────────────

    fn handle_env_editor_key(&mut self, key: KeyEvent) {
        if self.state.env_editor.editing_name {
            self.handle_env_name_edit_key(key);
            return;
        }
        if self.state.env_editor.editing {
            self.handle_env_editor_insert_key(key);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.save_current_env();
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                let col = self.state.env_editor.col;
                if col < 3 {
                    self.state.env_editor.editing = true;
                    let cursor = self.current_editor_field_len();
                    self.state.env_editor.cursor = cursor;
                }
            }
            KeyCode::Char('a') => {
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                    env.variables.push(EnvVariable::default());
                    self.state.env_editor.row = env.variables.len() - 1;
                    self.state.env_editor.col = 0;
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = true;
                }
            }
            KeyCode::Char('d') => {
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                    let row = self.state.env_editor.row;
                    if row < env.variables.len() {
                        env.variables.remove(row);
                        let new_len = env.variables.len();
                        self.state.env_editor.row = if new_len > 0 {
                            row.min(new_len - 1)
                        } else {
                            0
                        };
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let idx = self.state.env_editor.env_idx;
                let len = self
                    .state
                    .workspace
                    .environments
                    .get(idx)
                    .map(|e| e.variables.len())
                    .unwrap_or(0);
                if len > 0 {
                    self.state.env_editor.row = (self.state.env_editor.row + 1).min(len - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.env_editor.row = self.state.env_editor.row.saturating_sub(1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.state.env_editor.col = self.state.env_editor.col.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.state.env_editor.col = (self.state.env_editor.col + 1).min(3);
            }
            KeyCode::Char('r') => {
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.workspace.environments.get(idx) {
                    self.state.env_editor.name_cursor = env.name.len();
                    self.state.env_editor.editing_name = true;
                }
            }
            KeyCode::Char(' ') => {
                let idx = self.state.env_editor.env_idx;
                let row = self.state.env_editor.row;
                let col = self.state.env_editor.col;
                if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                    if let Some(var) = env.variables.get_mut(row) {
                        match col {
                            0 => var.enabled = !var.enabled,
                            3 => {
                                var.var_type = if var.var_type == VarType::Secret {
                                    VarType::Text
                                } else {
                                    VarType::Secret
                                };
                            }
                            _ => {
                                if col == 1 {
                                    self.state.env_editor.show_secret =
                                        !self.state.env_editor.show_secret;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_env_editor_insert_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.env_editor.editing = false;
            }
            KeyCode::Tab => {
                self.state.env_editor.editing = false;
                let col = self.state.env_editor.col;
                if col < 2 {
                    self.state.env_editor.col = col + 1;
                    self.state.env_editor.cursor = self.current_editor_field_len();
                    self.state.env_editor.editing = true;
                } else {
                    self.state.env_editor.col = 0;
                    let idx = self.state.env_editor.env_idx;
                    let len = self
                        .state
                        .workspace
                        .environments
                        .get(idx)
                        .map(|e| e.variables.len())
                        .unwrap_or(0);
                    let next_row = self.state.env_editor.row + 1;
                    if next_row >= len {
                        if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                            env.variables.push(EnvVariable::default());
                        }
                    }
                    let new_len = self
                        .state
                        .workspace
                        .environments
                        .get(idx)
                        .map(|e| e.variables.len())
                        .unwrap_or(0);
                    self.state.env_editor.row = next_row.min(new_len.saturating_sub(1));
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = true;
                }
            }
            KeyCode::Char(c) => {
                let cursor = self.state.env_editor.cursor;
                if let Some(field) = self.current_editor_field_mut() {
                    field.insert(cursor, c);
                    self.state.env_editor.cursor = cursor + c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                let cursor = self.state.env_editor.cursor;
                if cursor > 0 {
                    if let Some(field) = self.current_editor_field_mut() {
                        let prev = Self::prev_char_boundary_of(field, cursor);
                        field.drain(prev..cursor);
                        self.state.env_editor.cursor = prev;
                    }
                }
            }
            KeyCode::Delete => {
                let cursor = self.state.env_editor.cursor;
                if let Some(field) = self.current_editor_field_mut() {
                    if cursor < field.len() {
                        let next = Self::next_char_boundary_of(field, cursor);
                        field.drain(cursor..next);
                    }
                }
            }
            KeyCode::Left => {
                let cursor = self.state.env_editor.cursor;
                if let Some(field) = self.current_editor_field_mut() {
                    self.state.env_editor.cursor = Self::prev_char_boundary_of(field, cursor);
                }
            }
            KeyCode::Right => {
                let cursor = self.state.env_editor.cursor;
                if let Some(field) = self.current_editor_field_mut() {
                    self.state.env_editor.cursor = Self::next_char_boundary_of(field, cursor);
                }
            }
            KeyCode::Home => {
                self.state.env_editor.cursor = 0;
            }
            KeyCode::End => {
                self.state.env_editor.cursor = self.current_editor_field_len();
            }
            _ => {}
        }
    }

    fn handle_env_name_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.state.env_editor.editing_name = false;
                self.save_current_env();
            }
            KeyCode::Char(c) => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                    env.name.insert(cursor, c);
                    self.state.env_editor.name_cursor = cursor + c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if cursor > 0 {
                    if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                        let prev = Self::prev_char_boundary_of(&env.name, cursor);
                        env.name.drain(prev..cursor);
                        self.state.env_editor.name_cursor = prev;
                    }
                }
            }
            KeyCode::Delete => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.workspace.environments.get_mut(idx) {
                    if cursor < env.name.len() {
                        let next = Self::next_char_boundary_of(&env.name, cursor);
                        env.name.drain(cursor..next);
                    }
                }
            }
            KeyCode::Left => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.workspace.environments.get(idx) {
                    self.state.env_editor.name_cursor =
                        Self::prev_char_boundary_of(&env.name, cursor);
                }
            }
            KeyCode::Right => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.workspace.environments.get(idx) {
                    self.state.env_editor.name_cursor =
                        Self::next_char_boundary_of(&env.name, cursor);
                }
            }
            KeyCode::Home => {
                self.state.env_editor.name_cursor = 0;
            }
            KeyCode::End => {
                let idx = self.state.env_editor.env_idx;
                self.state.env_editor.name_cursor = self
                    .state
                    .workspace
                    .environments
                    .get(idx)
                    .map(|e| e.name.len())
                    .unwrap_or(0);
            }
            _ => {}
        }
    }

    fn current_editor_field_len(&self) -> usize {
        let idx = self.state.env_editor.env_idx;
        let row = self.state.env_editor.row;
        let col = self.state.env_editor.col;
        self.state
            .workspace
            .environments
            .get(idx)
            .and_then(|e| e.variables.get(row))
            .map(|v| match col {
                0 => v.key.len(),
                1 => v.value.len(),
                2 => v.description.len(),
                _ => 0,
            })
            .unwrap_or(0)
    }

    fn current_editor_field_mut(&mut self) -> Option<&mut String> {
        let idx = self.state.env_editor.env_idx;
        let row = self.state.env_editor.row;
        let col = self.state.env_editor.col;
        let var = self
            .state
            .workspace
            .environments
            .get_mut(idx)?
            .variables
            .get_mut(row)?;
        match col {
            0 => Some(&mut var.key),
            1 => Some(&mut var.value),
            2 => Some(&mut var.description),
            _ => None,
        }
    }

    fn save_current_env(&self) {
        let idx = self.state.env_editor.env_idx;
        let ws_name = &self.state.workspace.name;
        if let Some(env) = self.state.workspace.environments.get(idx) {
            let _ = env_storage::save_ws(ws_name, env);
        }
    }

    // ─── Workspace switcher ───────────────────────────────────────────────────

    fn handle_workspace_switcher_key(&mut self, key: KeyEvent) {
        if self.state.ws_switcher.naming {
            self.handle_ws_naming_key(key);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Enter => {
                let filter = self.state.ws_switcher.search.to_lowercase();
                let selected = self.state.ws_switcher.selected;
                let chosen = self
                    .state
                    .all_workspaces
                    .iter()
                    .filter(|w| filter.is_empty() || w.to_lowercase().contains(&filter))
                    .nth(selected)
                    .cloned();
                if let Some(name) = chosen {
                    if name != self.state.workspace.name {
                        let mut ws = ws_storage::load_workspace_full(&name);
                        if ws.open_tabs.is_empty() {
                            ws.open_tabs.push(RequestTab::default());
                        }
                        self.state.workspace = ws;
                    }
                }
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.state.ws_switcher.naming = true;
                self.state.ws_switcher.new_name = String::new();
                self.state.ws_switcher.new_name_cursor = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let filter = self.state.ws_switcher.search.to_lowercase();
                let count = self
                    .state
                    .all_workspaces
                    .iter()
                    .filter(|w| filter.is_empty() || w.to_lowercase().contains(&filter))
                    .count();
                if count > 0 {
                    self.state.ws_switcher.selected =
                        (self.state.ws_switcher.selected + 1).min(count - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.ws_switcher.selected =
                    self.state.ws_switcher.selected.saturating_sub(1);
            }
            KeyCode::Backspace => {
                let cursor = self.state.ws_switcher.search_cursor;
                if cursor > 0 {
                    let s = self.state.ws_switcher.search.clone();
                    let prev = Self::prev_char_boundary_of(&s, cursor);
                    self.state.ws_switcher.search.drain(prev..cursor);
                    self.state.ws_switcher.search_cursor = prev;
                    self.state.ws_switcher.selected = 0;
                }
            }
            KeyCode::Char(c) => {
                let cursor = self.state.ws_switcher.search_cursor;
                self.state.ws_switcher.search.insert(cursor, c);
                self.state.ws_switcher.search_cursor += c.len_utf8();
                self.state.ws_switcher.selected = 0;
            }
            _ => {}
        }
    }

    fn handle_ws_naming_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.ws_switcher.naming = false;
                self.state.ws_switcher.new_name = String::new();
            }
            KeyCode::Enter => {
                let name = if self.state.ws_switcher.new_name.trim().is_empty() {
                    return;
                } else {
                    self.state.ws_switcher.new_name.trim().to_string()
                };
                let ws_file = crate::state::workspace::WorkspaceFile {
                    name: name.clone(),
                    active_environment_idx: None,
                };
                let _ = ws_storage::save_workspace(&ws_file);
                self.state.all_workspaces = ws_storage::list_workspaces();
                // Switch to new workspace
                let mut ws = ws_storage::load_workspace_full(&name);
                if ws.open_tabs.is_empty() {
                    ws.open_tabs.push(RequestTab::default());
                }
                self.state.workspace = ws;
                self.state.ws_switcher.naming = false;
                self.state.ws_switcher.new_name = String::new();
                self.state.ws_switcher.new_name_cursor = 0;
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char(c) => {
                let cursor = self.state.ws_switcher.new_name_cursor;
                self.state.ws_switcher.new_name.insert(cursor, c);
                self.state.ws_switcher.new_name_cursor = cursor + c.len_utf8();
            }
            KeyCode::Backspace => {
                let cursor = self.state.ws_switcher.new_name_cursor;
                if cursor > 0 {
                    let s = self.state.ws_switcher.new_name.clone();
                    let prev = Self::prev_char_boundary_of(&s, cursor);
                    self.state.ws_switcher.new_name.drain(prev..cursor);
                    self.state.ws_switcher.new_name_cursor = prev;
                }
            }
            KeyCode::Left => {
                let cursor = self.state.ws_switcher.new_name_cursor;
                let s = self.state.ws_switcher.new_name.clone();
                self.state.ws_switcher.new_name_cursor = Self::prev_char_boundary_of(&s, cursor);
            }
            KeyCode::Right => {
                let cursor = self.state.ws_switcher.new_name_cursor;
                let s = self.state.ws_switcher.new_name.clone();
                self.state.ws_switcher.new_name_cursor = Self::next_char_boundary_of(&s, cursor);
            }
            KeyCode::Home => {
                self.state.ws_switcher.new_name_cursor = 0;
            }
            KeyCode::End => {
                self.state.ws_switcher.new_name_cursor = self.state.ws_switcher.new_name.len();
            }
            _ => {}
        }
    }

    // ─── Collection naming popup ──────────────────────────────────────────────

    fn handle_naming_key(&mut self, key: KeyEvent) {
        let is_new_request = matches!(self.state.naming.target, NamingTarget::NewRequest { .. });
        match key.code {
            KeyCode::Esc => {
                self.state.active_popup = ActivePopup::None;
                self.state.naming = NamingState::default();
            }
            KeyCode::Enter => {
                self.confirm_naming();
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Tab if is_new_request => {
                self.state.naming.method = cycle_method_next(&self.state.naming.method);
            }
            KeyCode::Right if is_new_request => {
                self.state.naming.method = cycle_method_next(&self.state.naming.method);
            }
            KeyCode::Left if is_new_request => {
                self.state.naming.method = cycle_method_prev(&self.state.naming.method);
            }
            KeyCode::Char(c) => {
                let cursor = self.state.naming.cursor;
                self.state.naming.input.insert(cursor, c);
                self.state.naming.cursor = cursor + c.len_utf8();
            }
            KeyCode::Backspace => {
                let cursor = self.state.naming.cursor;
                if cursor > 0 {
                    let s = self.state.naming.input.clone();
                    let prev = Self::prev_char_boundary_of(&s, cursor);
                    self.state.naming.input.drain(prev..cursor);
                    self.state.naming.cursor = prev;
                }
            }
            KeyCode::Delete => {
                let cursor = self.state.naming.cursor;
                let len = self.state.naming.input.len();
                if cursor < len {
                    let s = self.state.naming.input.clone();
                    let next = Self::next_char_boundary_of(&s, cursor);
                    self.state.naming.input.drain(cursor..next);
                }
            }
            KeyCode::Left => {
                let cursor = self.state.naming.cursor;
                let s = self.state.naming.input.clone();
                self.state.naming.cursor = Self::prev_char_boundary_of(&s, cursor);
            }
            KeyCode::Right => {
                let cursor = self.state.naming.cursor;
                let s = self.state.naming.input.clone();
                self.state.naming.cursor = Self::next_char_boundary_of(&s, cursor);
            }
            KeyCode::Home => {
                self.state.naming.cursor = 0;
            }
            KeyCode::End => {
                self.state.naming.cursor = self.state.naming.input.len();
            }
            _ => {}
        }
    }

    fn confirm_naming(&mut self) {
        let input = self.state.naming.input.trim().to_string();
        if input.is_empty() {
            self.state.naming = NamingState::default();
            return;
        }

        let ws_name = self.state.workspace.name.clone();
        let target = self.state.naming.target.clone();

        match target {
            NamingTarget::NewCollection => {
                let col = Collection::new(&input);
                let _ = col_storage::save_collection_meta(&ws_name, &col);
                self.state.workspace.collections.push(col);
            }
            NamingTarget::NewFolder { collection_id } => {
                if let Some(col) = self
                    .state
                    .workspace
                    .collections
                    .iter_mut()
                    .find(|c| c.id == collection_id)
                {
                    let folder = Folder::new(&input);
                    col.items.push(CollectionItem::Folder(folder));
                    let _ = col_storage::save_collection_meta(&ws_name, col);
                }
            }
            NamingTarget::NewRequest { collection_id, folder_id } => {
                let mut req = CollectionRequest::new(&input);
                req.method = self.state.naming.method.clone();
                if let Some(col) = self
                    .state
                    .workspace
                    .collections
                    .iter_mut()
                    .find(|c| c.id == collection_id)
                {
                    if let Some(fid) = folder_id {
                        // Find folder anywhere in the collection items
                        add_request_to_folder(&mut col.items, &fid, CollectionItem::Request(req));
                    } else {
                        col.items.push(CollectionItem::Request(req));
                    }
                    let _ = col_storage::save_collection_meta(&ws_name, col);
                }
            }
            NamingTarget::Rename { id, .. } => {
                // Find and rename the item with matching id in collections
                for col in &mut self.state.workspace.collections {
                    if col.id == id {
                        col.name = input.clone();
                        let _ = col_storage::save_collection_meta(&ws_name, col);
                        break;
                    }
                    if rename_item_in_list(&mut col.items, &id, &input) {
                        let _ = col_storage::save_collection_meta(&ws_name, col);
                        break;
                    }
                }
            }
        }

        self.state.naming = NamingState::default();
    }

    // ─── Confirm delete popup ─────────────────────────────────────────────────

    fn handle_confirm_delete_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.execute_delete();
                self.state.active_popup = ActivePopup::None;
                self.state.confirm_delete = ConfirmDeleteState::default();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.state.active_popup = ActivePopup::None;
                self.state.confirm_delete = ConfirmDeleteState::default();
            }
            _ => {}
        }
    }

    fn execute_delete(&mut self) {
        let target_id = self.state.confirm_delete.target_id.clone();
        let ws_name = self.state.workspace.name.clone();

        // Try to delete collection first
        let col_pos = self
            .state
            .workspace
            .collections
            .iter()
            .position(|c| c.id == target_id);
        if let Some(pos) = col_pos {
            let col_name = self.state.workspace.collections[pos].name.clone();
            let _ = col_storage::delete_collection(&ws_name, &col_name);
            self.state.workspace.collections.remove(pos);
            // Clamp cursor
            let len = self.state.workspace.collections.len();
            self.state.sidebar.cursor = self.state.sidebar.cursor.min(len.saturating_sub(1));
            return;
        }

        // Try to delete from within collections
        for col in &mut self.state.workspace.collections {
            if remove_item_from_list(&mut col.items, &target_id) {
                let _ = col_storage::save_collection_meta(&ws_name, col);
                break;
            }
        }
    }

    // ─── Normal key handling ──────────────────────────────────────────────────

    fn handle_normal_key(&mut self, key: KeyEvent) {
        // Alt+1..Alt+9: jump to open tab by index
        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Char(c @ '1'..='9') => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < self.state.workspace.open_tabs.len() {
                        self.sync_active_tab_to_collection();
                        self.state.workspace.active_tab_idx = idx;
                    }
                    return;
                }
                KeyCode::Char('w') => {
                    self.sync_active_tab_to_collection();
                    self.close_active_tab();
                    return;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Tab => self.state.focus = self.state.focus.next(),
            KeyCode::BackTab => self.state.focus = self.state.focus.prev(),
            KeyCode::Char('i') | KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar | Focus::Editor) {
                    self.state.mode = Mode::Insert;
                    if self.state.focus == Focus::Editor {
                        let active_tab = self
                            .state
                            .active_tab()
                            .map(|t| t.active_tab.clone());
                        if active_tab == Some(ActiveTab::Headers) {
                            let (row, col, len) = if let Some(tab) = self.state.active_tab() {
                                let row = tab.request.headers_row;
                                let col = tab.request.headers_col;
                                let len = tab
                                    .request
                                    .headers
                                    .get(row)
                                    .map(|p| if col == 0 { p.key.len() } else { p.value.len() })
                                    .unwrap_or(0);
                                (row, col, len)
                            } else {
                                (0, 0, 0)
                            };
                            let _ = (row, col);
                            if let Some(tab) = self.state.active_tab_mut() {
                                tab.request.headers_cursor = len;
                            }
                        } else {
                            if let Some(tab) = self.state.active_tab_mut() {
                                if tab.request.body
                                    == crate::state::request_state::RequestBody::None
                                {
                                    tab.request.body =
                                        crate::state::request_state::RequestBody::Json(
                                            String::new(),
                                        );
                                }
                            }
                        }
                    }
                } else if matches!(self.state.focus, Focus::Sidebar) {
                    self.handle_sidebar_enter();
                } else if matches!(self.state.focus, Focus::RequestTabs) {
                    self.state.focus = Focus::UrlBar;
                }
            }
            KeyCode::Char('[') => {
                if self.state.focus == Focus::UrlBar {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.method = tab.request.method.prev();
                    }
                } else {
                    self.sync_active_tab_to_collection();
                    self.prev_open_tab();
                }
            }
            KeyCode::Char(']') => {
                if self.state.focus == Focus::UrlBar {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.method = tab.request.method.next();
                    }
                } else {
                    self.sync_active_tab_to_collection();
                    self.next_open_tab();
                }
            }
            KeyCode::Esc => self.cancel_request(),
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.focus == Focus::Sidebar {
                    self.sidebar_move_cursor(1);
                } else if self.state.focus == Focus::Editor {
                    let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                    if active_tab == Some(ActiveTab::Headers) {
                        if let Some(tab) = self.state.active_tab_mut() {
                            let len = tab.request.headers.len();
                            if len > 0 {
                                tab.request.headers_row =
                                    (tab.request.headers_row + 1).min(len - 1);
                            }
                        }
                    } else if let Some(tab) = self.state.active_tab_mut() {
                        if let Some(resp) = &mut tab.response {
                            resp.scroll_offset = resp.scroll_offset.saturating_add(1);
                        }
                    }
                } else if let Some(tab) = self.state.active_tab_mut() {
                    if let Some(resp) = &mut tab.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_add(1);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.focus == Focus::Sidebar {
                    self.sidebar_move_cursor_up();
                } else if self.state.focus == Focus::Editor {
                    let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                    if active_tab == Some(ActiveTab::Headers) {
                        if let Some(tab) = self.state.active_tab_mut() {
                            tab.request.headers_row =
                                tab.request.headers_row.saturating_sub(1);
                        }
                    } else if let Some(tab) = self.state.active_tab_mut() {
                        if let Some(resp) = &mut tab.response {
                            resp.scroll_offset = resp.scroll_offset.saturating_sub(1);
                        }
                    }
                } else if let Some(tab) = self.state.active_tab_mut() {
                    if let Some(resp) = &mut tab.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_sub(1);
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h')
                if self.state.focus == Focus::TabBar =>
            {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.active_tab = tab.active_tab.prev();
                }
            }
            KeyCode::Right | KeyCode::Char('l')
                if self.state.focus == Focus::TabBar =>
            {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.active_tab = tab.active_tab.next();
                }
            }
            KeyCode::Char('h') if self.state.focus == Focus::Sidebar => {
                self.sidebar_collapse();
            }
            KeyCode::Char('l') if self.state.focus == Focus::Sidebar => {
                self.sidebar_expand();
            }
            KeyCode::Left
                if self.state.focus == Focus::Editor =>
            {
                let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                if active_tab == Some(ActiveTab::Headers) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.headers_col = 0;
                        let row = tab.request.headers_row;
                        let len =
                            tab.request.headers.get(row).map(|p| p.key.len()).unwrap_or(0);
                        tab.request.headers_cursor = len;
                    }
                }
            }
            KeyCode::Right
                if self.state.focus == Focus::Editor =>
            {
                let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                if active_tab == Some(ActiveTab::Headers) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.headers_col = 1;
                        let row = tab.request.headers_row;
                        let len =
                            tab.request.headers.get(row).map(|p| p.value.len()).unwrap_or(0);
                        tab.request.headers_cursor = len;
                    }
                }
            }
            KeyCode::Char('a')
                if self.state.focus == Focus::Editor =>
            {
                let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                if active_tab == Some(ActiveTab::Headers) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.headers.push(KeyValuePair::default());
                        let new_row = tab.request.headers.len() - 1;
                        tab.request.headers_row = new_row;
                        tab.request.headers_col = 0;
                        tab.request.headers_cursor = 0;
                        self.state.mode = Mode::Insert;
                    }
                }
            }
            KeyCode::Char('x') | KeyCode::Char('d')
                if self.state.focus == Focus::Editor =>
            {
                let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                if active_tab == Some(ActiveTab::Headers) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let len = tab.request.headers.len();
                        if len > 0 {
                            tab.request.headers.remove(tab.request.headers_row);
                            let new_len = tab.request.headers.len();
                            tab.request.headers_row = if new_len > 0 {
                                tab.request.headers_row.min(new_len - 1)
                            } else {
                                0
                            };
                        }
                    }
                }
            }
            KeyCode::Char(' ')
                if self.state.focus == Focus::Editor =>
            {
                let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
                if active_tab == Some(ActiveTab::Headers) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let row = tab.request.headers_row;
                        if let Some(pair) = tab.request.headers.get_mut(row) {
                            pair.enabled = !pair.enabled;
                        }
                    }
                }
            }
            // Sidebar-specific keys
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) && self.state.focus == Focus::Sidebar => {
                self.state.naming = NamingState {
                    target: NamingTarget::NewCollection,
                    ..NamingState::default()
                };
                self.state.active_popup = ActivePopup::CollectionNaming;
            }
            KeyCode::Char('n') if self.state.focus == Focus::Sidebar => {
                // New request at current cursor context
                let target = self.sidebar_new_request_target();
                self.state.naming = NamingState {
                    target,
                    method: "GET".to_string(),
                    ..NamingState::default()
                };
                self.state.active_popup = ActivePopup::CollectionNaming;
            }
            KeyCode::Char('f') if self.state.focus == Focus::Sidebar => {
                // New folder at current cursor context
                let target = self.sidebar_new_folder_target();
                self.state.naming = NamingState {
                    target,
                    ..NamingState::default()
                };
                self.state.active_popup = ActivePopup::CollectionNaming;
            }
            KeyCode::Char('r') if self.state.focus == Focus::Sidebar => {
                self.sidebar_rename();
            }
            KeyCode::Char('d') if self.state.focus == Focus::Sidebar => {
                self.sidebar_delete();
            }
            KeyCode::Char('D') if self.state.focus == Focus::Sidebar => {
                self.sidebar_duplicate();
            }
            KeyCode::Char('/') if self.state.focus == Focus::Sidebar => {
                self.state.sidebar.search_mode = true;
                self.state.sidebar.search_query.clear();
            }
            // RequestTabs-specific keys
            KeyCode::Left if self.state.focus == Focus::RequestTabs => {
                self.sync_active_tab_to_collection();
                self.prev_open_tab();
            }
            KeyCode::Right if self.state.focus == Focus::RequestTabs => {
                self.sync_active_tab_to_collection();
                self.next_open_tab();
            }
            KeyCode::Char('x') if self.state.focus == Focus::RequestTabs => {
                self.sync_active_tab_to_collection();
                self.close_active_tab();
            }
            KeyCode::Char('1') => self.state.focus = Focus::Sidebar,
            KeyCode::Char('2') => self.state.focus = Focus::UrlBar,
            KeyCode::Char('3') => self.state.focus = Focus::Editor,
            KeyCode::Char('4') => self.state.focus = Focus::ResponseViewer,
            _ => {}
        }
    }

    // ─── Sidebar helpers ──────────────────────────────────────────────────────

    fn sidebar_move_cursor(&mut self, delta: usize) {
        let nodes = flatten_tree(&self.state);
        let max = nodes.len().saturating_sub(1);
        let new_cursor = (self.state.sidebar.cursor + delta).min(max);
        self.state.sidebar.cursor = new_cursor;
        // Scroll down if needed
        // (We'll implement simple scroll clamping — caller must know visible height)
        // For now: no-op; layout scrolls based on cursor vs scroll_offset
        self.clamp_sidebar_scroll();
    }

    fn sidebar_move_cursor_up(&mut self) {
        self.state.sidebar.cursor = self.state.sidebar.cursor.saturating_sub(1);
        self.clamp_sidebar_scroll();
    }

    fn clamp_sidebar_scroll(&mut self) {
        // Keep cursor visible — conservative 20-line window
        let visible = 20usize;
        let cursor = self.state.sidebar.cursor;
        let scroll = self.state.sidebar.scroll_offset;
        if cursor < scroll {
            self.state.sidebar.scroll_offset = cursor;
        } else if cursor >= scroll + visible {
            self.state.sidebar.scroll_offset = cursor.saturating_sub(visible - 1);
        }
    }

    fn sidebar_collapse(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor) {
            match &node.kind {
                crate::ui::sidebar::NodeKind::Collection { .. }
                | crate::ui::sidebar::NodeKind::Folder { .. } => {
                    self.state.sidebar.collapsed_ids.insert(node.id.clone());
                }
                _ => {}
            }
        }
    }

    fn sidebar_expand(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor) {
            self.state.sidebar.collapsed_ids.remove(&node.id);
        }
    }

    fn handle_sidebar_enter(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor).cloned() {
            match node.kind {
                crate::ui::sidebar::NodeKind::Collection { collapsed }
                | crate::ui::sidebar::NodeKind::Folder { collapsed } => {
                    if collapsed {
                        self.state.sidebar.collapsed_ids.remove(&node.id);
                    } else {
                        self.state.sidebar.collapsed_ids.insert(node.id.clone());
                    }
                }
                crate::ui::sidebar::NodeKind::Request { method } => {
                    // Dedup: if already open, just focus it
                    if let Some(idx) = self.state.workspace.open_tabs.iter()
                        .position(|t| t.collection_id.as_deref() == Some(&node.id))
                    {
                        self.state.workspace.active_tab_idx = idx;
                        return;
                    }
                    // Load persisted state from collection
                    let saved = find_col_request_by_id(&self.state.workspace.collections, &node.id).cloned();
                    let mut tab = RequestTab::default();
                    tab.request.name = node.label.clone();
                    tab.request.method = crate::state::request_state::HttpMethod::from_str_or_get(&method);
                    tab.collection_id = Some(node.id.clone());
                    if let Some(saved) = saved {
                        tab.request.url = saved.url.clone();
                        if !saved.body_raw.is_empty() {
                            tab.request.body = crate::state::request_state::RequestBody::Json(saved.body_raw.clone());
                        }
                    }
                    self.state.workspace.open_tabs.push(tab);
                    self.state.workspace.active_tab_idx = self.state.workspace.open_tabs.len() - 1;
                }
            }
        }
    }

    fn sidebar_new_request_target(&self) -> NamingTarget {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor) {
            let col_id = self.find_collection_id_for_node(&node.id);
            let folder_id = match &node.kind {
                crate::ui::sidebar::NodeKind::Folder { .. } => Some(node.id.clone()),
                _ => None,
            };
            if let Some(cid) = col_id {
                return NamingTarget::NewRequest {
                    collection_id: cid,
                    folder_id,
                };
            }
        }
        NamingTarget::NewCollection
    }

    fn sidebar_new_folder_target(&self) -> NamingTarget {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor) {
            let col_id = self.find_collection_id_for_node(&node.id);
            if let Some(cid) = col_id {
                return NamingTarget::NewFolder { collection_id: cid };
            }
        }
        NamingTarget::NewCollection
    }

    fn find_collection_id_for_node(&self, node_id: &str) -> Option<String> {
        for col in &self.state.workspace.collections {
            if col.id == node_id {
                return Some(col.id.clone());
            }
            if item_exists_in_list(&col.items, node_id) {
                return Some(col.id.clone());
            }
        }
        None
    }

    fn sidebar_rename(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor).cloned() {
            self.state.naming = NamingState {
                target: NamingTarget::Rename {
                    id: node.id.clone(),
                    old_name: node.label.clone(),
                },
                input: node.label.clone(),
                cursor: node.label.len(),
                ..NamingState::default()
            };
            self.state.active_popup = ActivePopup::CollectionNaming;
        }
    }

    fn sidebar_delete(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor).cloned() {
            let msg = format!("Delete \"{}\"?", node.label);
            self.state.confirm_delete = ConfirmDeleteState {
                message: msg,
                target_id: node.id.clone(),
            };
            self.state.active_popup = ActivePopup::ConfirmDelete;
        }
    }

    fn sidebar_duplicate(&mut self) {
        let nodes = flatten_tree(&self.state);
        if let Some(node) = nodes.get(self.state.sidebar.cursor).cloned() {
            if let crate::ui::sidebar::NodeKind::Request { method } = &node.kind {
                let new_req = CollectionRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{} (copy)", node.label),
                    method: method.clone(),
                    url: String::new(),
                    body_raw: String::new(),
                };
                let ws_name = self.state.workspace.name.clone();
                // Insert after cursor in the containing collection/folder
                for col in &mut self.state.workspace.collections {
                    if insert_after_in_list(
                        &mut col.items,
                        &node.id,
                        CollectionItem::Request(new_req.clone()),
                    ) {
                        let _ = col_storage::save_collection_meta(&ws_name, col);
                        break;
                    }
                    // Also check if the original is directly in the collection
                    if col.items.iter().any(|item| match item {
                        CollectionItem::Request(r) => r.id == node.id,
                        _ => false,
                    }) {
                        col.items.push(CollectionItem::Request(new_req.clone()));
                        let _ = col_storage::save_collection_meta(&ws_name, col);
                        break;
                    }
                }
            }
        }
    }

    // ─── Open tab management ──────────────────────────────────────────────────

    fn next_open_tab(&mut self) {
        let len = self.state.workspace.open_tabs.len();
        if len == 0 {
            return;
        }
        self.state.workspace.active_tab_idx =
            (self.state.workspace.active_tab_idx + 1) % len;
    }

    fn prev_open_tab(&mut self) {
        let len = self.state.workspace.open_tabs.len();
        if len == 0 {
            return;
        }
        self.state.workspace.active_tab_idx =
            (self.state.workspace.active_tab_idx + len - 1) % len;
    }

    fn close_active_tab(&mut self) {
        let idx = self.state.workspace.active_tab_idx;
        let len = self.state.workspace.open_tabs.len();
        if len == 0 {
            return;
        }
        self.state.workspace.open_tabs.remove(idx);
        if self.state.workspace.open_tabs.is_empty() {
            self.state.workspace.open_tabs.push(RequestTab::default());
            self.state.workspace.active_tab_idx = 0;
        } else {
            self.state.workspace.active_tab_idx =
                self.state.workspace.active_tab_idx.min(
                    self.state.workspace.open_tabs.len() - 1,
                );
        }
    }

    // ─── Collection sync ──────────────────────────────────────────────────────

    fn sync_active_tab_to_collection(&mut self) {
        let idx = self.state.workspace.active_tab_idx;
        if let Some(tab) = self.state.workspace.open_tabs.get(idx) {
            let Some(req_id) = tab.collection_id.clone() else { return };
            let url = tab.request.url.clone();
            let method = tab.request.method.as_str().to_string();
            let body_raw = match &tab.request.body {
                crate::state::request_state::RequestBody::Json(s)
                | crate::state::request_state::RequestBody::Text(s) => s.clone(),
                _ => String::new(),
            };
            let ws_name = self.state.workspace.name.clone();
            for col in &mut self.state.workspace.collections {
                if update_col_request_state(&mut col.items, &req_id, &url, &method, &body_raw) {
                    let _ = col_storage::save_collection_meta(&ws_name, col);
                    break;
                }
            }
        }
    }

    // ─── Insert key handling ──────────────────────────────────────────────────

    fn handle_insert_key(&mut self, key: KeyEvent) {
        // Check if we're in sidebar search mode
        if self.state.focus == Focus::Sidebar && self.state.sidebar.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.state.sidebar.search_mode = false;
                    self.state.sidebar.search_query.clear();
                    self.state.mode = Mode::Normal;
                }
                KeyCode::Char(c) => {
                    self.state.sidebar.search_query.push(c);
                }
                KeyCode::Backspace => {
                    self.state.sidebar.search_query.pop();
                    if self.state.sidebar.search_query.is_empty() {
                        self.state.sidebar.search_mode = false;
                        self.state.mode = Mode::Normal;
                    }
                }
                _ => {}
            }
            return;
        }

        let active_tab = self.state.active_tab().map(|t| t.active_tab.clone());
        if self.state.focus == Focus::Editor && active_tab == Some(ActiveTab::Headers) {
            self.handle_headers_insert_key(key);
            return;
        }
        match key.code {
            KeyCode::Esc => self.state.mode = Mode::Normal,
            KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.mode = Mode::Normal;
                    self.send_request();
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                            let cursor = tab.request.body_cursor;
                            text.insert(cursor, '\n');
                            tab.request.body_cursor = cursor + 1;
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.url_cursor;
                        tab.request.url.insert(cursor, c);
                        tab.request.url_cursor += c.len_utf8();
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                            let cursor = tab.request.body_cursor;
                            text.insert(cursor, c);
                            tab.request.body_cursor = cursor + c.len_utf8();
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.url_cursor;
                        if cursor > 0 {
                            let url = tab.request.url.clone();
                            let prev = Self::prev_char_boundary_of(&url, cursor);
                            tab.request.url.drain(prev..cursor);
                            tab.request.url_cursor = prev;
                        }
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        if cursor > 0 {
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                let prev = Self::prev_char_boundary_of(text, cursor);
                                text.drain(prev..cursor);
                                tab.request.body_cursor = prev;
                            }
                        }
                    }
                }
            }
            KeyCode::Delete => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.url_cursor;
                        let url = tab.request.url.clone();
                        if cursor < url.len() {
                            let next = Self::next_char_boundary_of(&url, cursor);
                            tab.request.url.drain(cursor..next);
                        }
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let body_len = match &tab.request.body {
                            crate::state::request_state::RequestBody::Json(s)
                            | crate::state::request_state::RequestBody::Text(s) => s.len(),
                            _ => 0,
                        };
                        if cursor < body_len {
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                let next = Self::next_char_boundary_of(text, cursor);
                                text.drain(cursor..next);
                            }
                        }
                    }
                }
            }
            KeyCode::Left => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.url_cursor;
                        let url = tab.request.url.clone();
                        tab.request.url_cursor = Self::prev_char_boundary_of(&url, cursor);
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let new_cursor =
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                Self::prev_char_boundary_of(text, cursor)
                            } else {
                                cursor
                            };
                        tab.request.body_cursor = new_cursor;
                    }
                }
            }
            KeyCode::Right => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.url_cursor;
                        let url = tab.request.url.clone();
                        tab.request.url_cursor = Self::next_char_boundary_of(&url, cursor);
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let new_cursor =
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                Self::next_char_boundary_of(text, cursor)
                            } else {
                                cursor
                            };
                        tab.request.body_cursor = new_cursor;
                    }
                }
            }
            KeyCode::Up => {
                if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let body_snapshot = match &tab.request.body {
                            crate::state::request_state::RequestBody::Json(s)
                            | crate::state::request_state::RequestBody::Text(s) => s.clone(),
                            _ => String::new(),
                        };
                        tab.request.body_cursor = Self::body_move_up(&body_snapshot, cursor);
                    }
                }
            }
            KeyCode::Down => {
                if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let body_snapshot = match &tab.request.body {
                            crate::state::request_state::RequestBody::Json(s)
                            | crate::state::request_state::RequestBody::Text(s) => s.clone(),
                            _ => String::new(),
                        };
                        tab.request.body_cursor = Self::body_move_down(&body_snapshot, cursor);
                    }
                }
            }
            KeyCode::Home => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.url_cursor = 0;
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let new_cursor =
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                let before = &text[..cursor.min(text.len())];
                                match before.rfind('\n') {
                                    Some(i) => i + 1,
                                    None => 0,
                                }
                            } else {
                                cursor
                            };
                        tab.request.body_cursor = new_cursor;
                    }
                }
            }
            KeyCode::End => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        tab.request.url_cursor = tab.request.url.len();
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(tab) = self.state.active_tab_mut() {
                        let cursor = tab.request.body_cursor;
                        let new_cursor =
                            if let Some(text) = Self::body_text_mut(&mut tab.request.body) {
                                let after_start = cursor.min(text.len());
                                let after = &text[after_start..];
                                match after.find('\n') {
                                    Some(i) => after_start + i,
                                    None => text.len(),
                                }
                            } else {
                                cursor
                            };
                        tab.request.body_cursor = new_cursor;
                    }
                }
            }
            _ => {}
        }
    }

    /// Get a mutable reference to the body text string.
    fn body_text_mut(body: &mut crate::state::request_state::RequestBody) -> Option<&mut String> {
        use crate::state::request_state::RequestBody;
        match body {
            RequestBody::Json(s) | RequestBody::Text(s) => Some(s),
            RequestBody::None => {
                *body = RequestBody::Json(String::new());
                match body {
                    RequestBody::Json(s) => Some(s),
                    _ => None,
                }
            }
            RequestBody::Form(_) | RequestBody::Binary(_) => None,
        }
    }

    fn headers_active_text_mut(
        headers: &mut Vec<KeyValuePair>,
        row: usize,
        col: u8,
    ) -> Option<&mut String> {
        let pair = headers.get_mut(row)?;
        if col == 0 { Some(&mut pair.key) } else { Some(&mut pair.value) }
    }

    fn handle_headers_insert_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let cursor = tab.request.headers_cursor;
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    if let Some(text) =
                        Self::headers_active_text_mut(&mut tab.request.headers, row, col)
                    {
                        text.insert(cursor, c);
                        tab.request.headers_cursor = cursor + c.len_utf8();
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let cursor = tab.request.headers_cursor;
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    if cursor > 0 {
                        if let Some(text) =
                            Self::headers_active_text_mut(&mut tab.request.headers, row, col)
                        {
                            let prev = Self::prev_char_boundary_of(text, cursor);
                            text.drain(prev..cursor);
                            tab.request.headers_cursor = prev;
                        }
                    }
                }
            }
            KeyCode::Delete => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let cursor = tab.request.headers_cursor;
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    if let Some(text) =
                        Self::headers_active_text_mut(&mut tab.request.headers, row, col)
                    {
                        if cursor < text.len() {
                            let next = Self::next_char_boundary_of(text, cursor);
                            text.drain(cursor..next);
                        }
                    }
                }
            }
            KeyCode::Left => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let cursor = tab.request.headers_cursor;
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    let new_cursor = if let Some(text) =
                        Self::headers_active_text_mut(&mut tab.request.headers, row, col)
                    {
                        Self::prev_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                    tab.request.headers_cursor = new_cursor;
                }
            }
            KeyCode::Right => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let cursor = tab.request.headers_cursor;
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    let new_cursor = if let Some(text) =
                        Self::headers_active_text_mut(&mut tab.request.headers, row, col)
                    {
                        Self::next_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                    tab.request.headers_cursor = new_cursor;
                }
            }
            KeyCode::Home => {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.request.headers_cursor = 0;
                }
            }
            KeyCode::End => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let row = tab.request.headers_row;
                    let col = tab.request.headers_col;
                    let len = tab
                        .request
                        .headers
                        .get(row)
                        .map(|p| if col == 0 { p.key.len() } else { p.value.len() })
                        .unwrap_or(0);
                    tab.request.headers_cursor = len;
                }
            }
            KeyCode::Tab => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let col = tab.request.headers_col;
                    if col == 0 {
                        tab.request.headers_col = 1;
                        let row = tab.request.headers_row;
                        let val_len = tab
                            .request
                            .headers
                            .get(row)
                            .map(|p| p.value.len())
                            .unwrap_or(0);
                        tab.request.headers_cursor = val_len;
                    } else {
                        let next_row = tab.request.headers_row + 1;
                        if next_row >= tab.request.headers.len() {
                            tab.request.headers.push(KeyValuePair::default());
                        }
                        tab.request.headers_row =
                            next_row.min(tab.request.headers.len() - 1);
                        tab.request.headers_col = 0;
                        tab.request.headers_cursor = 0;
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(tab) = self.state.active_tab_mut() {
                    let next_row = tab.request.headers_row + 1;
                    if next_row >= tab.request.headers.len() {
                        tab.request.headers.push(KeyValuePair::default());
                    }
                    tab.request.headers_row = next_row.min(tab.request.headers.len() - 1);
                    tab.request.headers_col = 0;
                    tab.request.headers_cursor = 0;
                }
            }
            _ => {}
        }
    }

    // ─── Char boundary helpers ────────────────────────────────────────────────

    fn prev_char_boundary_of(text: &str, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let mut p = pos - 1;
        while p > 0 && !text.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn next_char_boundary_of(text: &str, pos: usize) -> usize {
        if pos >= text.len() {
            return text.len();
        }
        let mut p = pos + 1;
        while p < text.len() && !text.is_char_boundary(p) {
            p += 1;
        }
        p
    }

    fn body_move_up(text: &str, cursor: usize) -> usize {
        let clamped = cursor.min(text.len());
        let before = &text[..clamped];
        let lines: Vec<&str> = before.split('\n').collect();
        let current_row = lines.len().saturating_sub(1);
        let current_col = lines.last().map(|l| l.chars().count()).unwrap_or(0);
        if current_row == 0 {
            return 0;
        }
        let target_row = current_row - 1;
        let rows: Vec<&str> = text.split('\n').collect();
        let target_line = rows.get(target_row).copied().unwrap_or("");
        let target_col = current_col.min(target_line.chars().count());
        let row_start: usize = rows[..target_row].iter().map(|l| l.len() + 1).sum();
        let col_bytes: usize = target_line
            .char_indices()
            .nth(target_col)
            .map(|(i, _)| i)
            .unwrap_or(target_line.len());
        row_start + col_bytes
    }

    fn body_move_down(text: &str, cursor: usize) -> usize {
        let clamped = cursor.min(text.len());
        let before = &text[..clamped];
        let lines_before: Vec<&str> = before.split('\n').collect();
        let current_row = lines_before.len().saturating_sub(1);
        let current_col = lines_before.last().map(|l| l.chars().count()).unwrap_or(0);
        let rows: Vec<&str> = text.split('\n').collect();
        let target_row = current_row + 1;
        if target_row >= rows.len() {
            return text.len();
        }
        let target_line = rows[target_row];
        let target_col = current_col.min(target_line.chars().count());
        let row_start: usize = rows[..target_row].iter().map(|l| l.len() + 1).sum();
        let col_bytes: usize = target_line
            .char_indices()
            .nth(target_col)
            .map(|(i, _)| i)
            .unwrap_or(target_line.len());
        row_start + col_bytes
    }

    // ─── Mouse handling ───────────────────────────────────────────────────────

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if let Some(tab) = self.state.active_tab_mut() {
                    if let Some(resp) = &mut tab.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_add(3);
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(tab) = self.state.active_tab_mut() {
                    if let Some(resp) = &mut tab.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_sub(3);
                    }
                }
            }
            _ => {}
        }
    }

    // ─── Response handling ────────────────────────────────────────────────────

    fn handle_response(&mut self, result: Result<ResponseState, AppError>) {
        self.cancel = None;
        match result {
            Ok(mut response) => {
                if let ResponseBody::Text(text) = &response.body {
                    let lang = detect_lang(text);
                    response.highlighted_body = Some(highlight_text(text, lang));
                }
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.response = Some(response);
                    tab.request_status = RequestStatus::Idle;
                }
                self.sync_active_tab_to_collection();
            }
            Err(AppError::Cancelled) => {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.request_status = RequestStatus::Idle;
                }
            }
            Err(e) => {
                if let Some(tab) = self.state.active_tab_mut() {
                    tab.request_status = RequestStatus::Error(e.to_string());
                }
            }
        }
    }

    // ─── Tick handling ────────────────────────────────────────────────────────

    fn handle_tick(&mut self) {
        if let Some(tab) = self.state.active_tab_mut() {
            if let RequestStatus::Loading { spinner_tick } = &mut tab.request_status {
                *spinner_tick = spinner_tick.wrapping_add(1);
                self.state.dirty = true;
            }
        }
    }

    // ─── HTTP request ─────────────────────────────────────────────────────────

    fn send_request(&mut self) {
        let url_empty = self
            .state
            .active_tab()
            .map(|t| t.request.url.is_empty())
            .unwrap_or(true);
        if url_empty {
            return;
        }

        if let Some(token) = self.cancel.take() {
            token.cancel();
        }
        let token = CancellationToken::new();
        self.cancel = Some(token.clone());

        if let Some(tab) = self.state.active_tab_mut() {
            tab.request_status = RequestStatus::Loading { spinner_tick: 0 };
            tab.response = None;
        }

        // Build resolver and resolve URL + headers before cloning for the task
        let resolver = resolver_from_state(&self.state);
        let request = if let Some(tab) = self.state.active_tab() {
            let mut req = tab.request.clone();
            req.url = resolver.resolve_for_send(&req.url);
            for header in &mut req.headers {
                if header.enabled {
                    header.key = resolver.resolve_for_send(&header.key);
                    header.value = resolver.resolve_for_send(&header.value);
                }
            }
            req
        } else {
            return;
        };

        let client = self.client.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            execute(client, request, tx, token).await;
        });
    }

    pub fn cancel_request(&mut self) {
        if let Some(token) = self.cancel.take() {
            token.cancel();
        }
        if let Some(tab) = self.state.active_tab_mut() {
            tab.request_status = RequestStatus::Idle;
        }
    }
}

// ─── Trait extension for HttpMethod ──────────────────────────────────────────

trait HttpMethodExt {
    fn from_str_or_get(s: &str) -> crate::state::request_state::HttpMethod;
}

impl HttpMethodExt for crate::state::request_state::HttpMethod {
    fn from_str_or_get(s: &str) -> Self {
        use crate::state::request_state::HttpMethod;
        match s {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => HttpMethod::Get,
        }
    }
}

// ─── HTTP method cycling ──────────────────────────────────────────────────────

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

fn cycle_method_next(m: &str) -> String {
    let pos = METHODS.iter().position(|&x| x == m).unwrap_or(0);
    METHODS[(pos + 1) % METHODS.len()].to_string()
}

fn cycle_method_prev(m: &str) -> String {
    let pos = METHODS.iter().position(|&x| x == m).unwrap_or(0);
    METHODS[(pos + METHODS.len() - 1) % METHODS.len()].to_string()
}

// ─── Collection tree helpers ──────────────────────────────────────────────────

fn add_request_to_folder(
    items: &mut Vec<CollectionItem>,
    folder_id: &str,
    req: CollectionItem,
) -> bool {
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if f.id == folder_id {
                f.items.push(req);
                return true;
            }
            if add_request_to_folder(&mut f.items, folder_id, req.clone()) {
                return true;
            }
        }
    }
    false
}

fn rename_item_in_list(items: &mut Vec<CollectionItem>, id: &str, name: &str) -> bool {
    for item in items.iter_mut() {
        match item {
            CollectionItem::Folder(f) => {
                if f.id == id {
                    f.name = name.to_string();
                    return true;
                }
                if rename_item_in_list(&mut f.items, id, name) {
                    return true;
                }
            }
            CollectionItem::Request(r) => {
                if r.id == id {
                    r.name = name.to_string();
                    return true;
                }
            }
        }
    }
    false
}

fn remove_item_from_list(items: &mut Vec<CollectionItem>, id: &str) -> bool {
    let before = items.len();
    items.retain(|item| match item {
        CollectionItem::Folder(f) => f.id != id,
        CollectionItem::Request(r) => r.id != id,
    });
    if items.len() < before {
        return true;
    }
    // Recurse into folders
    for item in items.iter_mut() {
        if let CollectionItem::Folder(f) = item {
            if remove_item_from_list(&mut f.items, id) {
                return true;
            }
        }
    }
    false
}

fn item_exists_in_list(items: &[CollectionItem], id: &str) -> bool {
    for item in items {
        match item {
            CollectionItem::Folder(f) => {
                if f.id == id || item_exists_in_list(&f.items, id) {
                    return true;
                }
            }
            CollectionItem::Request(r) => {
                if r.id == id {
                    return true;
                }
            }
        }
    }
    false
}

fn insert_after_in_list(
    items: &mut Vec<CollectionItem>,
    after_id: &str,
    new_item: CollectionItem,
) -> bool {
    for i in 0..items.len() {
        let matches = match &items[i] {
            CollectionItem::Folder(f) => f.id == after_id,
            CollectionItem::Request(r) => r.id == after_id,
        };
        if matches {
            items.insert(i + 1, new_item);
            return true;
        }
        if let CollectionItem::Folder(f) = &mut items[i] {
            if insert_after_in_list(&mut f.items, after_id, new_item.clone()) {
                return true;
            }
        }
    }
    false
}

fn find_col_request_by_id<'a>(
    collections: &'a [Collection],
    id: &str,
) -> Option<&'a CollectionRequest> {
    for col in collections {
        if let Some(r) = find_request_in_items(&col.items, id) {
            return Some(r);
        }
    }
    None
}

fn find_request_in_items<'a>(
    items: &'a [CollectionItem],
    id: &str,
) -> Option<&'a CollectionRequest> {
    for item in items {
        match item {
            CollectionItem::Request(r) if r.id == id => return Some(r),
            CollectionItem::Folder(f) => {
                if let Some(r) = find_request_in_items(&f.items, id) {
                    return Some(r);
                }
            }
            _ => {}
        }
    }
    None
}

fn update_col_request_state(
    items: &mut Vec<CollectionItem>,
    id: &str,
    url: &str,
    method: &str,
    body_raw: &str,
) -> bool {
    for item in items.iter_mut() {
        match item {
            CollectionItem::Request(r) if r.id == id => {
                r.url = url.to_string();
                r.method = method.to_string();
                r.body_raw = body_raw.to_string();
                return true;
            }
            CollectionItem::Folder(f) => {
                if update_col_request_state(&mut f.items, id, url, method, body_raw) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
