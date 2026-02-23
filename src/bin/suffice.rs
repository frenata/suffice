use std::env;
use std::error::Error;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use tokio_stream::StreamExt;

use suffice::trainer::{Command, TrainerHandle};
use tokio::sync::mpsc;

#[derive(Debug, Default)]
enum Mode {
    Power,
    #[default]
    Resistance,
}

#[derive(Debug, Default)]
pub struct App {
    resistance: i16,
    power: i16,
    exit: bool,
    mode: Mode,
    to_trainer: Option<mpsc::UnboundedSender<Command>>,
}

impl App {
    pub async fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        mut trainer: TrainerHandle,
        tx: mpsc::UnboundedSender<Command>,
    ) -> io::Result<()> {
        self.to_trainer = Some(tx);
        self.to_trainer.as_ref().expect("").send(Command::Reset);
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
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
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
                self.to_trainer.as_ref().expect("").send(Command::Reset);
                self.to_trainer
                    .as_ref()
                    .expect("")
                    .send(Command::Power(self.power));
            }
            Mode::Resistance => {
                self.to_trainer.as_ref().expect("").send(Command::Reset);
                self.to_trainer
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
        let counter = Text::from(vec![Line::from(match self.mode {
            Mode::Power => vec!["Power: ".into(), self.power.to_string().yellow()],
            Mode::Resistance => vec!["Resistance: ".into(), self.resistance.to_string().yellow()],
        })]);

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

    // let backend = CrosstermBackend::new(io::stdout());
    // let mut terminal = Terminal::new(backend)?;
    //
    // let mut app = App::default();
    // app.init_terminal()?;
    let (tx, mut rx) = mpsc::unbounded_channel::<Command>();
    let inner = trainer.trainer.clone();
    tokio::spawn(async move {
        TrainerHandle::run(inner, rx).await;
    });

    // let mut event_stream = event::EventStream::new();
    // tokio::spawn(async move {
    //     loop {
    //         tokio::select! {
    //             maybe_ev = event_stream.next() => {
    //                 let ev = match maybe_ev {
    //                     None => break,
    //                     Some(Err(_)) => break,
    //                     Some(Ok(e)) => e,
    //                 };
    //                 // if tx.send(AppEvent::Input(ev)).await.is_err() {
    //                 //     break;
    //                 // }
    //             }
    //         }
    //     }
    // });

    let mut term = ratatui::init();
    let mut app = App::default();
    // ratatui::run(|terminal| app.run(terminal));
    // Ok(())
    let res = app
        .run(&mut term, trainer, tx.clone())
        .await
    //     // .inspect_err(|e| tracing::error!("Error in main event loop: {}", e))
    ;
    ratatui::restore();
    // drop(app);
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
