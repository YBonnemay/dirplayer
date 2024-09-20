use crate::app::{App, Tab};
use crate::directories::State;
use crate::files::{FileLine, Files};
use crate::utils;
use chrono::{DateTime, Utc};
// use crossterm::style::Stylize;
use itertools::Itertools;
use ratatui::widgets::Cell;
use ratatui::widgets::{Row, Table, TableState};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{palette::tailwind, Color, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Tabs, Widget},
    Frame,
};
use std::rc::Rc;
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter, Debug)]
pub enum Focus {
    #[default]
    Middle,
    Down,
}

#[derive(Clone, Copy, Display, FromRepr, EnumIter, Debug)]
pub enum MenuTab {
    #[strum(to_string = "Directories")]
    Directories,
    #[strum(to_string = "Files")]
    Files,
}

impl MenuTab {
    /// Return tab's name as a styled `Line`
    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    /// A block surrounding the tab's content
    fn block(self) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Directories => tailwind::BLUE,
            Self::Files => tailwind::EMERALD,
        }
    }
}

impl Widget for MenuTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Directories => {
                Paragraph::new("Directories")
                    .block(self.block())
                    .render(area, buf);
            }
            Self::Files => {
                Paragraph::new("Files")
                    .block(self.block())
                    .render(area, buf);
            }
        }
    }
}

pub fn get_chunks(f: &Frame) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .split(f.size())
}

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = crate::ui::get_chunks(f);

    let titles = MenuTab::iter().map(MenuTab::title);
    let highlight_style = (Color::default(), tailwind::RED.c700);
    let selected_tab_index = match app.current_place {
        Tab::Directories => 0,
        Tab::Files => 1,
    };

    let tabs_widget = Tabs::new(titles)
        .highlight_style(highlight_style)
        .select(selected_tab_index)
        .padding("", "")
        .divider(" ");

    f.render_widget(tabs_widget, chunks[0]);

    match app.current_place {
        Tab::Directories => {
            let completions_spans = app.directories.get_displayable_completions();

            let mut directories = app
                .directories
                .working_directories
                .iter()
                .enumerate()
                .map(|(i, working_path)| {
                    let mut style = Style::default().fg(Color::Yellow);

                    if i == (app.directories.line_index as usize)
                        && app.directories.state != State::Editing
                    {
                        style = style.bg(Color::Rgb(51, 51, 51));
                    };

                    let mut line = Line::from(Span::styled(working_path.path.clone(), style));
                    if i == app.directories.line_index as usize {
                        line = line.style(style);
                    }
                    line
                })
                .collect_vec();

            // Append candidate line
            if app.directories.state == State::Editing {
                let mut candidate_line = vec![
                    Span::from(app.directories.working_directory.clone()),
                    Span::from(" | ".to_string()),
                ];

                let mut tt = completions_spans.into_iter().flatten().collect_vec();
                candidate_line.append(&mut tt);

                directories.push(Line::from(candidate_line));
            };

            // let border_color_middle = Color::White;
            let border_color_middle = Color::DarkGray;

            // let constraints = &[Constraint::Percentage(80), Constraint::Length(30)];
            let constraints = &[];
            let list = Paragraph::new(directories).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color_middle)),
            );

            let mut state = TableState::default();
            state.select(Some(app.directories.line_index as usize));
            f.render_stateful_widget(
                Table::new(vec![Row::new(vec![""])], constraints),
                chunks[1],
                &mut state,
            );
            f.render_widget(list, chunks[1]);
        }

        Tab::Files => {
            // Display files
            let lines = &app.files.lines_filtered;

            if lines.len() < 10 {
                // let dir_entries: Vec<DirEntry> = vec![];
                let dir_entries = lines
                    .iter()
                    .map(|line| line.dir_entry.file_name())
                    .collect_vec();

                log::debug!("LIST lines {:?}", dir_entries);
            }

            let (slice_free_index_high_border, slice_index, slice_free_index_low_border) =
                utils::get_borders(
                    lines.len() as isize,
                    chunks[1].height as isize + 1,
                    app.files.line_index as isize,
                );

            let list_items =
                &lines[slice_free_index_high_border as usize..slice_free_index_low_border as usize];

            let list_items: Vec<Row> = list_items
                .iter()
                .filter_map(|e| {
                    let FileLine { dir_entry, indices } = e;
                    let metadata = dir_entry.metadata();
                    if metadata.is_err() {
                        return None;
                    }

                    let created = dir_entry.metadata().unwrap().created().unwrap();
                    let date_time = DateTime::<Utc>::from(created);
                    let dir_entry_path = dir_entry.file_name();

                    let path: &str = dir_entry_path.to_str().unwrap();

                    let spans = utils::style::string_to_styled_text(path, indices);
                    let (r, g, b) = Files::date_to_color(created);

                    let data = vec![
                        Cell::from(spans),
                        Cell::from(date_time.format("%Y-%m-%d %H-%M-%S").to_string())
                            .style(Style::default().fg(ratatui::style::Color::Rgb(r, g, b))),
                    ];
                    Some(Row::new(data))
                })
                .collect();

            let constraints = &[Constraint::Percentage(80), Constraint::Length(30)];
            let displayables = Table::new(list_items, constraints)
                .highlight_style(Style::default().bg(Color::Rgb(51, 51, 51)));

            let mut state = TableState::default();
            state.select(Some(slice_index as usize));
            f.render_stateful_widget(displayables, chunks[1], &mut state);

            let mode_footer = Paragraph::new(
                Line::from(app.files.filter.clone())
                    .style(Style::default().fg(ratatui::style::Color::Rgb(255, 255, 0))),
            )
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(mode_footer, chunks[2]);
        }
    }
}
