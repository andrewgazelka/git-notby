use std::collections::HashSet;
use std::path::PathBuf;
use clap::Parser;
use git2::{Repository, BlameOptions, Oid};
use colored::*;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[clap(name = "git-notby", about = "Find files not made by a specific user in a Git repo")]
struct Args {
    /// The user to exclude
    #[clap(name = "USER")]
    user: String,

    /// The path to the Git repository
    repo_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let repo_path = args.repo_path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let repo = Repository::open(repo_path)?;

    for entry in WalkDir::new(repo.workdir().unwrap())
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path().strip_prefix(repo.workdir().unwrap())?;
        let relative_path = file_path.to_str().unwrap();

        if let Ok(blame) = repo.blame_file(file_path, None) {
            let mut lines_not_by_user = 0;
            let mut other_authors = HashSet::new();

            for hunk in blame.iter() {
                let signature = hunk.final_signature();
                let author = signature.name().unwrap_or("Unknown");

                if author != args.user {
                    lines_not_by_user += hunk.lines_in_hunk();
                    other_authors.insert(author.to_string());
                }
            }

            if lines_not_by_user > 0 {
                println!(
                    "{} {} {}",
                    relative_path.blue(),
                    lines_not_by_user.to_string().red(),
                    other_authors
                        .iter()
                        .map(|author| format!("[{}]", author).yellow().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        }
    }

    Ok(())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}