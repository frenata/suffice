use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use derivative::Derivative;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Widget,
};
use tokio::sync::{broadcast, mpsc};
use tracing::{Level, event as ev, instrument};

use suffice::ftms::BikeData;
use suffice::trainer::Command;

use crate::state::*;
use crate::widgets::{Help, border, chart, level, rolling, totals};

#[derive(Debug, Default)]
enum View {
    #[default]
    Rolling,
    Totals,
    Chart,
}

#[derive(Debug, Derivative, Default)]
/// The main application data bundle
pub struct App {
    exit: bool,
    help: bool,

    to_trainer: Option<mpsc::UnboundedSender<Command>>,
    from_trainer: Option<broadcast::Receiver<BikeData>>,

    stats: Stats,
    ride: RideState,
    view: View,
}

impl App {
    /// The run loop for ratatui
    pub async fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        cmd_tx: mpsc::UnboundedSender<Command>,
        data_rx: broadcast::Receiver<BikeData>,
    ) -> io::Result<()> {
        self.to_trainer = Some(cmd_tx);
        self.from_trainer = Some(data_rx);
        let _ = self.to_trainer.as_ref().expect("").send(Command::Reset);
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    #[instrument(skip(self))]
    fn handle_events(&mut self) -> io::Result<()> {
        if let Ok(data) = self.from_trainer.as_mut().expect("").try_recv() {
            if let Some(stat) = data.power {
                self.stats.power.add(stat.into());
            }
            if let Some(stat) = data.cadence {
                self.stats.cadence.add(stat.into());
            }
            if let Some(stat) = data.heart_rate {
                self.stats.heart_rate.add(stat.into());
            }
            if let Some(stat) = data.speed {
                self.stats.speed.add(stat.into());
            }
            if let Some(stat) = data.distance {
                self.stats.distance.add(stat);
            }
        }

        if let Ok(e) = event::poll(Duration::from_millis(500))
            && e
        {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        } else {
            if self.ride.mode_dirty {
                let _ = self
                    .to_trainer
                    .as_ref()
                    .expect("send should always work")
                    .send(Command::Reset);
                ev!(Level::INFO, "sent reset command");
                self.ride.mode_dirty = false;
            }

            if self.ride.level_dirty {
                match self.ride.mode {
                    Mode::Power => {
                        let _ = self
                            .to_trainer
                            .as_ref()
                            .expect("send should always work")
                            .send(Command::Reset);
                        let _ = self
                            .to_trainer
                            .as_ref()
                            .expect("")
                            .send(Command::Power(self.ride.power));
                    }
                    Mode::Resistance => {
                        let _ = self.to_trainer.as_ref().expect("").send(Command::Reset);
                        let _ = self
                            .to_trainer
                            .as_ref()
                            .expect("")
                            .send(Command::Resist((self.ride.resistance as u8).into()));
                    }
                }
                ev!(
                    Level::INFO,
                    "sent level change command for {:?} mode",
                    self.ride.mode
                );
                self.ride.level_dirty = false;
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('?') => self.help = !self.help,
            KeyCode::Up => self.more(),
            KeyCode::Down => self.less(),
            KeyCode::Right => self.change_mode(1),
            KeyCode::Left => self.change_mode(-1),
            KeyCode::Tab => match self.view {
                View::Rolling => self.view = View::Totals,
                View::Totals => self.view = View::Chart,
                View::Chart => self.view = View::Rolling,
            },
            KeyCode::Char('r') => {
                let _ = self
                    .to_trainer
                    .as_ref()
                    .expect("")
                    .send(Command::ToggleRecording);
                self.ride.is_recording = !self.ride.is_recording;
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn more(&mut self) {
        match self.ride.mode {
            Mode::Power => self.ride.power = (self.ride.power + 10).clamp(0, 1000),
            Mode::Resistance => self.ride.resistance = (self.ride.resistance + 1).clamp(0, 100),
        }
        self.ride.level_dirty = true;
    }

    fn less(&mut self) {
        match self.ride.mode {
            Mode::Power => self.ride.power = (self.ride.power - 10).clamp(0, 1000),
            Mode::Resistance => self.ride.resistance = (self.ride.resistance - 1).clamp(0, 100),
        }
        self.ride.level_dirty = true;
    }

    fn change_mode(&mut self, _change: i8) {
        match self.ride.mode {
            Mode::Power => self.ride.mode = Mode::Resistance,
            Mode::Resistance => self.ride.mode = Mode::Power,
        }
        self.ride.mode_dirty = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outer_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(3),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Fill(99),
            ])
            .split(area);

        level(&self.ride).render(outer_layout[0], buf);
        match self.view {
            View::Rolling => rolling(&self.stats).render(outer_layout[1], buf),
            View::Totals => totals(&self.stats).render(outer_layout[1], buf),
            View::Chart => {
                chart(
                    "power",
                    Style::default().magenta(),
                    &mut self.stats.power.data()[..],
                )
                .render(outer_layout[1], buf);

                chart(
                    "cadence",
                    Style::default().cyan(),
                    &mut self.stats.cadence.data()[..],
                )
                .render(outer_layout[2], buf);
            }
        }

        if self.help {
            Help::default().render(area, buf);
        }
        border(&self.ride).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render_no_data() {
        let mut app = App {
            ride: RideState {
                level_dirty: false,
                mode_dirty: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));

        let (_data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "╭────────────── -= It Suffices =- ───────────────╮",
            "│                  Resistance: 0                 │",
            "│                                                │",
            "│                  3s Power: NaN                 │",
            "│                 3s Cadence: NaN                │",
            "│               3s Heart Rate: NaN               │",
            "│                  3s Speed: NaN                 │",
            "│                                                │",
            "│                                                │",
            "╰───────── Record <R> Help <?> Quit <Q> ─────────╯",
        ]);

        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(29, 3, 3, 1), counter_style);
        expected.set_style(Rect::new(30, 4, 3, 1), counter_style);
        expected.set_style(Rect::new(31, 5, 3, 1), counter_style);
        expected.set_style(Rect::new(29, 6, 3, 1), counter_style);

        expected.set_style(Rect::new(18, 9, 4, 1), key_style);
        expected.set_style(Rect::new(27, 9, 4, 1), key_style);
        expected.set_style(Rect::new(36, 9, 4, 1), key_style);

        assert_eq!(buf, expected);
    }

    #[test]
    fn render_with_data() {
        let mut app = App {
            ride: RideState {
                level_dirty: false,
                mode_dirty: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));

        let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);

        let _ = data_tx.send(BikeData {
            power: Some(100),
            cadence: Some(50),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(120),
            cadence: Some(52),
            heart_rate: Some(90),
            resistance: None,
            speed: Some(3000),
            distance: Some(19),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(125),
            cadence: Some(51),
            heart_rate: Some(91),
            resistance: None,
            speed: Some(3205),
            distance: Some(11),
            ..Default::default()
        });

        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "╭────────────── -= It Suffices =- ───────────────╮",
            "│                  Resistance: 0                 │",
            "│                                                │",
            "│                3s Power: 115.00                │",
            "│                3s Cadence: 51.00               │",
            "│              3s Heart Rate: 91.33              │",
            "│                 3s Speed: 20.72                │",
            "│                                                │",
            "│                                                │",
            "╰───────── Record <R> Help <?> Quit <Q> ─────────╯",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(27, 3, 6, 1), counter_style);
        expected.set_style(Rect::new(29, 4, 5, 1), counter_style);
        expected.set_style(Rect::new(30, 5, 5, 1), counter_style);
        expected.set_style(Rect::new(28, 6, 5, 1), counter_style);

        expected.set_style(Rect::new(18, 9, 4, 1), key_style);
        expected.set_style(Rect::new(27, 9, 4, 1), key_style);
        expected.set_style(Rect::new(36, 9, 4, 1), key_style);

        assert_eq!(buf, expected);
    }

    #[test]
    fn render_totals_no_data() {
        let mut app = App {
            ride: RideState {
                level_dirty: false,
                mode_dirty: false,
                ..Default::default()
            },
            view: View::Totals,
            ..Default::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));

        let (_data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "╭────────────── -= It Suffices =- ───────────────╮",
            "│                  Resistance: 0                 │",
            "│                                                │",
            "│               Total Work: 0.0 kJ               │",
            "│            Total Distance: 0.000 km            │",
            "│                                                │",
            "│                                                │",
            "│                                                │",
            "│                                                │",
            "╰───────── Record <R> Help <?> Quit <Q> ─────────╯",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(28, 3, 6, 1), counter_style);
        expected.set_style(Rect::new(29, 4, 8, 1), counter_style);

        expected.set_style(Rect::new(18, 9, 4, 1), key_style);
        expected.set_style(Rect::new(27, 9, 4, 1), key_style);
        expected.set_style(Rect::new(36, 9, 4, 1), key_style);

        assert_eq!(buf, expected);
    }

    #[test]
    fn render_chart_no_data() {
        let mut app = App {
            ride: RideState {
                level_dirty: false,
                mode_dirty: false,
                ..Default::default()
            },
            view: View::Chart,
            ..Default::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 13));

        let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);

        let _ = data_tx.send(BikeData {
            power: Some(100),
            cadence: Some(50),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(100),
            cadence: Some(50),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(100),
            cadence: Some(50),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(80),
            cadence: Some(70),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(90),
            cadence: Some(75),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
            distance: Some(14),
            ..Default::default()
        });

        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "╭────────────── -= It Suffices =- ───────────────╮",
            "│                  Resistance: 0                 │",
            "│                                                │",
            "│                                                │",
            "│100│               ⡠⠔⠊⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠁        │",
            "│   │⠤⣀⣀        ⢀⡠⠔⠉                             │",
            "│80 │   ⠉⠑⠒⠤⠤⣀⠤⠒⠁                                │",
            "│                                                │",
            "│                                                │",
            "│75│⠉⠉⠑⠒⠒⠒⠒⠤⠤⠤⣀                                  │",
            "│  │           ⠉⠒⠤⣀                              │",
            "│50│               ⠉⠒⠤⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀         │",
            "╰───────── Record <R> Help <?> Quit <Q> ─────────╯",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        let power_style = Style::new().magenta();
        let cadence_style = Style::new().cyan();

        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(20, 4, 21, 1), power_style);
        expected.set_style(Rect::new(5, 5, 3, 1), power_style);
        expected.set_style(Rect::new(16, 5, 4, 1), power_style);
        expected.set_style(Rect::new(8, 6, 9, 1), power_style);
        expected.set_style(Rect::new(4, 9, 11, 1), cadence_style);
        expected.set_style(Rect::new(15, 10, 4, 1), cadence_style);
        expected.set_style(Rect::new(19, 11, 21, 1), cadence_style);

        expected.set_style(Rect::new(18, 12, 4, 1), key_style);
        expected.set_style(Rect::new(27, 12, 4, 1), key_style);
        expected.set_style(Rect::new(36, 12, 4, 1), key_style);

        assert_eq!(buf, expected);
    }
}
