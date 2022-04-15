//#![allow(unused_imports)]
use crossterm::{
    event::{poll, read, DisableMouseCapture, Event as CEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen},
};
use git2::{Branch, BranchType, Error, Repository};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::{
    io, thread,
    time::{Duration, Instant},
};
use tui::{
    Frame,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Clear, Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

enum Event<I> {
    Input(I),
    Tick,
}

enum BranchTypeState {
    Local,
    Remote,
}

impl BranchTypeState {
    fn next(current_state: BranchTypeState) -> BranchTypeState {
        match current_state {
            BranchTypeState::Local => {
                BranchTypeState::Remote
            },
            BranchTypeState::Remote => {
                BranchTypeState::Local
            }
        }
    }
}

struct BranchState {
    local: ListState,
    remote: ListState,
    widget: BranchTypeState,
}

impl BranchState {
    fn new() -> Self {
        Self {
            local: ListState::default(),
            remote: ListState::default(),
            widget: BranchTypeState::Local,
        }
    }
}

struct App<'a> {
    show_delete_branch: bool,
    repo: &'a Repository,
    branch_state: BranchState,
}

impl<'a> App<'a> {
    fn new(repo: &'a Repository) -> Self {
        Self {
            show_delete_branch: false,
            repo,
            branch_state: BranchState::new(),
        }
    }
}

fn main() -> crossterm::Result<()> {
    let repo = match Repository::open("/Users/reina/school/groupwork/capstone/") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(500);

    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if poll(timeout).is_ok() {
                if let CEvent::Key(key) = read().expect("Should read event") {
                    tx.send(Event::Input(key)).expect("Should send event");
                }
            }

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
                last_tick = Instant::now();
            }
        }
    });

    // setup terminal
    enable_raw_mode()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let app = App::new(&repo);
    let res = run_app(&mut terminal, app, rx);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App, rx: Receiver<Event<crossterm::event::KeyEvent>>) -> io::Result<()> {
    app.branch_state.local.select(Some(0));
    app.branch_state.remote.select(Some(0));

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        match rx.recv() {
            Ok(event) => match event {
                Event::Input(input) => match input.code {
                    KeyCode::Char('d') => {
                        app.show_delete_branch = !app.show_delete_branch;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    KeyCode::Char('j') => {
                        move_down(&mut app.branch_state, app.repo);
                    }
                    KeyCode::Char('k') => {
                        move_up(&mut app.branch_state);
                    }
                    KeyCode::Char('n') => {
                        if input.modifiers == KeyModifiers::CONTROL {
                            app.branch_state.widget = BranchTypeState::next(app.branch_state.widget);
                        }
                    }
                    KeyCode::Enter => {
                        select_branch(&app.branch_state, app.repo);
                    }
                    _ => {}
                },
                Event::Tick => {}
            },
            Err(e) => {
                eprintln!("FIX ME {e}")
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
        let size = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(2),
                ]
                .as_ref(),
            )
            .split(size);

        let header = Paragraph::new("Git Buddy")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        let (local, remote) = render_branches(app.repo);
        let branch_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints(
                [Constraint::Percentage(50), Constraint::Percentage(50)].as_ref(),
            )
            .split(chunks[1]);

        match app.branch_state.widget {
            BranchTypeState::Local => {
                f.render_stateful_widget(local, branch_chunks[0], &mut app.branch_state.local);
                f.render_widget(remote, branch_chunks[1]);
            },
            BranchTypeState::Remote => {
                f.render_widget(local, branch_chunks[0]);
                f.render_stateful_widget(remote, branch_chunks[1], &mut app.branch_state.remote);
            }
        }
    if app.show_delete_branch {
        let block = Block::default().title("Popup").borders(Borders::ALL);
        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area); //this clears out the background
        f.render_widget(block, area);
    }
}

fn select_branch(state: &BranchState, repo: &Repository) {
    let branch_list: Vec<Branch>;
    let index: usize;
    let mut branch_name;

    match state.widget {
        BranchTypeState::Local => {
            branch_list = get_branches(repo, BranchType::Local);
            index = state.local.selected().expect("Should have state");
            branch_name = get_branch_name_by_index(&branch_list, index);

        },
        BranchTypeState::Remote => {
            branch_list = get_branches(repo, BranchType::Remote);
            index = state.remote.selected().expect("Should have state");
            branch_name = get_branch_name_by_index(&branch_list, index);
            let branch = new_branch(repo, &branch_name);
            branch_name = get_branch_name(&branch);
        },
    }
    match checkout_branch(repo, &branch_name) {
        Ok(_) => {},
        Err(e) => { eprintln!("Failed to checkout branch: {e}"); },
    };
}

fn move_down(state: &mut BranchState, repo: &Repository) {
    let branch_list: Vec<Branch>;
    match state.widget {
        BranchTypeState::Local => {
            branch_list = get_branches(repo, BranchType::Local);
            if let Some(selected) = state.local.selected() {
                if selected < branch_list.len() - 1 {
                    state.local.select(Some(selected + 1));
                }
            }
        },
        BranchTypeState::Remote => {
            branch_list = get_branches(repo, BranchType::Remote);
            if let Some(selected) = state.remote.selected() {
                if selected < branch_list.len() - 1 {
                    state.remote.select(Some(selected + 1));
                }
            }
        },
    }
}

fn move_up(state: &mut BranchState) {
    match state.widget {
        BranchTypeState::Local => {
            if let Some(selected) = state.local.selected() {
                if selected > 0 {
                    state.local.select(Some(selected - 1));
                }
            }
        },
        BranchTypeState::Remote => {
            if let Some(selected) = state.remote.selected() {
                if selected > 0 {
                    state.remote.select(Some(selected - 1));
                }
            }
        },
    }
}

fn render_branches(repo: &Repository) -> (List, List) {
    let local_branches = render_local_branches(get_branches(repo, BranchType::Local));
    let remote_branches = render_remote_branches(get_branches(repo, BranchType::Remote));

    let local_branch_list = List::new(local_branches)
        .block(Block::default().title("Local").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::LightBlue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    let remote_branch_list = List::new(remote_branches)
        .block(Block::default().title("Remote").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::LightBlue)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    (local_branch_list, remote_branch_list)
}

fn render_local_branches(branches: Vec<Branch>) -> Vec<ListItem> {
    branches
        .iter()
        .map(|branch| {
            let b = branch.name()
                .expect("Branch should exist")
                .expect("Branch should have a name");
            if branch.is_head() {
                return ListItem::new(format!("* {}", b))
                    .style(Style::default().fg(Color::Green));
            }
            ListItem::new(format!("  {}", b))
        })
        .collect()
}

fn render_remote_branches(branches: Vec<Branch>) -> Vec<ListItem> {
    branches
        .iter()
        .map(|branch| {
            let b = branch.name()
                .expect("Branch should exist")
                .expect("Branch should have a name");
                return ListItem::new(format!("  {}", b))
                    .style(Style::default().fg(Color::Red));
        })
        .collect()
}

fn get_branches(repo: &Repository, branch_type: BranchType) -> Vec<Branch> {
    let mut branch_list: Vec<Branch> = Vec::new();

    let branches = repo.branches(Some(branch_type)).unwrap();
    let branches: Vec<Result<(Branch, BranchType), Error>> =
        branches.collect::<Vec<Result<_, _>>>();

    for branch in branches {
        match branch {
            Ok(b) => branch_list.push(b.0),
            Err(e) => eprintln!("{}", e),
        }
    }

    branch_list
}


fn get_branch_name_by_index(branches: &[Branch], index: usize) -> String {
    let branch = branches.get(index).expect("Branch should exist");
    get_branch_name(branch)
}

fn get_branch_name(branch: &Branch) -> String {
    let name = branch
        .name()
        .expect("Branch should exist")
        .expect("Branch should have a nanme");
    name.to_string()
}

fn new_branch<'a>(repo: &'a Repository, refname: &str)  -> Branch<'a> {
    // TODO: Check if reference already exists
    let (object, _reference) = repo.revparse_ext(refname).expect("Object not found");
    let commit = object.as_commit().unwrap();
    repo.branch(&refname[7..], commit, false).unwrap()
}

fn checkout_branch(repo: &Repository, refname: &str) -> Result<(), git2::Error> {
    // TODO: Determine full refname before this point?
    let full_ref = format!("refs/heads/{refname}");
    let cur_ref = repo.head()?;
    let statuses = repo.statuses(Some(git2::StatusOptions::new().include_ignored(false)))?;

    if statuses.is_empty() {
        repo.set_head(&full_ref)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;
        if let Err(e) = repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force(),)) {
          repo.set_head(cur_ref.name().unwrap())?;
          return Err(e);
       }
       return Ok(());
    } else {
        // Handle uncommitted changes
    }
    Ok(())
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
