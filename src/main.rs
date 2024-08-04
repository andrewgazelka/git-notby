use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use clap::Parser;
use colored::Colorize;
use git2::{Repository, Tree};

#[derive(Parser, Debug)]
#[clap(
    name = "git-notby",
    about = "Find files not made by a specific user in a Git repo"
)]
struct Args {
    /// The user to exclude
    #[clap(name = "USER")]
    user: String,

    /// The path to the Git repository
    repo_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let repo_path = args
        .repo_path
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let repo = Repository::open(repo_path)?;

    let head = repo.head()?;
    let tree = head.peel_to_tree()?;

    process_tree(&repo, &tree, "", &args.user)?;

    Ok(())
}

fn process_tree(
    repo: &Repository,
    tree: &Tree,
    prefix: &str,
    user: &str,
) -> Result<(), git2::Error> {
    for entry in tree {
        let entry_name = entry.name().unwrap_or("");
        let path = if prefix.is_empty() {
            entry_name.to_string()
        } else {
            format!("{prefix}/{entry_name}")
        };

        let object = entry.to_object(repo)?;

        if let Some(subtree) = object.as_tree() {
            process_tree(repo, subtree, &path, user)?;
        } else if let Some(blob) = object.as_blob() {
            process_file(repo, &path, blob, user)?;
        }
    }

    Ok(())
}

fn process_file(
    repo: &Repository,
    path: &str,
    _blob: &git2::Blob,
    user: &str,
) -> Result<(), git2::Error> {
    let file_path = Path::new(path);
    let blame = repo.blame_file(file_path, None)?;

    let (lines_not_by_user, other_authors) = analyze_blame(&blame, user);

    if lines_not_by_user > 0 {
        print_file_info(path, lines_not_by_user, &other_authors);
    }

    Ok(())
}

fn analyze_blame(blame: &git2::Blame, user: &str) -> (usize, HashSet<String>) {
    let mut lines_not_by_user = 0;
    let mut other_authors = HashSet::new();

    for hunk in blame.iter() {
        let signature = hunk.final_signature();
        let author = signature.name().unwrap_or("Unknown");

        if author != user {
            lines_not_by_user += hunk.lines_in_hunk();
            other_authors.insert(author.to_string());
        }
    }

    (lines_not_by_user, other_authors)
}

fn print_file_info(path: &str, lines_not_by_user: usize, other_authors: &HashSet<String>) {
    println!(
        "{} {} {}",
        path.blue(),
        lines_not_by_user.to_string().red(),
        other_authors
            .iter()
            .map(|author| format!("[{author}]").yellow().to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
}
