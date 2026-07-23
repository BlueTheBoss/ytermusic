use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{
    consts::CONFIG,
    lyrics::lrclib::LyricLine,
    utils::to_bidi_string,
};

use super::{EventResponse, ManagerMessage, Screen, Screens};

pub struct LyricsView {
    pub goto: Screens,
    pub lyrics: Vec<LyricLine>,
    pub current_line: usize,
    pub current_elapsed: f64,
}

impl LyricsView {
    pub fn new() -> Self {
        Self {
            goto: Screens::MusicPlayer,
            lyrics: Vec::new(),
            current_line: 0,
            current_elapsed: 0.0,
        }
    }

    fn find_current_line(&mut self, elapsed_secs: f64) {
        // Binary search since lyrics are sorted by timestamp
        let mut low = 0;
        let mut high = self.lyrics.len();
        while low < high {
            let mid = (low + high) / 2;
            if self.lyrics[mid].timestamp <= elapsed_secs {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        // low is the first index where timestamp > elapsed_secs
        // So the current line is low - 1 (or 0 if low == 0)
        self.current_line = low.saturating_sub(1);
    }
}

impl Screen for LyricsView {
    fn on_mouse_press(&mut self, _: crossterm::event::MouseEvent, _: &Rect) -> EventResponse {
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(self.goto).event(),
            KeyCode::Char('q') => ManagerMessage::ChangeState(self.goto).event(),
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        self.find_current_line(self.current_elapsed);

        let height = (frame.size().height.saturating_sub(4) as usize).max(1);
        let start = self.current_line.saturating_sub(height / 2);
        let end = start + height;

        let lines: Vec<Line> = self.lyrics[start..end.min(self.lyrics.len())]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let is_current = start + i == self.current_line;
                let style = if is_current {
                    CONFIG.player.text_playing_style
                } else {
                    CONFIG.player.text_next_style
                };
                Line::from(Span::styled(to_bidi_string(&line.text), style))
            })
            .collect();

        let lyrics = if lines.is_empty() {
            Paragraph::new(" No lyrics found ")
                .style(CONFIG.player.text_searching_style)
                .alignment(Alignment::Center)
        } else {
            Paragraph::new(lines)
                .style(CONFIG.player.text_next_style)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
        };

        frame.render_widget(
            lyrics.block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Lyrics ")
                    .border_type(BorderType::Plain),
            ),
            frame.size(),
        );
    }

    fn handle_global_message(&mut self, message: ManagerMessage) -> EventResponse {
        match message {
            ManagerMessage::SetLyrics(lyrics) => {
                self.lyrics = lyrics;
                self.current_line = 0;
                EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        EventResponse::None
    }
}
