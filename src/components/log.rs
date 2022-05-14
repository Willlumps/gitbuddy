use crate::git::gitlog::{fetch_history, Commit};
use crate::component_style::ComponentTheme;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, List as TuiList, ListItem, ListState};
use tui::Frame;

use std::path::PathBuf;

pub struct LogComponent {
    pub logs: Vec<Commit>,
    pub state: ListState,
    pub focused: bool,
    pub position: usize,
    repo_path: PathBuf,
    style: ComponentTheme,
}

impl LogComponent {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            logs: Vec::new(),
            state: ListState::default(),
            focused: false,
            position: 0,
            style: ComponentTheme::default(),
            repo_path,
        }
    }

    pub fn draw<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        rect: Rect,
    ) -> Result<()> {
        let list_items: Vec<ListItem> = self
            .logs
            .iter()
            .map(|item| {
                let text = Spans::from(vec![
                    Span::styled(item.get_id(), Style::default().fg(Color::Green)),
                    Span::raw(" "),
                    Span::raw(item.get_message()),
                ]);
                ListItem::new(text)
            })
            .collect();
        let list = TuiList::new(list_items)
            .block(
                Block::default()
                    .title(" Log ")
                    .borders(Borders::ALL)
                    .style(self.style.style())
                    .border_style(self.style.border_style())
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, rect, &mut self.state);

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        self.logs = fetch_history(&self.repo_path)?;
        Ok(())
    }

    pub fn handle_event(&mut self, ev: KeyEvent) {
        if !self.focused {
            return;
        }
        match ev.code {
            KeyCode::Char('j') if ev.modifiers == KeyModifiers::CONTROL => {
                self.decrement_position();
            }
            KeyCode::Char('k') if ev.modifiers == KeyModifiers::CONTROL => {
                self.increment_position();
            }
            _ => {}
        }
    }

    fn increment_position(&mut self) {
        self.position = self.position.saturating_sub(1);
        self.state.select(Some(self.position));
    }

    fn decrement_position(&mut self) {
        if self.position < self.logs.len() - 1 {
            self.position += 1;
            self.state.select(Some(self.position));
        }
    }

    pub fn focus(&mut self, focus: bool) {
        if focus {
            self.style = ComponentTheme::focused();
        } else {
            self.style = ComponentTheme::default();
        }
        self.focused = focus;
    }
}
