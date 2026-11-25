//! Git operations. Like GitHub Desktop, this shells out to the `git` CLI instead of using a library,
//! so the user's config, hooks, credential helpers and SSH keys all work unchanged.

pub mod branches;
pub mod changes;
pub mod history;
pub mod parse;
pub mod remote;
pub mod repository;
pub mod runner;
pub mod stash;
pub mod tags;

pub use runner::{GitError, GitResult};
