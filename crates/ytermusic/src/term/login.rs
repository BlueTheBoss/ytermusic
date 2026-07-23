use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use ytpapi2::oauth::{self, OAuthToken};

use crate::{consts::CONFIG, run_service, utils::get_project_dirs};

use super::{EventResponse, ManagerMessage, Screen, Screens};

pub struct Login {
    state: LoginState,
    goto: Screens,
}

enum LoginState {
    Init,
    WaitingCode,
    ShowCode {
        verification_url: String,
        user_code: String,
    },
    Success,
    Error(String),
}

impl Login {
    pub fn new() -> Self {
        Self {
            state: LoginState::Init,
            goto: Screens::Playlist,
        }
    }

    pub fn set_goto(&mut self, screen: Screens) {
        self.goto = screen;
    }

    pub fn goto(&self) -> Screens {
        self.goto
    }

    fn start_login_flow(&mut self) {
        self.state = LoginState::WaitingCode;
        let (tx, _rx) = flume::unbounded::<ManagerMessage>();
        run_service(async move {
            match oauth::request_device_code().await {
                Ok(code_info) => {
                    tx
                        .send(ManagerMessage::LoginCode(
                            code_info.verification_url.clone(),
                            code_info.user_code.clone(),
                        ))
                        .unwrap();
                    match oauth::poll_for_token(&code_info.device_code, code_info.interval).await {
                        Ok(token) => {
                            save_token(&token);
                            tx.send(ManagerMessage::LoginSuccess).unwrap();
                        }
                        Err(e) => {
                            tx.send(ManagerMessage::LoginError(format!("{e:?}")))
                                .unwrap();
                        }
                    }
                }
                Err(e) => {
                    tx.send(ManagerMessage::LoginError(format!("{e:?}")))
                        .unwrap();
                }
            }
        });
    }
}

fn save_token(token: &OAuthToken) {
    if let Some(dirs) = get_project_dirs() {
        let path = dirs.config_dir().join("oauth.json");
        if let Ok(json) = serde_json::to_string_pretty(token) {
            let _ = std::fs::create_dir_all(dirs.config_dir());
            let _ = std::fs::write(&path, json);
        }
    }
}

pub fn load_token() -> Option<OAuthToken> {
    let dirs = get_project_dirs()?;
    let path = dirs.config_dir().join("oauth.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

impl Screen for Login {
    fn on_mouse_press(&mut self, _: crossterm::event::MouseEvent, _: &Rect) -> EventResponse {
        EventResponse::None
    }

    fn on_key_press(&mut self, key: KeyEvent, _: &Rect) -> EventResponse {
        match key.code {
            KeyCode::Esc => ManagerMessage::ChangeState(self.goto).event(),
            _ => EventResponse::None,
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let (text, is_error) = match &self.state {
            LoginState::Init => ("Starting login...".to_string(), false),
            LoginState::WaitingCode => ("Requesting device code...".to_string(), false),
            LoginState::ShowCode {
                verification_url,
                user_code,
            } => (
                format!(
                    "Login to YouTube Music\n\n\
                     Go to: {verification_url}\n\
                     Enter code: {user_code}\n\n\
                     Waiting for authorization...\n\
                     Press Esc to cancel"
                ),
                false,
            ),
            LoginState::Success => ("Login successful! Returning...".to_string(), false),
            LoginState::Error(msg) => (format!("Login failed:\n{msg}"), true),
        };

        let style = if is_error {
            CONFIG.player.text_error_style
        } else {
            CONFIG.player.text_next_style
        };

        frame.render_widget(
            Paragraph::new(text)
                .style(style)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Login ")
                        .border_type(BorderType::Plain),
                ),
            frame.size(),
        );
    }

    fn handle_global_message(&mut self, message: ManagerMessage) -> EventResponse {
        match message {
            ManagerMessage::LoginCode(verification_url, user_code) => {
                self.state = LoginState::ShowCode {
                    verification_url,
                    user_code,
                };
                EventResponse::None
            }
            ManagerMessage::LoginSuccess => {
                self.state = LoginState::Success;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                });
                EventResponse::None
            }
            ManagerMessage::LoginError(msg) => {
                self.state = LoginState::Error(msg);
                EventResponse::None
            }
            ManagerMessage::ChangeState(_) => {
                self.state = LoginState::Init;
                EventResponse::None
            }
            _ => EventResponse::None,
        }
    }

    fn close(&mut self, _: Screens) -> EventResponse {
        self.state = LoginState::Init;
        EventResponse::None
    }

    fn open(&mut self) -> EventResponse {
        self.start_login_flow();
        EventResponse::None
    }
}
