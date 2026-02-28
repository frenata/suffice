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
use tokio::sync::{broadcast, mpsc};

use suffice::bluetooth::BluetoothDevice;
use suffice::ftms::BikeData;
use suffice::trainer::{Command, Trainer};

#[derive(Debug, Default)]
enum Mode {
    Power,
    #[default]
    Resistance,
}

#[derive(Debug)]
struct Stats {
    power: fixed_deque::Deque<u16>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    let device = BluetoothDevice::new(target.clone()).await;
    if device.is_none() {
        return Err(format!("{} not found", target).into());
    }
    let device = device.unwrap();
    let mut trainer = Trainer::<BluetoothDevice>::new(device).await;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);

    tokio::spawn(async move {
        let _ = trainer.run(cmd_rx, data_tx).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render_no_data() {
        let mut app = App::default();
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
        let mut app = App::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 10));

        let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);
        app.from_trainer = Some(data_rx);

        let _ = data_tx.send(BikeData {
            power: Some(100),
            cadence: Some(50),
            heart_rate: Some(93),
            resistance: None,
            speed: Some(10),
        });

        let _ = data_tx.send(BikeData {
            power: Some(120),
            cadence: Some(52),
            heart_rate: Some(90),
            resistance: None,
            speed: Some(10),
        });

        let _ = data_tx.send(BikeData {
            power: Some(125),
            cadence: Some(51),
            heart_rate: Some(91),
            resistance: None,
            speed: Some(10),
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
