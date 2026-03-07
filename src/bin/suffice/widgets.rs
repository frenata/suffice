use ratatui::{
    style::{Style, Stylize},
    symbols::{self, border},
    text::{Line, Text},
    widgets::{Axis, Block, Chart, Dataset, GraphType, Padding, Paragraph},
};

use crate::state::{Mode, RideState, Stats};

pub(crate) fn border(ride: &RideState) -> Paragraph<'_> {
    let title = Line::from(" -= It Suffices =- ".bold());
    let instructions = Line::from(vec![
        " More ".into(),
        "<Up>".blue().bold(),
        " Less ".into(),
        "<Down>".blue().bold(),
        " Record ".into(),
        if ride.is_recording {
            "<R>".red().slow_blink().bold()
        } else {
            "<R>".blue().bold()
        },
        " Quit ".into(),
        "<Q> ".blue().bold(),
    ]);
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(instructions.centered())
        .border_set(border::THICK);

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
        Line::from(vec!["Total Work: ".into(), format!("{}", joules).yellow()]),
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

    let x_axis = Axis::default()
        // .style(Style::default().white())
        .bounds([0.0, size as f64]);

    let y_axis = Axis::default()
        // .style(Style::default().white())
        .bounds([min, max])
        .labels([min.to_string(), max.to_string()]);

    Chart::new(datasets)
        .block(Block::new().padding(Padding::uniform(1)))
        .x_axis(x_axis)
        .y_axis(y_axis)
}
