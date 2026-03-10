use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    symbols::{self, border},
    text::{Line, Text},
    widgets::{Axis, Block, Chart, Clear, Dataset, GraphType, Padding, Paragraph, Widget},
};

use crate::state::{Mode, RideState, Stats};

pub(crate) fn border(ride: &RideState) -> Paragraph<'_> {
    let title = Line::from(" -= It Suffices =- ".bold());
    let instructions = Line::from(vec![
        " Record ".into(),
        if ride.is_recording {
            "<R> ".red().slow_blink().bold()
        } else {
            "<R> ".blue().bold()
        },
        "Help ".into(),
        "<?> ".blue().bold(),
        "Quit ".into(),
        "<Q> ".blue().bold(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.centered())
        .border_set(border::ROUNDED);

    Paragraph::new("").centered().block(block)
}

pub(crate) fn level(ride: &RideState) -> Paragraph<'_> {
    let curr = Text::from(vec![Line::from(match ride.mode {
        Mode::Power => vec!["Power: ".into(), ride.power.to_string().yellow()],
        Mode::Resistance => {
            vec!["Resistance: ".into(), ride.resistance.to_string().yellow()]
        }
    })]);

    Paragraph::new(curr)
        .centered()
        .block(Block::new().padding(Padding::uniform(1)))
}

pub(crate) fn rolling(stats: &Stats) -> Paragraph<'_> {
    let power_3s = stats.power.rolling(3);
    let cadence_3s = stats.cadence.rolling(3);
    let heart_3s = stats.heart_rate.rolling(3);
    let speed_3s = stats.speed.rolling(3);

    let lines = Text::from(vec![
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
        Line::from(vec![
            "3s Speed: ".into(),
            format!("{:.2}", speed_3s / 100.).yellow(),
        ]),
    ]);

    Paragraph::new(lines).centered()
}

pub(crate) fn totals(stats: &Stats) -> Paragraph<'_> {
    let joules = stats.power.total();
    let dist_total = stats.distance.total();

    let lines = Text::from(vec![
        Line::from(vec![
            "Total Work: ".into(),
            format!("{:.1} kJ", joules as f32 / 1000.).yellow(),
        ]),
        Line::from(vec![
            "Total Distance: ".into(),
            format!("{:.3} km", dist_total as f32 / 1000.).yellow(),
        ]),
    ]);

    Paragraph::new(lines).centered()
}

pub(crate) fn chart<'a>(name: &'a str, color: Style, pslice: &'a mut [(f64, f64)]) -> Chart<'a> {
    let raw = pslice.iter().map(|(_, d)| *d as i32);
    let min: f64 = raw.clone().min().unwrap_or_default().into();
    let max: f64 = raw.clone().max().unwrap_or_default().into();
    let size = raw.len();

    let datasets = vec![
        Dataset::default()
            .name(name)
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(color)
            .data(pslice),
    ];

    let x_axis = Axis::default().bounds([0.0, size as f64]);

    let y_axis = Axis::default()
        .bounds([min, max])
        .labels([min.to_string(), max.to_string()]);

    Chart::new(datasets)
        .block(Block::new().padding(Padding::uniform(1)))
        .x_axis(x_axis)
        .y_axis(y_axis)
}

#[derive(Default)]
pub(crate) struct Help {}

impl Widget for Help {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("Controls".bold());
        let commands = Text::from(vec![
            Line::from(vec![" Help".into()]),
            Line::from(vec![" More".into()]),
            Line::from(vec![" Less".into()]),
            Line::from(vec![" Record".into()]),
            Line::from(vec![" Modes (Resist/ERG)".into()]),
            Line::from(vec![" Views (Rolling/Totals/Charts)".into()]),
            Line::from(vec![" Quit".into()]),
        ]);
        let keys = Text::from(vec![
            Line::from(vec!["<?>".blue().bold()]),
            Line::from(vec!["<Up>".blue().bold()]),
            Line::from(vec!["<Down>".blue().bold()]),
            Line::from(vec!["<R>".blue().bold()]),
            Line::from(vec!["<Left/Right>".blue().bold()]),
            Line::from(vec!["<Tab>".blue().bold()]),
            Line::from(vec!["<Q> ".blue().bold()]),
        ]);
        let block = Block::bordered()
            // .on_cyan()
            .title(title.centered())
            .border_set(border::ROUNDED);

        let [_, popup, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(1),
                Constraint::Min(9),
                Constraint::Min(1),
            ])
            .areas(area);
        let [_, outer, _] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Fill(8),
                Constraint::Fill(1),
            ])
            .areas(popup);
        let [_, inner, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Fill(10), Constraint::Min(1)])
            .areas(outer);
        let [left, _, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(20),
                Constraint::Min(1),
                Constraint::Fill(20),
            ])
            .areas(inner);

        Clear.render(outer, buf);
        Paragraph::new("")
            .centered()
            .block(block)
            .render(outer, buf);

        Paragraph::new(commands)
            .block(Block::new().padding(Padding::uniform(1)))
            .right_aligned()
            .render(left, buf);

        Paragraph::new(keys)
            .block(Block::new().padding(Padding::uniform(1)))
            .left_aligned()
            .render(right, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    #[test]
    fn render_help() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 50, 13));
        let help = Help::default();
        help.render(buf.area, &mut buf);

        let mut expected = Buffer::with_lines(vec![
            "                                                  ",
            "                                                  ",
            "     ╭───────────────Controls───────────────╮     ",
            "     │                                      │     ",
            "     │              Help   <?>              │     ",
            "     │              More   <Up>             │     ",
            "     │              Less   <Down>           │     ",
            "     │            Record   <R>              │     ",
            "     │ Modes (Resist/ERG   <Left/Right>     │     ",
            "     │                                      │     ",
            "     ╰──────────────────────────────────────╯     ",
            "                                                  ",
            "                                                  ",
        ]);
        let key_style = Style::new().blue().bold();

        expected.set_style(Rect::new(21, 2, 8, 1), Style::new().bold());
        expected.set_style(Rect::new(27, 4, 3, 1), key_style);
        expected.set_style(Rect::new(27, 5, 4, 1), key_style);
        expected.set_style(Rect::new(27, 6, 6, 1), key_style);
        expected.set_style(Rect::new(27, 7, 3, 1), key_style);
        expected.set_style(Rect::new(27, 8, 12, 1), key_style);

        assert_eq!(buf, expected);
    }
}
