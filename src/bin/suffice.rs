use std::env;
use std::error::Error;
use std::io;
use std::time::Duration;

use average::Mean;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use fixed_deque::Deque;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use suffice::ftms::BikeData;

use suffice::trainer::{Command, TrainerHandle};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Default)]
enum Mode {
    Power,
    #[default]
    Resistance,
}

#[derive(Debug)]
struct Stats {
    power: fixed_deque::Deque<i16>,
    cadence: fixed_deque::Deque<u8>,
    heart_rate: fixed_deque::Deque<u8>,
}

impl Stats {
    fn rolling_power(&self, n: usize) -> f64 {
        let m: Mean = self.power.iter().rev().take(n).map(|n| *n as f64).collect();
        m.mean()
    }

    fn rolling_cadence(&self, n: usize) -> f64 {
        let m: Mean = self
            .cadence
            .iter()
            .rev()
            .take(n)
            .map(|n| *n as f64)
            .collect();
        m.mean()
    }

    fn rolling_heart_rate(&self, n: usize) -> f64 {
        let m: Mean = self
            .heart_rate
            .iter()
            .rev()
            .take(n)
            .map(|n| *n as f64)
            .collect();
        m.mean()
    }
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            power: Deque::new(30),
            cadence: Deque::new(30),
            heart_rate: Deque::new(30),
        }
    }
}

#[derive(Debug, Default)]
pub struct App {
    resistance: i16,
    power: i16,
    exit: bool,
    mode: Mode,
    to_trainer: Option<mpsc::UnboundedSender<Command>>,
    from_trainer: Option<broadcast::Receiver<BikeData>>,
    stats: Stats,
}

impl App {
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

    fn handle_events(&mut self) -> io::Result<()> {
        if let Ok(e) = event::poll(Duration::from_millis(100))
            && e
        {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }

        if let Ok(data) = self.from_trainer.as_mut().expect("").try_recv() {
            eprintln!("{:?}", data);
            self.stats.power.push_back(data.power.unwrap());
            self.stats.cadence.push_back(data.cadence.unwrap());
            self.stats.heart_rate.push_back(data.heart_rate.unwrap());
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
            _ => {}
        }

        match self.mode {
            Mode::Power => {
                let _ = self.to_trainer.as_ref().expect("").send(Command::Reset);
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
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn more(&mut self) {
        match self.mode {
            Mode::Power => self.power = (self.power + 10).clamp(0, 1000),
            Mode::Resistance => self.resistance = (self.resistance + 1).clamp(0, 100),
        }
    }

    fn less(&mut self) {
        match self.mode {
            Mode::Power => self.power = (self.power - 10).clamp(0, 1000),
            Mode::Resistance => self.resistance = (self.resistance - 1).clamp(0, 100),
        }
    }

    fn change_mode(&mut self, _change: i8) {
        match self.mode {
            Mode::Power => self.mode = Mode::Resistance,
            Mode::Resistance => self.mode = Mode::Power,
        }
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
                format!("{:?}", power_3s).yellow(),
            ]),
            Line::from(vec![
                "3s Cadence: ".into(),
                format!("{:?}", cadence_3s).yellow(),
            ]),
            Line::from(vec![
                "3s Heart Rate: ".into(),
                format!("{:?}", heart_3s).yellow(),
            ]),
        ]);

        Paragraph::new(counter)
            .centered()
            .block(block)
            .render(area, buf)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    let trainer = TrainerHandle::find(target.clone()).await;
    if trainer.is_none() {
        return Err(format!("{} not found", target).into());
    }
    let trainer = trainer.unwrap();
    trainer.connect().await;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);
    let trainer_copy = trainer.trainer.clone();
    tokio::spawn(async move {
        TrainerHandle::run(trainer_copy, cmd_rx, data_tx).await;
    });

    let mut term = ratatui::init();
    let mut app = App::default();
    let res = app
        .run(&mut term, cmd_tx.clone(), data_rx)
        .await
    // .inspect_err(|e| tracing::error!("Error in main event loop: {}", e))
    ;
    ratatui::restore();
    Ok(res?)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use ratatui::style::Style;
//
//     #[test]
//     fn render() {
//         let app = App::default();
//         let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));
//
//         app.render(buf.area, &mut buf);
//
//         let mut expected = Buffer::with_lines(vec![
//             "┏━━━━━━━━━━━━━━ -= It Suffices =- ━━━━━━━━━━━━━━━┓",
//             "┃                  Resistance: 0                 ┃",
//             "┃                                                ┃",
//             "┗━━━━━━━━ More <Up> Less <Down> Quit <Q> ━━━━━━━━┛",
//         ]);
//         let title_style = Style::new().bold();
//         let counter_style = Style::new().yellow();
//         let key_style = Style::new().blue().bold();
//         expected.set_style(Rect::new(15, 0, 19, 1), title_style);
//         expected.set_style(Rect::new(31, 1, 1, 1), counter_style);
//         expected.set_style(Rect::new(15, 3, 4, 1), key_style);
//         expected.set_style(Rect::new(25, 3, 6, 1), key_style);
//         expected.set_style(Rect::new(37, 3, 4, 1), key_style);
//
//         assert_eq!(buf, expected);
//     }
// }
