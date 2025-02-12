use super::{repository::repo, RepoPath};
use crate::error::Result;
pub use git2_hooks::PrepareCommitMsgSource;
use scopetime::scope_time;

///
#[derive(Debug, PartialEq, Eq)]
pub enum HookResult {
	/// Everything went fine
	Ok,
	/// Hook returned error
	NotOk(String),
}

impl From<git2_hooks::HookResult> for HookResult {
	fn from(v: git2_hooks::HookResult) -> Self {
		match v {
			git2_hooks::HookResult::Ok { .. }
			| git2_hooks::HookResult::NoHookFound => Self::Ok,
			git2_hooks::HookResult::RunNotSuccessful {
				stdout,
				stderr,
				..
			} => Self::NotOk(format!("{stdout}{stderr}")),
		}
	}
}

/// see `git2_hooks::hooks_commit_msg`
pub fn hooks_commit_msg(
	repo_path: &RepoPath,
	msg: &mut String,
) -> Result<HookResult> {
	scope_time!("hooks_commit_msg");

	let repo = repo(repo_path)?;

	Ok(git2_hooks::hooks_commit_msg(&repo, None, msg)?.into())
}

/// see `git2_hooks::hooks_pre_commit`
pub fn hooks_pre_commit(repo_path: &RepoPath) -> Result<HookResult> {
	scope_time!("hooks_pre_commit");

	let repo = repo(repo_path)?;

	Ok(git2_hooks::hooks_pre_commit(&repo, None)?.into())
}

/// see `git2_hooks::hooks_post_commit`
pub fn hooks_post_commit(repo_path: &RepoPath) -> Result<HookResult> {
	scope_time!("hooks_post_commit");

	let repo = repo(repo_path)?;

	Ok(git2_hooks::hooks_post_commit(&repo, None)?.into())
}

/// see `git2_hooks::hooks_prepare_commit_msg`
pub fn hooks_prepare_commit_msg(
	repo_path: &RepoPath,
	source: PrepareCommitMsgSource,
	msg: &mut String,
) -> Result<HookResult> {
	scope_time!("hooks_prepare_commit_msg");

	let repo = repo(repo_path)?;

	Ok(git2_hooks::hooks_prepare_commit_msg(
		&repo, None, source, msg,
	)?
	.into())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sync::tests::repo_init;

	#[test]
	fn test_post_commit_hook_reject_in_subfolder() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();

		let hook = b"#!/bin/sh
	echo 'rejected'
	exit 1
	        ";

		git2_hooks::create_hook(
			&repo,
			git2_hooks::HOOK_POST_COMMIT,
			hook,
		);

		let subfolder = root.join("foo/");
		std::fs::create_dir_all(&subfolder).unwrap();

		let res =
			hooks_post_commit(&subfolder.to_str().unwrap().into())
				.unwrap();

		assert_eq!(
			res,
			HookResult::NotOk(String::from("rejected\n"))
		);
	}

	// make sure we run the hooks with the correct pwd.
	// for non-bare repos this is the dir of the worktree
	// unfortunately does not work on windows
	#[test]
	#[cfg(unix)]
	fn test_pre_commit_workdir() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();
		let workdir =
			crate::sync::utils::repo_work_dir(repo_path).unwrap();

		let hook = b"#!/bin/sh
	echo $(pwd)
	exit 1
	        ";

		git2_hooks::create_hook(
			&repo,
			git2_hooks::HOOK_PRE_COMMIT,
			hook,
		);
		let res = hooks_pre_commit(repo_path).unwrap();
		if let HookResult::NotOk(res) = res {
			assert_eq!(
				std::path::Path::new(res.trim_end()),
				std::path::Path::new(&workdir)
			);
		} else {
			assert!(false);
		}
	}

	#[test]
	fn test_hooks_commit_msg_reject_in_subfolder() {
		let (_td, repo) = repo_init().unwrap();
		let root = repo.path().parent().unwrap();

		let hook = b"#!/bin/sh
	echo 'msg' > $1
	echo 'rejected'
	exit 1
	        ";

		git2_hooks::create_hook(
			&repo,
			git2_hooks::HOOK_COMMIT_MSG,
			hook,
		);

		let subfolder = root.join("foo/");
		std::fs::create_dir_all(&subfolder).unwrap();

		let mut msg = String::from("test");
		let res = hooks_commit_msg(
			&subfolder.to_str().unwrap().into(),
			&mut msg,
		)
		.unwrap();

		assert_eq!(
			res,
			HookResult::NotOk(String::from("rejected\n"))
		);

		assert_eq!(msg, String::from("msg\n"));
	}
}
