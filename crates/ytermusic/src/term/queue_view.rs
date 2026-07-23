use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    widgets::{Block, BorderType, Borders},
    Frame,
};
use ytpapi2::YoutubeMusicVideoRef;

use crate::{structures::sound_action::SoundAction, utils::to_bidi_string};

use super::{list_selector::ListSelector, EventResponse, ManagerMessage, Screen, Screens};

pub struct QueueView {
    pub goto: Screens,
    pub items: Vec<YoutubeMusicVideoRef>,
    pub list_selector: ListSelector,
}

impl QueueView {
    pub fn new() -> Self {
        Self {
            goto: Screens::MusicPlayer,
            items: Vec::new(),
            list_selector: ListSelector::default(),
        }
    }
}

impl Screen for QueueView {
    fn on_mouse_press(&mut self, _: crossterm::event::MouseEvent, _: &Rect) -> EventResponse {
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ManagerMessage::ChangeState(self.goto).event(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_selector.scroll_up();
                EventResponse::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_selector.scroll_down();
                EventResponse::None
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                let idx = self.list_selector.get_relative_position();
                let abs = if idx < 0 { 0 } else { idx as usize };
                if abs < self.items.len() {
                    self.items.remove(abs);
                    self.list_selector.list_size = self.items.len();
                    ManagerMessage::PassTo(
                        Screens::MusicPlayer,
                        Box::new(ManagerMessage::SoundAction(
                            SoundAction::RemoveQueueIndex(abs),
                        )),
                    )
                    .event()
                } else {
                    EventResponse::None
                }
            }
            KeyCode::Char('u') => {
                let idx = self.list_selector.get_relative_position();
                if idx > 0 {
                    let i = idx as usize;
                    let j = i - 1;
                    self.items.swap(i, j);
                    self.list_selector.scroll_up();
                    ManagerMessage::PassTo(
                        Screens::MusicPlayer,
                        Box::new(ManagerMessage::SoundAction(SoundAction::MoveQueueItem(
                            i, j,
                        ))),
                    )
                    .event()
                } else {
                    EventResponse::None
                }
            }
            KeyCode::Char('n') => {
                let idx = self.list_selector.get_relative_position();
                let i = idx as usize;
                if i + 1 < self.items.len() {
                    let j = i + 1;
                    self.items.swap(i, j);
                    self.list_selector.scroll_down();
                    ManagerMessage::PassTo(
                        Screens::MusicPlayer,
                        Box::new(ManagerMessage::SoundAction(SoundAction::MoveQueueItem(
                            i, j,
                        ))),
                    )
                    .event()
                } else {
                    EventResponse::None
                }
            }
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        self.list_selector.update(self.items.len(), 0);
        self.list_selector.render(
            frame.size(),
            frame.buffer_mut(),
            |index, select, scroll| {
                let style = if select {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Green)
                } else if scroll {
                    ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
                } else {
                    ratatui::style::Style::default()
                };
                let text = self
                    .items
                    .get(index)
                    .map(|e| {
                        format!(
                            " {} {} | {} ",
                            index + 1,
                            to_bidi_string(&e.author),
                            to_bidi_string(&e.title),
                        )
                    })
                    .unwrap_or_default();
                (style, text)
            },
            " Queue ",
        );
        frame.render_widget(
            Block::default()
                .title(" Queue ")
                .borders(Borders::ALL)
                .border_type(BorderType::Plain),
            frame.size(),
        );
    }

    fn handle_global_message(&mut self, _message: ManagerMessage) -> EventResponse {
        EventResponse::None
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
