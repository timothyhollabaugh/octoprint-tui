use futures::Future;

use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Text, Widget};
use tui::Terminal;

use crate::octoprint::JobResponse;
use crate::octoprint::OctoprintError;
use crate::octoprint::StateResponse;

pub enum UiEvent {
    JobUpdate(JobResponse),
    StateUpdate(StateResponse),
}

impl From<JobResponse> for UiEvent {
    fn from(job: JobResponse) -> UiEvent {
        UiEvent::JobUpdate(job)
    }
}

#[derive(Debug)]
pub enum UiError {
    Timer(tokio_timer::Error),
    Octoprint(OctoprintError),
}

impl From<tokio_timer::Error> for UiError {
    fn from(err: tokio_timer::Error) -> UiError {
        UiError::Timer(err)
    }
}

impl From<OctoprintError> for UiError {
    fn from(err: OctoprintError) -> UiError {
        UiError::Octoprint(err)
    }
}

#[derive(Clone)]
struct UiState {
    progress: f64,
    filename: Option<String>,
    status: Option<String>,
    print_time: Option<f64>,
    estimated_time: Option<f64>,
    remaining_time: Option<f64>,
    hotend_temp: Option<f64>,
    hotend_target: Option<f64>,
    bed_temp: Option<f64>,
    bed_target: Option<f64>,
}

pub struct Ui<B: Backend> {
    terminal: Terminal<B>,
    state: UiState,
}

impl<B: Backend> Ui<B> {
    pub fn new(mut terminal: Terminal<B>) -> Ui<B> {
        terminal.clear().expect("Could not clear terminal");
        terminal.hide_cursor().expect("Could not hide cursor");

        let state = UiState {
            progress: 0.0,
            filename: None,
            status: None,
            print_time: None,
            estimated_time: None,
            remaining_time: None,
            hotend_temp: None,
            hotend_target: None,
            bed_temp: None,
            bed_target: None,
        };

        Ui { terminal, state }
    }

    pub fn draw(&mut self, event: UiEvent) {
        match event {
            UiEvent::JobUpdate(job) => {
                self.state.progress = job.progress.completion.unwrap_or(0.0);
                self.state.filename = job.job.file.name;
                self.state.print_time = job.progress.print_time;
                self.state.estimated_time =
                    job.job.last_print_time.or(job.job.estimated_print_time);
                self.state.remaining_time = job.progress.print_time_left;
            }
            UiEvent::StateUpdate(state) => {
                self.state.status = state.state.map(|s| s.text);
                self.state.hotend_temp = state
                    .temperature
                    .clone()
                    .and_then(|t| t.tool0)
                    .map(|t| t.actual);
                self.state.hotend_target = state
                    .temperature
                    .clone()
                    .and_then(|t| t.tool0)
                    .map(|t| t.target);
                self.state.bed_temp = state
                    .temperature
                    .clone()
                    .and_then(|t| t.bed)
                    .map(|t| t.actual);
                self.state.bed_target = state
                    .temperature
                    .clone()
                    .and_then(|t| t.bed)
                    .map(|t| t.target);
            }
        }

        let state = self.state.clone();

        self.terminal
            .draw(|mut f| {
                let size = f.size();

                let style = Style::default().fg(Color::White).bg(Color::Black);

                Block::default().style(style).render(&mut f, size);

                let title = state.filename.unwrap_or("No File".to_string());

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Length(1), // Status
                            Constraint::Length(1), // Filename
                            Constraint::Length(5),
                            Constraint::Length(2), // Temperatures
                            Constraint::Min(5),
                            Constraint::Length(2), // Times
                            Constraint::Length(1),
                            Constraint::Length(1), // Progress
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let status_chunk = chunks[1];
                let filename_chunk = chunks[2];
                let temperatures_chunk = chunks[4];
                let times_chunk = chunks[6];
                let progress_chunk = chunks[8];

                Paragraph::new(
                    [Text::Styled(
                        state.status.unwrap_or("No Status".to_string()).into(),
                        style,
                    )]
                    .into_iter(),
                )
                .style(style)
                .alignment(Alignment::Center)
                .render(&mut f, status_chunk);

                Paragraph::new([Text::Styled(title.into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, filename_chunk);

                let temperature_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                    .split(temperatures_chunk);

                let hotend_chucks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
                    .split(temperature_chunks[0]);

                Paragraph::new([Text::Styled("Hotend".into(), style)].into_iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, hotend_chucks[0]);

                Paragraph::new(
                    [Text::Styled(
                        format!(
                            "{}/{}°C",
                            state
                            .hotend_temp
                                .map(|t| format!("{:.2}", t))
                                .unwrap_or("--".to_string()),
                            state
                                .hotend_target
                                .map(|t| format!("{:.0}", t))
                                .unwrap_or("--".to_string()),
                        )
                        .into(),
                        style,
                    )]
                    .into_iter(),
                )
                .style(style)
                .alignment(Alignment::Center)
                .render(&mut f, hotend_chucks[1]);

                let bed_chucks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
                    .split(temperature_chunks[1]);

                Paragraph::new([Text::Styled("Bed".into(), style)].into_iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, bed_chucks[0]);

                Paragraph::new(
                    [Text::Styled(
                        format!(
                            "{}/{}°C",
                            state
                                .bed_temp
                                .map(|t| format!("{:.2}", t))
                                .unwrap_or("--".to_string()),
                            state
                                .bed_target
                                .map(|t| format!("{:.0}", t))
                                .unwrap_or("--".to_string()),
                        )
                        .into(),
                        style,
                    )]
                    .into_iter(),
                )
                .style(style)
                .alignment(Alignment::Center)
                .render(&mut f, bed_chucks[1]);

                let time_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                            Constraint::Ratio(1, 3),
                        ]
                        .as_ref(),
                    )
                    .split(times_chunk);

                let print_time = match state.print_time {
                    Some(s) => {
                        let (hours, minutes, seconds) = seconds_to_time(s);
                        format!("{:.0}:{:02.0}:{:02.0}", hours, minutes, seconds)
                    }
                    None => "--:--:--".to_string(),
                };

                let print_time_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
                    .split(time_chunks[0]);

                Paragraph::new([Text::Styled("Print Time".into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, print_time_chunks[0]);

                Paragraph::new([Text::Styled(print_time.into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, print_time_chunks[1]);

                let estimated_time = match state.estimated_time {
                    Some(s) => {
                        let (hours, minutes, seconds) = seconds_to_time(s);
                        format!("{:.0}:{:2.0}:{:2.0}", hours, minutes, seconds)
                    }
                    None => "--:--:--".to_string(),
                };

                let estimated_time_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
                    .split(time_chunks[1]);

                Paragraph::new([Text::Styled("Estimated Time".into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, estimated_time_chunks[0]);

                Paragraph::new([Text::Styled(estimated_time.into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, estimated_time_chunks[1]);

                let remaining_time = match state.remaining_time {
                    Some(s) => {
                        let (hours, minutes, seconds) = seconds_to_time(s);
                        format!("{:.0}:{:2.0}:{:2.0}", hours, minutes, seconds)
                    }
                    None => "--:--:--".to_string(),
                };

                let remaining_time_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
                    .split(time_chunks[2]);

                Paragraph::new([Text::Styled("Remaining Time".into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, remaining_time_chunks[0]);

                Paragraph::new([Text::Styled(remaining_time.into(), style)].iter())
                    .style(style)
                    .alignment(Alignment::Center)
                    .render(&mut f, remaining_time_chunks[1]);

                if state.progress > 0.0 {
                    Gauge::default()
                        .style(
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Black)
                                .modifier(Modifier::ITALIC),
                        )
                        .label(&format!("{:.2}%", state.progress))
                        .percent(state.progress as u16)
                        .render(&mut f, progress_chunk);
                }
            })
            .expect("Could not draw to terminal");
    }
}

fn seconds_to_time(seconds: f64) -> (u64, u64, f64) {
    let hours = (seconds / (60.0 * 60.0)) as u64;
    let seconds = seconds % (60.0 * 60.0);
    let minutes = (seconds / 60.0) as u64;
    let seconds = seconds % 60.0;

    (hours, minutes, seconds)
}
