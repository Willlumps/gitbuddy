use std::path::Path;

use anyhow::Result;
use git2::{BranchType, Repository};

use crate::git::diff::head;
use crate::git::log::Commit;
use crate::git::repo;

#[derive(Clone, Debug)]
pub struct Branch {
    pub name: String,
    pub branch_type: BranchType,
    pub last_commit: Commit,
}

pub fn checkout_local_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let repo = repo(repo_path)?;

    // Need to change the files in the working directory as well as set the HEAD
    let (object, reference) = repo.revparse_ext(branch_name).expect("Object not found");

    repo.checkout_tree(&object, None)?;

    match reference {
        // gref is an actual reference like branches or tags
        Some(gref) => repo.set_head(gref.name().unwrap()),
        // this is a commit, not a reference
        None => repo.set_head_detached(object.id()),
    }
    .expect("Failed to set HEAD");

    Ok(())
}

pub fn checkout_remote_branch(repo_path: &Path, remote_branch_name: &str) -> Result<()> {
    let repo = repo(repo_path)?;
    let mut remote = remote_branch_name.split('/');
    let remote_name = remote.next().expect("Remote should exist");
    let name = remote.collect::<Vec<&str>>().join("");

    if does_local_branch_exist(&repo, &name) {
        return Err(anyhow::Error::msg("Local branch already exists"));
    }

    let (object, _reference) = repo
        .revparse_ext(remote_branch_name)
        .expect("Object not found");
    let commit = object.as_commit().unwrap();

    repo.branch(&name, commit, false)?;

    let (object, reference) = repo.revparse_ext(&name).expect("Object not found");

    repo.checkout_tree(&object, None)?;

    match reference {
        Some(gref) => repo.set_head(gref.name().unwrap()),
        None => repo.set_head_detached(object.id()),
    }
    .expect("Failed to set HEAD");

    set_upstream_branch(repo_path, remote_name, &name)?;

    Ok(())
}

pub fn delete_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let repo = repo(repo_path)?;
    let mut branch = repo.find_branch(branch_name, BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

pub fn get_branches(repo_path: &Path) -> Result<Vec<Branch>> {
    let repo = repo(repo_path)?;
    let mut branch_list = Vec::new();

    let mut local_branches = repo
        .branches(Some(git2::BranchType::Local))?
        .collect::<Vec<_>>();

    let mut remote_branches = repo
        .branches(Some(git2::BranchType::Remote))?
        .collect::<Vec<_>>();

    local_branches.append(&mut remote_branches);

    for git_branch in local_branches {
        let (branch, branch_type) = git_branch?;
        let reference = branch.get();

        let name = reference
            .shorthand()
            .expect("Branch name is not valid UTF-8");
        let commit = reference.peel_to_commit()?;

        branch_list.push(Branch {
            name: name.to_string(),
            branch_type,
            last_commit: Commit::from_git_commit(commit),
        });
    }
    Ok(branch_list)
}

pub fn branch_from_head(repo_path: &Path, new_branch_name: &str) -> Result<()> {
    let repo = repo(repo_path)?;
    let head = head(repo_path)?;
    let (object, _reference) = repo.revparse_ext(&head).expect("Revspec not found");
    match object.as_commit() {
        Some(commit) => {
            if let Err(err) = repo.branch(new_branch_name, commit, false) {
                return Err(anyhow::Error::from(err));
            }
        }
        None => {
            return Err(anyhow::Error::msg("Object is not a commit"));
        }
    }
    Ok(())
}

pub fn set_upstream_branch(repo_path: &Path, remote_name: &str, branch_name: &str) -> Result<()> {
    let repo = repo(repo_path)?;
    let mut branch = repo.find_branch(branch_name, BranchType::Local)?;

    if branch.upstream().is_err() {
        branch.set_upstream(Some(format!("{}/{}", remote_name, branch_name).as_str()))?;
    }
    Ok(())
}

fn does_local_branch_exist(repo: &Repository, branch_name: &str) -> bool {
    repo.find_branch(branch_name, BranchType::Local).is_ok()
}
