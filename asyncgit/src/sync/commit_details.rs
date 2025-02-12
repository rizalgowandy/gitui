use super::{commits_info::get_message, CommitId, RepoPath};
use crate::{error::Result, sync::repository::repo};
use git2::Signature;
use scopetime::scope_time;

///
#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub struct CommitSignature {
	///
	pub name: String,
	///
	pub email: String,
	/// time in secs since Unix epoch
	pub time: i64,
}

impl CommitSignature {
	/// convert from git2-rs `Signature`
	pub fn from(s: &Signature<'_>) -> Self {
		Self {
			name: s.name().unwrap_or("").to_string(),
			email: s.email().unwrap_or("").to_string(),

			time: s.when().seconds(),
		}
	}
}

///
#[derive(Default, Clone)]
pub struct CommitMessage {
	/// first line
	pub subject: String,
	/// remaining lines if more than one
	pub body: Option<String>,
}

impl CommitMessage {
	///
	pub fn from(s: &str) -> Self {
		let mut lines = s.lines();
		let subject = lines.next().map_or_else(
			String::new,
			std::string::ToString::to_string,
		);

		let body: Vec<String> =
			lines.map(std::string::ToString::to_string).collect();

		Self {
			subject,
			body: if body.is_empty() {
				None
			} else {
				Some(body.join("\n"))
			},
		}
	}

	///
	pub fn combine(self) -> String {
		if let Some(body) = self.body {
			format!("{}\n{body}", self.subject)
		} else {
			self.subject
		}
	}
}

///
#[derive(Default, Clone)]
pub struct CommitDetails {
	///
	pub author: CommitSignature,
	/// committer when differs to `author` otherwise None
	pub committer: Option<CommitSignature>,
	///
	pub message: Option<CommitMessage>,
	///
	pub hash: String,
}

impl CommitDetails {
	///
	pub fn short_hash(&self) -> &str {
		&self.hash[0..7]
	}
}

///
pub fn get_commit_details(
	repo_path: &RepoPath,
	id: CommitId,
) -> Result<CommitDetails> {
	scope_time!("get_commit_details");

	let repo = repo(repo_path)?;

	let commit = repo.find_commit(id.into())?;

	let author = CommitSignature::from(&commit.author());
	let committer = CommitSignature::from(&commit.committer());
	let committer = if author == committer {
		None
	} else {
		Some(committer)
	};

	let msg =
		CommitMessage::from(get_message(&commit, None).as_str());

	let details = CommitDetails {
		author,
		committer,
		message: Some(msg),
		hash: id.to_string(),
	};

	Ok(details)
}

#[cfg(test)]
mod tests {
	use super::{get_commit_details, CommitMessage};
	use crate::{
		error::Result,
		sync::{
			commit, stage_add_file, tests::repo_init_empty, RepoPath,
		},
	};
	use std::{fs::File, io::Write, path::Path};

	#[test]
	fn test_msg_invalid_utf8() -> Result<()> {
		let file_path = Path::new("foo");
		let (_td, repo) = repo_init_empty().unwrap();
		let root = repo.path().parent().unwrap();
		let repo_path: &RepoPath =
			&root.as_os_str().to_str().unwrap().into();

		File::create(root.join(file_path))?.write_all(b"a")?;
		stage_add_file(repo_path, file_path).unwrap();

		let msg = invalidstring::invalid_utf8("test msg");
		let id = commit(repo_path, msg.as_str()).unwrap();

		let res = get_commit_details(repo_path, id).unwrap();

		assert!(res
			.message
			.as_ref()
			.unwrap()
			.subject
			.starts_with("test msg"));

		Ok(())
	}

	#[test]
	fn test_msg_linefeeds() -> Result<()> {
		let msg = CommitMessage::from("foo\nbar\r\ntest");

		assert_eq!(msg.subject, String::from("foo"),);
		assert_eq!(msg.body, Some(String::from("bar\ntest")),);

		Ok(())
	}

	#[test]
	fn test_commit_message_combine() -> Result<()> {
		let msg = CommitMessage::from("foo\nbar\r\ntest");

		assert_eq!(msg.combine(), String::from("foo\nbar\ntest"));

		Ok(())
	}
}
