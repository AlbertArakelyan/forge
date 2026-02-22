use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::event::Event;
use crate::http::{client::build_client, executor::execute};
use crate::state::app_state::{AppState, RequestStatus};
use crate::state::focus::Focus;
use crate::state::mode::Mode;
use crate::state::response_state::ResponseState;

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
                ..Default::default()
            },
            client: build_client(),
            tx,
            cancel: None,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => match self.state.mode {
                Mode::Normal => self.handle_normal_key(key),
                Mode::Insert => self.handle_insert_key(key),
                Mode::Command | Mode::Visual => {}
            },
            Event::Key(_) => {}
            Event::Response(result) => self.handle_response(result),
            Event::Tick => self.handle_tick(),
            Event::Resize(_, _) | Event::Mouse(_) => {}
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Tab => self.state.focus = self.state.focus.next(),
            KeyCode::BackTab => self.state.focus = self.state.focus.prev(),
            KeyCode::Char('i') | KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.mode = Mode::Insert;
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
                if matches!(self.state.focus, Focus::ResponseViewer) {
                    if let Some(resp) = &mut self.state.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_add(1);
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if matches!(self.state.focus, Focus::ResponseViewer) {
                    if let Some(resp) = &mut self.state.response {
                        resp.scroll_offset = resp.scroll_offset.saturating_sub(1);
                    }
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
        match key.code {
            KeyCode::Esc => self.state.mode = Mode::Normal,
            KeyCode::Enter => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.mode = Mode::Normal;
                    self.send_request();
                }
            }
            KeyCode::Char(c) => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    self.state.request.url.insert(cursor, c);
                    self.state.request.url_cursor += c.len_utf8();
                }
            }
            KeyCode::Backspace => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    if cursor > 0 {
                        let prev = self.prev_char_boundary(cursor);
                        self.state.request.url.drain(prev..cursor);
                        self.state.request.url_cursor = prev;
                    }
                }
            }
            KeyCode::Delete => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    if cursor < self.state.request.url.len() {
                        let next = self.next_char_boundary(cursor);
                        self.state.request.url.drain(cursor..next);
                    }
                }
            }
            KeyCode::Left => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    self.state.request.url_cursor = self.prev_char_boundary(cursor);
                }
            }
            KeyCode::Right => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    let cursor = self.state.request.url_cursor;
                    self.state.request.url_cursor = self.next_char_boundary(cursor);
                }
            }
            KeyCode::Home => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.request.url_cursor = 0;
                }
            }
            KeyCode::End => {
                if matches!(self.state.focus, Focus::UrlBar) {
                    self.state.request.url_cursor = self.state.request.url.len();
                }
            }
            _ => {}
        }
    }

    fn prev_char_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let url = &self.state.request.url;
        let mut p = pos - 1;
        while p > 0 && !url.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn next_char_boundary(&self, pos: usize) -> usize {
        let url = &self.state.request.url;
        if pos >= url.len() {
            return url.len();
        }
        let mut p = pos + 1;
        while p < url.len() && !url.is_char_boundary(p) {
            p += 1;
        }
        p
    }

    fn handle_response(&mut self, result: Result<ResponseState, AppError>) {
        self.cancel = None;
        match result {
            Ok(response) => {
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
