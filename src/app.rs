use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::event::Event;
use crate::http::{client::build_client, executor::execute};
use crate::state::app_state::{AppState, RequestStatus};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::response_state::{ResponseBody, ResponseState};
use crate::ui::highlight::{detect_lang, highlight_text};

pub struct App {
    pub state: AppState,
    client: reqwest::Client,
    tx: UnboundedSender<Event>,
    cancel: Option<CancellationToken>,
}

impl App {
    pub fn new(tx: UnboundedSender<Event>) -> Self {
        Self {
            state: AppState {
                sidebar_visible: true,
                dirty: true,
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

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Tab => self.state.focus = self.state.focus.next(),
            KeyCode::BackTab => self.state.focus = self.state.focus.prev(),
            KeyCode::Char('i') | KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar | Focus::Editor) {
                    self.state.mode = Mode::Insert;
                    // When entering Insert on the body editor, initialize body to Json if None
                    if self.state.focus == Focus::Editor {
                        if self.state.request.body == crate::state::request_state::RequestBody::None {
                            self.state.request.body = crate::state::request_state::RequestBody::Json(String::new());
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
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.send_request();
            }
            KeyCode::Esc => self.cancel_request(),
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(resp) = &mut self.state.response {
                    resp.scroll_offset = resp.scroll_offset.saturating_sub(1);
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.state.focus == Focus::TabBar => {
                self.state.active_tab = self.state.active_tab.prev();
            }
            KeyCode::Right | KeyCode::Char('l') if self.state.focus == Focus::TabBar => {
                self.state.active_tab = self.state.active_tab.next();
            }
            KeyCode::Char('1') => self.state.focus = Focus::Sidebar,
            KeyCode::Char('2') => self.state.focus = Focus::UrlBar,
            KeyCode::Char('3') => self.state.focus = Focus::Editor,
            KeyCode::Char('4') => self.state.focus = Focus::ResponseViewer,
            _ => {}
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) {
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
            return 0; // already on first line
        }
        // Find start of the target row (current_row - 1)
        let target_row = current_row - 1;
        let rows: Vec<&str> = text.split('\n').collect();
        let target_line = rows.get(target_row).copied().unwrap_or("");
        let target_col = current_col.min(target_line.chars().count());
        // Byte offset = sum of (len+1) for rows before target_row, plus col byte offset
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
            return text.len(); // already on last line, jump to end
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
                // Pre-compute syntax highlighting once so the renderer never does it.
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

        let client = self.client.clone();
        let request = self.state.request.clone();
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
