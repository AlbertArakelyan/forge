use crossterm::event::{KeyEvent, MouseEvent};
use crate::state::response_state::ResponseState;
use crate::error::AppError;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Response(Result<ResponseState, AppError>),
    Resize(u16, u16),
}
