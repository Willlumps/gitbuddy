use crate::component_style::ComponentTheme;
use crate::git::gitdiff::get_diff;
use crate::list_window::{ListWindow, ScrollDirection};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::backend::Backend;
use tui::layout::Rect;
use tui::style::Style;
use tui::text::Span;
use tui::widgets::{Block, BorderType, Borders, List as TuiList, ListItem, ListState};
use tui::Frame;

use std::error::Error;

pub struct DiffComponent {
    pub diffs: Vec<DiffLine>,
    pub state: ListState,
    pub focused: bool,
    style: ComponentTheme,
    path: String,
    window: ListWindow,
}

#[derive(Debug, PartialEq)]
pub struct DiffLine {
    pub content: String,
    pub origin: char,
    pub style: Style,
}

impl DiffLine {
    pub fn origin(&self) -> char {
        self.origin
    }

    pub fn style(&self) -> Style {
        self.style
    }

    pub fn content(&self) -> &String {
        &self.content
    }
}

impl DiffComponent {
    pub fn new(repo_path: &str) -> Self {
        let diffs = get_diff(repo_path).unwrap();
        let len = diffs.len();

        Self {
            diffs,
            state: ListState::default(),
            focused: false,
            style: ComponentTheme::default(),
            path: repo_path.to_string(),
            window: ListWindow::new(0, 0, 0, len, 0),
        }
    }
}

impl DiffComponent {
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, rect: Rect) -> Result<(), Box<dyn Error>> {
        self.window.set_height((f.size().height as usize) - 4);

        let list_items: Vec<ListItem> = self
            .diffs
            .iter()
            .map(|item| {
                let content = match item.origin() {
                    '-' => format!("-{}", item.content()),
                    '+' => format!("+{}", item.content()),
                    _ => item.content().to_string(),
                };
                let text = Span::styled(content, item.style());
                ListItem::new(text)
            })
            .collect();
        let list = TuiList::new(list_items).block(
            Block::default()
                .title(" Diff ")
                .style(self.style.style())
                .borders(Borders::ALL)
                .border_style(self.style.border_style())
                .border_type(BorderType::Rounded),
        );

        f.render_stateful_widget(list, rect, &mut self.state);

        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let path = &self.path;
        let diff = get_diff(path.as_ref())?;
        if diff.len() != self.diffs.len() {
            self.render_diff();
            self.diffs = diff;
            self.window.set_size(self.diffs.len());
        }
        Ok(())
    }

    pub fn handle_event(&mut self, ev: KeyEvent) {
        if !self.focused {
            return;
        }
        match ev.code {
            KeyCode::Char('j') => {
                self.scroll_down(1);
            },
            KeyCode::Char('k') => {
                self.scroll_up(1);
            },
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                let height = self.window.height();
                self.scroll_down(height / 2);
            },
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                let height = self.window.height();
                self.scroll_up(height / 2);
            },
            _ => {}
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

    fn render_diff(&mut self) {
        self.window.reset();
        self.state.select(self.window.position());
    }

    fn scroll_up(&mut self, i: usize) {
        self.window.scroll(ScrollDirection::Up, i);
        self.state.select(self.window.position());
    }

    fn scroll_down(&mut self, i: usize) {
        self.window.scroll(ScrollDirection::Down, i);
        self.state.select(self.window.position());
    }
}