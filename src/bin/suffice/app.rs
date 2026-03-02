use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use derivative::Derivative;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{Level, event as ev, instrument};

use suffice::ftms::BikeData;
use suffice::trainer::Command;

use crate::stats::Stats;

#[derive(Debug, Default)]
enum Mode {
    Power,
    #[default]
    Resistance,
}

#[derive(Debug, Derivative)]
#[derivative(Default)]
/// The main application data bundle
pub struct App {
    resistance: i16,
    power: i16,
    exit: bool,
    mode: Mode,
    #[derivative(Default(value = "true"))]
    mode_dirty: bool,
    #[derivative(Default(value = "true"))]
    level_dirty: bool,
    to_trainer: Option<mpsc::UnboundedSender<Command>>,
    from_trainer: Option<broadcast::Receiver<BikeData>>,
    stats: Stats,
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
                self.stats.power.push_back(stat);
            }
            if let Some(stat) = data.cadence {
                self.stats.cadence.push_back(stat);
            }
            if let Some(stat) = data.heart_rate {
                self.stats.heart_rate.push_back(stat);
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
            // ev!(Level::INFO, "no event happened");

            if self.mode_dirty {
                let _ = self
                    .to_trainer
                    .as_ref()
                    .expect("send should always work")
                    .send(Command::Reset);
                ev!(Level::INFO, "sent reset command");
                self.mode_dirty = false;
            }

            if self.level_dirty {
                match self.mode {
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
                            .send(Command::Power(self.power));
                    }
                    Mode::Resistance => {
                        let _ = self.to_trainer.as_ref().expect("").send(Command::Reset);
                        let _ = self
                            .to_trainer
                            .as_ref()
                            .expect("")
                            .send(Command::Resist((self.resistance as u8).into()));
                    }
                }
                ev!(
                    Level::INFO,
                    "sent level change command for {:?} mode",
                    self.mode
                );
                self.level_dirty = false;
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.more(),
            KeyCode::Down => self.less(),
            KeyCode::Right => self.change_mode(1),
            KeyCode::Left => self.change_mode(-1),
            KeyCode::Char('r') => {
                let _ = self
                    .to_trainer
                    .as_ref()
                    .expect("")
                    .send(Command::ToggleRecording);
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn more(&mut self) {
        match self.mode {
            Mode::Power => self.power = (self.power + 10).clamp(0, 1000),
            Mode::Resistance => self.resistance = (self.resistance + 1).clamp(0, 100),
        }
        self.level_dirty = true;
    }

    fn less(&mut self) {
        match self.mode {
            Mode::Power => self.power = (self.power - 10).clamp(0, 1000),
            Mode::Resistance => self.resistance = (self.resistance - 1).clamp(0, 100),
        }
        self.level_dirty = true;
    }

    fn change_mode(&mut self, _change: i8) {
        match self.mode {
            Mode::Power => self.mode = Mode::Resistance,
            Mode::Resistance => self.mode = Mode::Power,
        }
        self.mode_dirty = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" -= It Suffices =- ".bold());
        let instructions = Line::from(vec![
            " More ".into(),
            "<Up>".blue().bold(),
            " Less ".into(),
            "<Down>".blue().bold(),
            " Record ".into(),
            "<R>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let power_3s = self.stats.rolling_power(3);
        let cadence_3s = self.stats.rolling_cadence(3);
        let heart_3s = self.stats.rolling_heart_rate(3);

        let counter = Text::from(vec![
            Line::from(match self.mode {
                Mode::Power => vec!["Power: ".into(), self.power.to_string().yellow()],
                Mode::Resistance => {
                    vec!["Resistance: ".into(), self.resistance.to_string().yellow()]
                }
            }),
            Line::from(vec![]),
            Line::from(vec![
                "3s Power: ".into(),
                format!("{:.2}", power_3s).yellow(),
            ]),
            Line::from(vec![
                "3s Cadence: ".into(),
                format!("{:.2}", cadence_3s).yellow(),
            ]),
            Line::from(vec![
                "3s Heart Rate: ".into(),
                format!("{:.2}", heart_3s).yellow(),
            ]),
        ]);

        Paragraph::new(counter)
            .centered()
            .block(block)
            .render(area, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render_no_data() {
        let mut app = App {
            level_dirty: false,
            mode_dirty: false,
            ..Default::default()
        };
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));

        let (_data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┏━━━━━━━━━━━━━━ -= It Suffices =- ━━━━━━━━━━━━━━━┓",
            "┃                  Resistance: 0                 ┃",
            "┃                                                ┃",
            "┃                  3s Power: NaN                 ┃",
            "┃                 3s Cadence: NaN                ┃",
            "┃               3s Heart Rate: NaN               ┃",
            "┃                                                ┃",
            "┃                                                ┃",
            "┃                                                ┃",
            "┗━━ More <Up> Less <Down> Record <R> Quit <Q> ━━━┛",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(29, 3, 3, 1), counter_style);
        expected.set_style(Rect::new(30, 4, 3, 1), counter_style);
        expected.set_style(Rect::new(31, 5, 3, 1), counter_style);

        expected.set_style(Rect::new(9, 9, 4, 1), key_style);
        expected.set_style(Rect::new(19, 9, 6, 1), key_style);
        expected.set_style(Rect::new(33, 9, 3, 1), key_style);
        expected.set_style(Rect::new(42, 9, 4, 1), key_style);

        assert_eq!(buf, expected);
    }

    #[test]
    fn render_with_data() {
        let mut app = App {
            level_dirty: false,
            mode_dirty: false,
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
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(120),
            cadence: Some(52),
            heart_rate: Some(90),
            resistance: None,
            speed: Some(10),
            ..Default::default()
        });

        let _ = data_tx.send(BikeData {
            power: Some(125),
            cadence: Some(51),
            heart_rate: Some(91),
            resistance: None,
            speed: Some(10),
            ..Default::default()
        });

        let _ = app.handle_events();
        let _ = app.handle_events();
        let _ = app.handle_events();
        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┏━━━━━━━━━━━━━━ -= It Suffices =- ━━━━━━━━━━━━━━━┓",
            "┃                  Resistance: 0                 ┃",
            "┃                                                ┃",
            "┃                3s Power: 115.00                ┃",
            "┃                3s Cadence: 51.00               ┃",
            "┃              3s Heart Rate: 91.33              ┃",
            "┃                                                ┃",
            "┃                                                ┃",
            "┃                                                ┃",
            "┗━━ More <Up> Less <Down> Record <R> Quit <Q> ━━━┛",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);

        expected.set_style(Rect::new(27, 3, 6, 1), counter_style);
        expected.set_style(Rect::new(29, 4, 5, 1), counter_style);
        expected.set_style(Rect::new(30, 5, 5, 1), counter_style);

        expected.set_style(Rect::new(9, 9, 4, 1), key_style);
        expected.set_style(Rect::new(19, 9, 6, 1), key_style);
        expected.set_style(Rect::new(33, 9, 3, 1), key_style);
        expected.set_style(Rect::new(42, 9, 4, 1), key_style);

        assert_eq!(buf, expected);
    }
}
