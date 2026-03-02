use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use crossterm::event::KeyCode;
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatable::table::table;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};
use russh::keys::ssh_key::PublicKey;
use russh::keys::ssh_key::rand_core::OsRng;
use russh::server::*;
use russh::{Channel, ChannelId, Pty};
use tokio::sync::mpsc::{Sender, UnboundedSender, unbounded_channel};
use wraptatui::widgets::state::state_with_default;
use wraptatui::{Pass, PassReturn, draw, handle_key_event, init};

struct TerminalHandle {
    sender: UnboundedSender<Vec<u8>>,
    // The sink collects the data which is finally sent to sender.
    sink: Vec<u8>,
}

impl TerminalHandle {
    async fn start(handle: Handle, channel_id: ChannelId) -> (Self, tokio::task::JoinHandle<()>) {
        let (sender, mut receiver) = unbounded_channel::<Vec<u8>>();
        let join_handle = tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                let result = handle.data(channel_id, data.into()).await;
                if result.is_err() {
                    eprintln!("Failed to send data: {result:?}");
                }
            }
        });

        (
            Self {
                sender,
                sink: Vec::new(),
            },
            join_handle,
        )
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.sender.send(self.sink.clone());
        if result.is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                result.unwrap_err(),
            ));
        }

        self.sink.clear();
        Ok(())
    }
}

#[derive(Clone)]
struct AppServer;

impl AppServer {
    pub async fn run(&mut self, private_key: russh::keys::PrivateKey) -> Result<(), anyhow::Error> {
        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![private_key],
            nodelay: true,
            ..Default::default()
        };

        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}

impl Server for AppServer {
    type Handler = Client;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Client {
        Client {
            channels: HashMap::new(),
        }
    }
}

struct Client {
    channels: HashMap<ChannelId, SessionChannel>,
}

enum SessionChannel {
    Opened,
    PtyAllocated {
        size: (u16, u16),
    },
    Running {
        events: Sender<crossterm::event::Event>,
    },
}

impl Handler for Client {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        self.channels.insert(channel.id(), SessionChannel::Opened);
        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        // TODO: buffering? the unwrap is likely incorrect.
        let event =
            terminput_crossterm::to_crossterm(terminput::Event::parse_from(data)?.unwrap())?;

        match &self.channels[&channel] {
            SessionChannel::Running { events } => events.send(event).await?,
            _ => {}
        }

        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let session_channel = self.channels.get_mut(&channel).unwrap();

        match session_channel {
            &mut SessionChannel::PtyAllocated { size } => {
                let (tx, mut rx) = tokio::sync::mpsc::channel(1);

                *session_channel = SessionChannel::Running { events: tx };

                let handle = session.handle();

                tokio::spawn(async move {
                    let (mut terminal_handle, writer_task_handle) =
                        TerminalHandle::start(handle.clone(), channel).await;

                    execute!(terminal_handle, EnterAlternateScreen)?;

                    let backend = CrosstermBackend::new(terminal_handle);

                    // the correct viewport area will be set when the client request a pty
                    let options = TerminalOptions {
                        viewport: Viewport::Fixed(Rect {
                            x: 0,
                            y: 0,
                            width: size.0,
                            height: size.1,
                        }),
                    };

                    let mut terminal = Terminal::with_options(backend, options)?;

                    fn widget<'a>(p: Pass<'a>) -> PassReturn<'a, impl Sized + use<>> {
                        state_with_default(p, |p, data: &mut ratatable::database_views::State| {
                            table(p, data, || Box::new(ratatable::database_views::MainView {}))
                        })
                    }

                    let mut state = init(&mut widget);

                    while let Some(event) = {
                        terminal.draw(|frame| {
                            let cursor_position = draw(
                                &mut widget,
                                &mut state,
                                wraptatui::Focus::Focused,
                                frame.area(),
                                frame.buffer_mut(),
                            );

                            if let Some(position) = cursor_position {
                                frame.set_cursor_position(position);
                            };
                        })?;

                        rx.recv().await
                    } {
                        match event {
                            crossterm::event::Event::Key(key_event) => {
                                let handled = handle_key_event(&mut widget, &mut state, key_event);

                                if !handled && key_event.code == KeyCode::Char('q') {
                                    break;
                                }
                            }
                            crossterm::event::Event::Resize(width, height) => {
                                terminal.resize(Rect {
                                    x: 0,
                                    y: 0,
                                    width,
                                    height,
                                })?
                            }
                            _ => (),
                        }
                    }

                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                    drop(terminal);

                    writer_task_handle.await?;

                    handle.close(channel).await.ok();

                    Result::<(), anyhow::Error>::Ok(())
                });
            }
            _ => session.channel_failure(channel)?,
        }

        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        match self.channels.get_mut(&channel).unwrap() {
            SessionChannel::Opened => {}
            SessionChannel::PtyAllocated { size } => {
                *size = (col_width as u16, row_height as u16);
            }
            SessionChannel::Running { events } => {
                events
                    .send(crossterm::event::Event::Resize(
                        col_width as u16,
                        row_height as u16,
                    ))
                    .await?
            }
        }

        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let session_channel = self.channels.get_mut(&channel).unwrap();

        match session_channel {
            SessionChannel::Opened => {
                *session_channel = SessionChannel::PtyAllocated {
                    size: (col_width as u16, row_height as u16),
                };
                session.channel_success(channel)?;
            }
            _ => {
                session.channel_failure(channel)?;
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut server = AppServer;

    let private_key = std::env::var("RATATABLE_SSH_PRIVATE_KEY")
        .map(Some)
        .or_else(|e| match e {
            std::env::VarError::NotPresent => Ok(None),
            std::env::VarError::NotUnicode(_) => Err(e),
        })?
        .map(|key| {
            russh::keys::PrivateKey::from_openssh(key).context("Failed parsing SSH private key")
        })
        .transpose()?
        .map_or_else(
            || russh::keys::PrivateKey::random(&mut OsRng, russh::keys::Algorithm::Ed25519),
            Ok,
        )?;

    server.run(private_key).await?;

    Ok(())
}
