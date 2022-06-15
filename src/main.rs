mod app;
mod component_style;
mod components;
mod error;
mod git;
mod list_window;

use crate::app::{App, ProgramEvent};
use crate::components::{centered_rect, ComponentType};
use crate::error::Error;
use crate::git::{init_new_repo, is_empty_repo, is_repo};

use std::env::current_dir;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver, Select};
use crossterm::event::{
    poll, read, DisableMouseCapture, Event as CEvent, KeyCode, KeyEvent, KeyModifiers,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use git::commit::create_initial_commit;
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::Text;
use tui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph};
use tui::{Frame, Terminal};

pub enum Event<I> {
    Input(I),
    Tick,
}

fn main() -> Result<()> {
    let (tx, rx) = unbounded();
    let (ev_tx, ev_rx) = unbounded();
    let tick_rate = Duration::from_millis(2000);

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if let Ok(poll) = poll(timeout) {
                if poll {
                    if let CEvent::Key(key) = read().expect("Should read event") {
                        tx.send(Event::Input(key)).expect("Should send event");
                    }
                } else if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
                    last_tick = Instant::now();
                }
            }
        }
    });

    // setup terminal
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Grab the project root for dev purposes, this will eventually want to be
    // replaced with a passed argument or the current dir where the program
    // is executed from.
    //let repo_path = current_dir()?;
    //let repo_path = std::path::PathBuf::from("/Users/reina/rust/programming-rust");
    let repo_path = std::path::PathBuf::from("/Users/reina/projects/rust/test");

    #[allow(clippy::collapsible_if)]
    if !is_repo(&repo_path) {
        if prompt_new_repo(&repo_path, &mut terminal, rx.clone()).is_err() {
            restore_terminal(&mut terminal)?;
            return Ok(());
        }
    }

    if is_empty_repo(&repo_path)? {
        if let Err(err) = create_initial_commit(&repo_path) {
            eprintln!("Failed to make initial commit: {err:?}");
            return Ok(());
        }
    }

    // Initialize and run
    let mut app = App::new(repo_path, &ev_tx);
    let res = run_app(&mut terminal, &mut app, rx, ev_rx);

    restore_terminal(&mut terminal)?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn restore_terminal<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    io::stdout().execute(DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn prompt_new_repo<B: Backend>(
    repo_path: &Path,
    terminal: &mut Terminal<B>,
    rx: Receiver<Event<KeyEvent>>,
) -> Result<(), Error> {
    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let area = centered_rect(40, 8, f.size());

            let border = Block::default()
                .style(Style::default())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);

            let container = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(4), Constraint::Length(4)].as_ref())
                .split(area);

            let mut prompt = Text::raw("!! Directory is not a repository !!");
            prompt.extend(Text::raw("Initialize new repo at\n"));
            prompt.extend(Text::styled(format!("{:?}?", repo_path), Style::default().fg(Color::Yellow)));

            let init_prompt = Paragraph::new(prompt)
                .alignment(tui::layout::Alignment::Center)
                .style(Style::default().fg(Color::White));

            let options = vec![ListItem::new("Yes"), ListItem::new("No")];

            let list = List::new(options)
                .highlight_style(
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_widget(Clear, area);
            f.render_widget(border, area);
            f.render_widget(init_prompt, container[0]);
            f.render_stateful_widget(list, container[1], &mut state);
        })?;

        let input_event = rx.recv().expect("Failed to receive");
        match input_event {
            Event::Input(input) => {
                match input.code {
                    KeyCode::Char('j') => {
                        if state.selected() == Some(0) {
                            state.select(Some(1))
                        }
                    }
                    KeyCode::Char('k') => {
                        if state.selected() == Some(1) {
                            state.select(Some(0))
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selection) = state.selected() {
                            if selection == 0 {
                                // init repo
                                init_new_repo(repo_path)?;
                                break;
                            } else {
                                return Err(Error::Unknown("NO".to_string()));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Tick => {}
        }
    }
    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    rx: Receiver<Event<KeyEvent>>,
    event_rx: Receiver<ProgramEvent>,
) -> Result<(), Error> {
    loop {
        app.update()?;

        terminal.draw(|f| {
            if let Err(e) = ui(f, app) {
                eprintln!("Draw error: {}", e);
            }
        })?;

        let mut select = Select::new();
        select.recv(&event_rx);
        select.recv(&rx);

        let oper = select.select();
        match oper.index() {
            0 => {
                let event = oper.recv(&event_rx).expect("Receive failed");
                match event {
                    ProgramEvent::Error(error) => {
                        app.display_error(error);
                    }
                    ProgramEvent::Focus(component) => {
                        app.focus(component);
                    }
                    ProgramEvent::Git(git_event) => {
                        app.handle_git_event(git_event)?;
                    }
                }
            }
            1 => {
                let input_event = oper.recv(&rx).expect("Receive failed");
                if app.is_popup_visible() {
                    app.handle_popup_input(input_event);
                } else {
                    match input_event {
                        Event::Input(input) => match input.code {
                            KeyCode::Char('q') if input.modifiers == KeyModifiers::CONTROL => {
                                return Ok(());
                            }
                            KeyCode::Char('1') => {
                                app.focus(ComponentType::FilesComponent);
                            }
                            KeyCode::Char('2') => {
                                app.focus(ComponentType::BranchComponent);
                            }
                            KeyCode::Char('3') => {
                                app.focus(ComponentType::LogComponent);
                            }
                            KeyCode::Char('4') => {
                                app.focus(ComponentType::DiffComponent);
                            }
                            _ => {
                                app.handle_input(input);
                            }
                        },
                        Event::Tick => {}
                    }
                }
            }
            _ => {
                unreachable!();
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) -> Result<()> {
    let size = f.size();
    let container = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(size);

    // Status, Files, Branches, Logs?
    let left_container = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(4),
                Constraint::Length(8),
                Constraint::Length(15),
                Constraint::Length(8),
            ]
            .as_ref(),
        )
        .split(container[0]);

    app.status.draw(f, left_container[0])?;
    app.branches.draw(f, left_container[2])?;
    app.logs.draw(f, left_container[3])?;
    app.files.draw(f, left_container[1])?;
    app.diff.draw(f, container[1])?;

    if app.is_popup_visible() {
        app.draw_popup(f, size)?;
    }

    Ok(())
}
