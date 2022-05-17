use crate::component_style::ComponentTheme;
use crate::git::stage::{stage_all, stage_file, unstage_file};
use crate::git::git_status::{get_file_status, FileStatus, StatusLoc, StatusType};
use crate::components::commit_popup::CommitPopup;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::{Modifier, Style};
use tui::text::Span;
use tui::widgets::{Block, BorderType, Borders, List as TuiList, ListItem, ListState};
use tui::Frame;

use super::Component;
use std::path::PathBuf;

pub struct FileComponent {
    files: Vec<FileStatus>,
    state: ListState,
    focused: bool,
    position: usize,
    style: ComponentTheme,
    repo_path: PathBuf,
    pub commit_popup: CommitPopup,
}

// TODO:
//  - Show file diff in window if desired
//  - Files that have some hunks staged while others aren't
//    - Show both staged and unstaged?

impl FileComponent {
    pub fn new(repo_path: PathBuf) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            files: Vec::new(),
            state,
            focused: false,
            position: 0,
            style: ComponentTheme::default(),
            commit_popup: CommitPopup::new(repo_path.clone()),
            repo_path,
        }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, rect: Rect) -> Result<()> {
        let list_items: Vec<ListItem> = self
            .files
            .iter()
            .map(|item| {
                let status_type = char::from(item.status_type.clone());
                let style = ComponentTheme::file_status_style(item.status_loc.clone());
                ListItem::new(Span::styled(
                    format!("{} {}", status_type, item.path.clone()),
                    style,
                ))
            })
            .collect();
        let list = TuiList::new(list_items)
            .block(
                Block::default()
                    .title(" Files ")
                    .style(self.style.style())
                    .borders(Borders::ALL)
                    .border_style(self.style.border_style())
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, rect, &mut self.state);

        Ok(())
    }

    fn increment_position(&mut self) {
        if self.position != 0 {
            self.position -= 1;
            self.state.select(Some(self.position));
        }
    }

    fn decrement_position(&mut self) {
        if self.position < self.files.len() - 1 {
            self.position += 1;
            self.state.select(Some(self.position));
        }
    }
}

impl Component for FileComponent {
    fn update(&mut self) -> Result<()> {
        self.files = get_file_status(&self.repo_path)?;
        if self.files.is_empty() {
            self.files.push(FileStatus {
                path: "Working tree clean".to_string(),
                status_type: StatusType::Unmodified,
                status_loc: StatusLoc::None,
            });
        }
        Ok(())
    }

    fn handle_event(&mut self, ev: KeyEvent) -> Result<()> {
        if !self.focused {
            return Ok(());
        }
        match ev.code {
            KeyCode::Char('j') => {
                self.decrement_position();
            }
            KeyCode::Char('k') => {
                self.increment_position();
            }
            KeyCode::Char('s') if ev.modifiers == KeyModifiers::CONTROL => {
                stage_all(&self.repo_path)?;
            }
            KeyCode::Char('s') => {
                if let Some(file) = self.files.get(self.position) {
                    stage_file(&self.repo_path, &file.path)?;
                }
            }
            KeyCode::Char('u') => {
                if let Some(file) = self.files.get(self.position) {
                    unstage_file(&self.repo_path, &file.path)?;
                }
            }
            KeyCode::Char('c') => {
                // TODO: Check if there are staged files to commit?
                self.commit_popup.focus();
            }
            _ => {}
        }

        Ok(())
    }

    fn focus(&mut self, focus: bool) {
        // Comment
        if focus {
            self.style = ComponentTheme::focused();
        } else {
            self.style = ComponentTheme::default();
        }
        self.focused = focus;
    }
}
