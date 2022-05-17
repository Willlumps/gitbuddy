use crate::components::branchlist::BranchComponent;
use crate::components::diff::DiffComponent;
use crate::components::files::FileComponent;
use crate::components::log::LogComponent;
use crate::components::status::StatusComponent;
use crate::components::{Component, ComponentType};

use anyhow::Result;
use std::path::PathBuf;

pub struct App {
    pub branches: BranchComponent,
    pub logs: LogComponent,
    pub files: FileComponent,
    pub diff: DiffComponent,
    pub status: StatusComponent,
    pub repo_path: PathBuf,
    pub focused_component: ComponentType,
}

impl App {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            branches: BranchComponent::new(),
            logs: LogComponent::new(repo_path.clone()),
            files: FileComponent::new(repo_path.clone()),
            diff: DiffComponent::new(repo_path.clone()),
            status: StatusComponent::new(repo_path.clone()),
            repo_path,
            focused_component: ComponentType::None,
        }
    }

    pub fn update(&mut self) -> Result<()> {
        self.branches.update()?;
        self.diff.update()?;
        self.logs.update()?;
        self.status.update()?;
        self.files.update()?;
        Ok(())
    }

    pub fn focus(&mut self, component: ComponentType) {
        let current_focus = self.focused_component.clone();
        self._focus(current_focus, false);
        self._focus(component, true);
    }

    fn _focus(&mut self, component: ComponentType, focus: bool) {
        match component {
            ComponentType::LogComponent => {
                self.logs.focus(focus);
            }
            ComponentType::DiffComponent => {
                self.diff.focus(focus);
            }
            ComponentType::BranchComponent => {
                self.branches.focus(focus);
            }
            ComponentType::FilesComponent => {
                self.files.focus(focus);
            }
            ComponentType::None => {}
        }

        self.focused_component = component;
    }
}
