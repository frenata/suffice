use std::env;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use suffice::trainer::{Command, TrainerHandle};

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
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let args: Vec<String> = env::args().collect();
        let target = args[1].clone();
        // if let Some(trainer) = TrainerHandle::find(target.clone()).await {
        //     eprintln!("{:?}\n", trainer);
        //     trainer.connect().await;
        // } else {
        //     // return Err(format!("{} not found", target).into());
        //     // return Err(format!("{} not found", target).into());
        //     return Ok(());
        // }

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

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render() {
        let app = App::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));

        app.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "┏━━━━━━━━━━━━━━ -= It Suffices =- ━━━━━━━━━━━━━━━┓",
            "┃                  Resistance: 0                 ┃",
            "┃                                                ┃",
            "┗━━━━━━━━ More <Up> Less <Down> Quit <Q> ━━━━━━━━┛",
        ]);
        let title_style = Style::new().bold();
        let counter_style = Style::new().yellow();
        let key_style = Style::new().blue().bold();
        expected.set_style(Rect::new(15, 0, 19, 1), title_style);
        expected.set_style(Rect::new(31, 1, 1, 1), counter_style);
        expected.set_style(Rect::new(15, 3, 4, 1), key_style);
        expected.set_style(Rect::new(25, 3, 6, 1), key_style);
        expected.set_style(Rect::new(37, 3, 4, 1), key_style);

        assert_eq!(buf, expected);
    }
}
