use std::path::Path;

use crossbeam::channel::Sender;
use git2::string_array::StringArray;
use git2::{Cred, PushOptions, RemoteCallbacks};

use crate::error::Error;
use crate::git::diff::head;
use crate::git::repo;

use super::branch::set_upstream_branch;

pub fn add_remote(repo_path: &Path, name: &str, url: &str) -> Result<(), Error> {
    let repo = repo(repo_path)?;
    repo.remote(name, url)?;

    Ok(())
}

pub fn get_remotes(repo_path: &Path) -> Result<StringArray, Error> {
    let repo = repo(repo_path)?;
    let remotes = repo.remotes()?;
    Ok(remotes)
}

pub fn push(repo_path: &Path, progress_sender: Sender<i8>, remote: &str) -> Result<(), Error> {
    let repo = repo(repo_path)?;

    let mut callbacks = RemoteCallbacks::new();
    let mut remote = repo.find_remote(remote)?;

    // TODO: This sometimes fails credential check and loop indefinitely
    callbacks.credentials(|_url, username_from_url, allowed_types| {
        if allowed_types.is_ssh_key() {
            match username_from_url {
                Some(username) => Cred::ssh_key_from_agent(username),
                None => Err(git2::Error::from_str("Where da username??")),
            }
        } else if allowed_types.is_user_pass_plaintext() {
            // Do people actually use plaintext user/pass ??
            unimplemented!();
        } else {
            Cred::default()
        }
    });

    callbacks.push_transfer_progress(|current, total, _bytes| {
        if let Some(percentage) = current.checked_div(total) {
            progress_sender
                .send((percentage * 100) as i8)
                .expect("Send failed");
        } else {
            progress_sender.send(100).expect("Send failed");
        }
    });

    callbacks.push_update_reference(|_remote, _status| {
        // TODO
        if _status.is_some() {
            panic!("oh no {}", _status.unwrap());
        }
        Ok(())
    });

    let mut options = PushOptions::new();
    let head = head(repo_path)?;
    let refspec = format!("refs/heads/{}", head);

    options.remote_callbacks(callbacks);
    remote.push(&[refspec], Some(&mut options))?;

    set_upstream_branch(repo_path, "origin", "master")?;

    Ok(())
}