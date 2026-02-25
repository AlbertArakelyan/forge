use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::event::Event;
use crate::http::{client::build_client, executor::execute};
use crate::state::app_state::{ActivePopup, ActiveTab, AppState, RequestStatus};
use crate::state::environment::{EnvVariable, Environment, VarType};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::request_state::KeyValuePair;
use crate::state::response_state::{ResponseBody, ResponseState};
use crate::env::resolver::resolver_from_state;
use crate::storage::environment as env_storage;
use crate::ui::highlight::{detect_lang, highlight_text};

pub struct App {
    pub state: AppState,
    client: reqwest::Client,
    tx: UnboundedSender<Event>,
    cancel: Option<CancellationToken>,
}

impl App {
    pub fn new(tx: UnboundedSender<Event>) -> Self {
        let environments = env_storage::load_all();
        Self {
            state: AppState {
                sidebar_visible: true,
                dirty: true,
                environments,
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
        match self.state.active_popup {
            ActivePopup::EnvSwitcher => self.handle_env_switcher_key(key),
            ActivePopup::EnvEditor => self.handle_env_editor_key(key),
            ActivePopup::None => {}
        }
    }

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
                    .environments
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                    .nth(selected)
                    .map(|(i, _)| i);
                if let Some(i) = idx {
                    self.state.active_env_idx = Some(i);
                }
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char('e') if key.modifiers.is_empty() => {
                // Open editor for selected environment
                let filter = self.state.env_switcher.search.to_lowercase();
                let selected = self.state.env_switcher.selected;
                let idx = self
                    .state
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
                } else if self.state.environments.is_empty() {
                    // No environments — create a new one and open editor
                    let new_env = Environment::default();
                    self.state.environments.push(new_env);
                    let i = self.state.environments.len() - 1;
                    self.state.env_editor.env_idx = i;
                    self.state.env_editor.row = 0;
                    self.state.env_editor.col = 0;
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = false;
                    self.state.env_editor.show_secret = false;
                    self.state.active_popup = ActivePopup::EnvEditor;
                }
            }
            KeyCode::Char('n') if key.modifiers.is_empty() => {
                // Enter naming mode — the environment is created after Enter
                self.state.env_switcher.naming = true;
                self.state.env_switcher.new_name = String::new();
                self.state.env_switcher.new_name_cursor = 0;
            }
            KeyCode::Char('d') if key.modifiers.is_empty() => {
                // Delete the selected environment
                let filter = self.state.env_switcher.search.to_lowercase();
                let selected = self.state.env_switcher.selected;
                let idx = self
                    .state
                    .environments
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                    .nth(selected)
                    .map(|(i, _)| i);
                if let Some(i) = idx {
                    let env_id = self.state.environments[i].id.clone();
                    let _ = env_storage::delete(&env_id);
                    self.state.environments.remove(i);
                    // Update active_env_idx
                    match self.state.active_env_idx {
                        Some(ai) if ai == i => self.state.active_env_idx = None,
                        Some(ai) if ai > i => self.state.active_env_idx = Some(ai - 1),
                        _ => {}
                    }
                    // Clamp selected
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
                self.state.environments.push(new_env);
                let i = self.state.environments.len() - 1;
                self.state.env_switcher.selected = i;
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
                // Save and close
                self.save_current_env();
                self.state.active_popup = ActivePopup::None;
            }
            KeyCode::Char('i') | KeyCode::Enter => {
                // Start editing the current cell (except checkbox and type toggle cols)
                let col = self.state.env_editor.col;
                if col < 3 {
                    self.state.env_editor.editing = true;
                    // Set cursor to end of current field
                    let cursor = self.current_editor_field_len();
                    self.state.env_editor.cursor = cursor;
                }
            }
            KeyCode::Char('a') => {
                // Add a new variable row
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.environments.get_mut(idx) {
                    env.variables.push(EnvVariable::default());
                    self.state.env_editor.row = env.variables.len() - 1;
                    self.state.env_editor.col = 0;
                    self.state.env_editor.cursor = 0;
                    self.state.env_editor.editing = true;
                }
            }
            KeyCode::Char('d') => {
                // Delete current row
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.environments.get_mut(idx) {
                    let row = self.state.env_editor.row;
                    if row < env.variables.len() {
                        env.variables.remove(row);
                        let new_len = env.variables.len();
                        if new_len > 0 {
                            self.state.env_editor.row = row.min(new_len - 1);
                        } else {
                            self.state.env_editor.row = 0;
                        }
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let idx = self.state.env_editor.env_idx;
                let len = self.state.environments.get(idx).map(|e| e.variables.len()).unwrap_or(0);
                if len > 0 {
                    self.state.env_editor.row =
                        (self.state.env_editor.row + 1).min(len - 1);
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
                // Enter name-editing mode
                let idx = self.state.env_editor.env_idx;
                if let Some(env) = self.state.environments.get(idx) {
                    self.state.env_editor.name_cursor = env.name.len();
                    self.state.env_editor.editing_name = true;
                }
            }
            KeyCode::Char(' ') => {
                let idx = self.state.env_editor.env_idx;
                let row = self.state.env_editor.row;
                let col = self.state.env_editor.col;
                if let Some(env) = self.state.environments.get_mut(idx) {
                    if let Some(var) = env.variables.get_mut(row) {
                        match col {
                            0 => var.enabled = !var.enabled,
                            3 => {
                                // Toggle secret/text
                                var.var_type = if var.var_type == VarType::Secret {
                                    VarType::Text
                                } else {
                                    VarType::Secret
                                };
                            }
                            _ => {
                                // Toggle show_secret when on value col
                                if col == 1 {
                                    self.state.env_editor.show_secret = !self.state.env_editor.show_secret;
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
                    // Move to next row
                    let idx = self.state.env_editor.env_idx;
                    let len = self.state.environments.get(idx).map(|e| e.variables.len()).unwrap_or(0);
                    let next_row = self.state.env_editor.row + 1;
                    if next_row >= len {
                        if let Some(env) = self.state.environments.get_mut(idx) {
                            env.variables.push(EnvVariable::default());
                        }
                    }
                    let new_len = self.state.environments.get(idx).map(|e| e.variables.len()).unwrap_or(0);
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
                if let Some(env) = self.state.environments.get_mut(idx) {
                    env.name.insert(cursor, c);
                    self.state.env_editor.name_cursor = cursor + c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if cursor > 0 {
                    if let Some(env) = self.state.environments.get_mut(idx) {
                        let prev = Self::prev_char_boundary_of(&env.name, cursor);
                        env.name.drain(prev..cursor);
                        self.state.env_editor.name_cursor = prev;
                    }
                }
            }
            KeyCode::Delete => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.environments.get_mut(idx) {
                    if cursor < env.name.len() {
                        let next = Self::next_char_boundary_of(&env.name, cursor);
                        env.name.drain(cursor..next);
                    }
                }
            }
            KeyCode::Left => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.environments.get(idx) {
                    self.state.env_editor.name_cursor =
                        Self::prev_char_boundary_of(&env.name, cursor);
                }
            }
            KeyCode::Right => {
                let idx = self.state.env_editor.env_idx;
                let cursor = self.state.env_editor.name_cursor;
                if let Some(env) = self.state.environments.get(idx) {
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
        self.state.environments.get(idx)
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
        let var = self.state.environments.get_mut(idx)?.variables.get_mut(row)?;
        match col {
            0 => Some(&mut var.key),
            1 => Some(&mut var.value),
            2 => Some(&mut var.description),
            _ => None,
        }
    }

    fn save_current_env(&self) {
        let idx = self.state.env_editor.env_idx;
        if let Some(env) = self.state.environments.get(idx) {
            let _ = env_storage::save(env);
        }
    }

    // -------------------------------------------------------------------------
    // Normal key handling (unchanged from Round 1)
    // -------------------------------------------------------------------------

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Tab => self.state.focus = self.state.focus.next(),
            KeyCode::BackTab => self.state.focus = self.state.focus.prev(),
            KeyCode::Char('i') | KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar | Focus::Editor) {
                    self.state.mode = Mode::Insert;
                    if self.state.focus == Focus::Editor {
                        if self.state.active_tab == ActiveTab::Headers {
                            // Set cursor to end of active cell
                            let row = self.state.request.headers_row;
                            let col = self.state.request.headers_col;
                            if let Some(pair) = self.state.request.headers.get(row) {
                                let len = if col == 0 { pair.key.len() } else { pair.value.len() };
                                self.state.request.headers_cursor = len;
                            }
                        } else {
                            // Body editor: initialize body to Json if None
                            if self.state.request.body == crate::state::request_state::RequestBody::None {
                                self.state.request.body = crate::state::request_state::RequestBody::Json(String::new());
                            }
                        }
                    }
                }
            }
            KeyCode::Char('[') => {
                self.state.request.method = self.state.request.method.prev();
            }
            KeyCode::Char(']') => {
                self.state.request.method = self.state.request.method.next();
            }
            KeyCode::Esc => self.cancel_request(),
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers
                {
                    let len = self.state.request.headers.len();
                    if len > 0 {
                        self.state.request.headers_row =
                            (self.state.request.headers_row + 1).min(len - 1);
                    }
                } else if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers
                {
                    self.state.request.headers_row =
                        self.state.request.headers_row.saturating_sub(1);
                } else if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.state.focus == Focus::TabBar => {
                self.state.active_tab = self.state.active_tab.prev();
            }
            KeyCode::Right | KeyCode::Char('l') if self.state.focus == Focus::TabBar => {
                self.state.active_tab = self.state.active_tab.next();
            }
            KeyCode::Left
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers =>
            {
                self.state.request.headers_col = 0;
                let row = self.state.request.headers_row;
                let len = self.state.request.headers.get(row).map(|p| p.key.len()).unwrap_or(0);
                self.state.request.headers_cursor = len;
            }
            KeyCode::Right
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers =>
            {
                self.state.request.headers_col = 1;
                let row = self.state.request.headers_row;
                let len = self.state.request.headers.get(row).map(|p| p.value.len()).unwrap_or(0);
                self.state.request.headers_cursor = len;
            }
            KeyCode::Char('a')
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers =>
            {
                self.state.request.headers.push(KeyValuePair::default());
                let new_row = self.state.request.headers.len() - 1;
                self.state.request.headers_row = new_row;
                self.state.request.headers_col = 0;
                self.state.request.headers_cursor = 0;
                self.state.mode = Mode::Insert;
            }
            KeyCode::Char('x') | KeyCode::Char('d')
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers =>
            {
                let len = self.state.request.headers.len();
                if len > 0 {
                    self.state.request.headers.remove(self.state.request.headers_row);
                    let new_len = self.state.request.headers.len();
                    self.state.request.headers_row = if new_len > 0 {
                        self.state.request.headers_row.min(new_len - 1)
                    } else {
                        0
                    };
                }
            }
            KeyCode::Char(' ')
                if self.state.focus == Focus::Editor
                    && self.state.active_tab == ActiveTab::Headers =>
            {
                if let Some(pair) = self.state.request.headers.get_mut(self.state.request.headers_row) {
                    pair.enabled = !pair.enabled;
                }
            }
            KeyCode::Char('1') => self.state.focus = Focus::Sidebar,
            KeyCode::Char('2') => self.state.focus = Focus::UrlBar,
            KeyCode::Char('3') => self.state.focus = Focus::Editor,
            KeyCode::Char('4') => self.state.focus = Focus::ResponseViewer,
            _ => {}
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) {
        if self.state.focus == Focus::Editor && self.state.active_tab == ActiveTab::Headers {
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
                    // Insert newline in body editor
                    if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        let cursor = self.state.request.body_cursor;
                        text.insert(cursor, '\n');
                        self.state.request.body_cursor = cursor + 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    self.state.request.url.insert(cursor, c);
                    self.state.request.url_cursor += c.len_utf8();
                } else if matches!(self.state.focus, Focus::Editor) {
                    if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        let cursor = self.state.request.body_cursor;
                        text.insert(cursor, c);
                        self.state.request.body_cursor = cursor + c.len_utf8();
                    }
                }
            }
            KeyCode::Backspace => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    if cursor > 0 {
                        let url = self.state.request.url.clone();
                        let prev = Self::prev_char_boundary_of(&url, cursor);
                        self.state.request.url.drain(prev..cursor);
                        self.state.request.url_cursor = prev;
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    if cursor > 0 {
                        if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                            let prev = Self::prev_char_boundary_of(text, cursor);
                            text.drain(prev..cursor);
                            self.state.request.body_cursor = prev;
                        }
                    }
                }
            }
            KeyCode::Delete => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    let url = self.state.request.url.clone();
                    if cursor < url.len() {
                        let next = Self::next_char_boundary_of(&url, cursor);
                        self.state.request.url.drain(cursor..next);
                    }
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let body_len = match &self.state.request.body {
                        crate::state::request_state::RequestBody::Json(s) |
                        crate::state::request_state::RequestBody::Text(s) => s.len(),
                        _ => 0,
                    };
                    if cursor < body_len {
                        if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                            let next = Self::next_char_boundary_of(text, cursor);
                            text.drain(cursor..next);
                        }
                    }
                }
            }
            KeyCode::Left => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    let url = self.state.request.url.clone();
                    self.state.request.url_cursor = Self::prev_char_boundary_of(&url, cursor);
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let new_cursor = if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        Self::prev_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                    self.state.request.body_cursor = new_cursor;
                }
            }
            KeyCode::Right => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    let url = self.state.request.url.clone();
                    self.state.request.url_cursor = Self::next_char_boundary_of(&url, cursor);
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let new_cursor = if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        Self::next_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                    self.state.request.body_cursor = new_cursor;
                }
            }
            KeyCode::Up => {
                if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let body_snapshot = match &self.state.request.body {
                        crate::state::request_state::RequestBody::Json(s) |
                        crate::state::request_state::RequestBody::Text(s) => s.clone(),
                        _ => String::new(),
                    };
                    self.state.request.body_cursor = Self::body_move_up(&body_snapshot, cursor);
                }
            }
            KeyCode::Down => {
                if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let body_snapshot = match &self.state.request.body {
                        crate::state::request_state::RequestBody::Json(s) |
                        crate::state::request_state::RequestBody::Text(s) => s.clone(),
                        _ => String::new(),
                    };
                    self.state.request.body_cursor = Self::body_move_down(&body_snapshot, cursor);
                }
            }
            KeyCode::Home => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.request.url_cursor = 0;
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let new_cursor = if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        let before = &text[..cursor.min(text.len())];
                        match before.rfind('\n') {
                            Some(i) => i + 1,
                            None => 0,
                        }
                    } else {
                        cursor
                    };
                    self.state.request.body_cursor = new_cursor;
                }
            }
            KeyCode::End => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.request.url_cursor = self.state.request.url.len();
                } else if matches!(self.state.focus, Focus::Editor) {
                    let cursor = self.state.request.body_cursor;
                    let new_cursor = if let Some(text) = Self::body_text_mut(&mut self.state.request.body) {
                        let after_start = cursor.min(text.len());
                        let after = &text[after_start..];
                        match after.find('\n') {
                            Some(i) => after_start + i,
                            None => text.len(),
                        }
                    } else {
                        cursor
                    };
                    self.state.request.body_cursor = new_cursor;
                }
            }
            _ => {}
        }
    }

    /// Get a mutable reference to the body text string.
    /// If body is None, initialize it to Json("").
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
                let cursor = self.state.request.headers_cursor;
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                if let Some(text) =
                    Self::headers_active_text_mut(&mut self.state.request.headers, row, col)
                {
                    text.insert(cursor, c);
                    self.state.request.headers_cursor = cursor + c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                let cursor = self.state.request.headers_cursor;
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                if cursor > 0 {
                    if let Some(text) =
                        Self::headers_active_text_mut(&mut self.state.request.headers, row, col)
                    {
                        let prev = Self::prev_char_boundary_of(text, cursor);
                        text.drain(prev..cursor);
                        self.state.request.headers_cursor = prev;
                    }
                }
            }
            KeyCode::Delete => {
                let cursor = self.state.request.headers_cursor;
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                if let Some(text) =
                    Self::headers_active_text_mut(&mut self.state.request.headers, row, col)
                {
                    if cursor < text.len() {
                        let next = Self::next_char_boundary_of(text, cursor);
                        text.drain(cursor..next);
                    }
                }
            }
            KeyCode::Left => {
                let cursor = self.state.request.headers_cursor;
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                let new_cursor =
                    if let Some(text) =
                        Self::headers_active_text_mut(&mut self.state.request.headers, row, col)
                    {
                        Self::prev_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                self.state.request.headers_cursor = new_cursor;
            }
            KeyCode::Right => {
                let cursor = self.state.request.headers_cursor;
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                let new_cursor =
                    if let Some(text) =
                        Self::headers_active_text_mut(&mut self.state.request.headers, row, col)
                    {
                        Self::next_char_boundary_of(text, cursor)
                    } else {
                        cursor
                    };
                self.state.request.headers_cursor = new_cursor;
            }
            KeyCode::Home => {
                self.state.request.headers_cursor = 0;
            }
            KeyCode::End => {
                let row = self.state.request.headers_row;
                let col = self.state.request.headers_col;
                let len = self.state.request.headers
                    .get(row)
                    .map(|p| if col == 0 { p.key.len() } else { p.value.len() })
                    .unwrap_or(0);
                self.state.request.headers_cursor = len;
            }
            KeyCode::Tab => {
                let col = self.state.request.headers_col;
                if col == 0 {
                    self.state.request.headers_col = 1;
                    let row = self.state.request.headers_row;
                    let val_len = self.state.request.headers
                        .get(row)
                        .map(|p| p.value.len())
                        .unwrap_or(0);
                    self.state.request.headers_cursor = val_len;
                } else {
                    let next_row = self.state.request.headers_row + 1;
                    if next_row >= self.state.request.headers.len() {
                        self.state.request.headers.push(KeyValuePair::default());
                    }
                    self.state.request.headers_row =
                        next_row.min(self.state.request.headers.len() - 1);
                    self.state.request.headers_col = 0;
                    self.state.request.headers_cursor = 0;
                }
            }
            KeyCode::Enter => {
                let next_row = self.state.request.headers_row + 1;
                if next_row >= self.state.request.headers.len() {
                    self.state.request.headers.push(KeyValuePair::default());
                }
                self.state.request.headers_row =
                    next_row.min(self.state.request.headers.len() - 1);
                self.state.request.headers_col = 0;
                self.state.request.headers_cursor = 0;
            }
            _ => {}
        }
    }

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

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_add(3);
                }
            }
            MouseEventKind::ScrollUp => {
                if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_sub(3);
                }
            }
            _ => {}
        }
    }

    fn handle_response(&mut self, result: Result<ResponseState, AppError>) {
        self.cancel = None;
        match result {
            Ok(mut response) => {
                if let ResponseBody::Text(text) = &response.body {
                    let lang = detect_lang(text);
                    response.highlighted_body = Some(highlight_text(text, lang));
                }
                self.state.response = Some(response);
                self.state.request_status = RequestStatus::Idle;
            }
            Err(AppError::Cancelled) => {
                self.state.request_status = RequestStatus::Idle;
            }
            Err(e) => {
                self.state.request_status = RequestStatus::Error(e.to_string());
            }
        }
    }

    fn handle_tick(&mut self) {
        if let RequestStatus::Loading { spinner_tick } = &mut self.state.request_status {
            *spinner_tick = spinner_tick.wrapping_add(1);
            self.state.dirty = true;
        }
    }

    fn send_request(&mut self) {
        if self.state.request.url.is_empty() {
            return;
        }
        if let Some(token) = self.cancel.take() {
            token.cancel();
        }
        let token = CancellationToken::new();
        self.cancel = Some(token.clone());
        self.state.request_status = RequestStatus::Loading { spinner_tick: 0 };
        self.state.response = None;

        // Build resolver and resolve URL + headers before cloning for the task
        let resolver = resolver_from_state(&self.state);
        let mut request = self.state.request.clone();
        request.url = resolver.resolve_for_send(&request.url);
        for header in &mut request.headers {
            if header.enabled {
                header.key = resolver.resolve_for_send(&header.key);
                header.value = resolver.resolve_for_send(&header.value);
            }
        }

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
        self.state.request_status = RequestStatus::Idle;
    }
}
