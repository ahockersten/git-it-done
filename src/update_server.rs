use git2::{Commit, IndexAddOption, Repository, Signature, Time};
use leptos::logging;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

// Without this the size of the arguments given to update_checkbox_status
// are not known at compile time
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateCheckboxStatusArgs {
    file_path: String,
    line_number: usize,
    is_checked: bool,
}

#[server(UpdateCheckboxStatus)]
async fn update_checkbox_status(
    file_path: String,
    line_number: usize,
    is_checked: bool,
) -> Result<(), ServerFnError> {
    use std::fs::OpenOptions;
    use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
    use std::path::PathBuf;

    // Construct the full path relative to the server execution context
    // Ensure this logic matches how the file path is determined in HomePage
    let base_dir = env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let mut path = PathBuf::from(base_dir);
    // Use the relative path passed from the client
    path.push(&file_path);

    logging::log!(
        "Attempting to update file: {:?}, line: {}, checked: {}",
        path,
        line_number,
        is_checked
    );

    // Basic security check: Ensure the path stays within the data directory
    // This is a simplified check; a production app might need more robust validation.
    match (
        std::fs::canonicalize(PathBuf::from(
            env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()),
        )),
        std::fs::canonicalize(&path),
    ) {
        (Ok(canonical_data_dir), Ok(canonical_path)) => {
            if !canonical_path.starts_with(canonical_data_dir) {
                logging::error!("Invalid file path requested: {:?}", path);
                return Err(ServerFnError::ServerError(
                    "Invalid file path requested".into(),
                ));
            }
        }
        (Err(e), _) => {
            logging::error!("Failed to canonicalize data directory path: {}", e);
            return Err(ServerFnError::ServerError(format!(
                "Data directory path validation failed: {}",
                e
            )));
        }
        (_, Err(e)) => {
            logging::error!("Failed to canonicalize file path ({:?}): {}", path, e);
            return Err(ServerFnError::ServerError(format!(
                "File path validation failed for {:?}: {}",
                path, e
            )));
        }
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| -> ServerFnError {
            ServerFnError::ServerError(format!("Failed to open file {:?}: {}", path, e))
        })?;

    let mut lines = BufReader::new(&file)
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| -> ServerFnError {
            ServerFnError::ServerError(format!("Failed to read lines from file {:?}: {}", path, e))
        })?;

    if line_number > 0 && line_number <= lines.len() {
        let line_index = line_number - 1; // Adjust to 0-based index
        let line = &mut lines[line_index];
        let original_line = line.clone(); // Keep original for logging

        let mut modified = false;
        // Find the checkbox pattern and update it
        // Ensure we handle potential leading whitespace correctly
        let trimmed_line = line.trim_start();
        if trimmed_line.starts_with("- [ ] ") {
            if is_checked {
                if let Some(pos) = line.find("- [ ] ") {
                    // Find the actual position in the original line
                    line.replace_range(pos..(pos + 6), "- [x] ");
                    modified = true;
                }
            }
        } else if trimmed_line.starts_with("- [x] ") {
            if !is_checked {
                if let Some(pos) = line.find("- [x] ") {
                    // Find the actual position in the original line
                    line.replace_range(pos..(pos + 6), "- [ ] ");
                    modified = true;
                }
            }
        } else {
            logging::warn!(
                "Line {} in {:?} does not contain a checkbox pattern: {}",
                line_number,
                path,
                original_line
            );
            return Err(ServerFnError::ServerError(format!(
                "Line {} is not a checkbox item",
                line_number
            )));
        }

        if modified {
            file.seek(SeekFrom::Start(0))?;
            file.set_len(0)?; // Truncate the file
            for updated_line in lines {
                writeln!(file, "{}", updated_line).map_err(|e| -> ServerFnError {
                    ServerFnError::ServerError(format!(
                        "Failed to write line to file {:?}: {}",
                        path, e
                    ))
                })?;
            }
            logging::log!("File updated successfully: {:?}", path);

            // Commit the changes using git2
            let repo_path_str = env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
            let repo_path = Path::new(&repo_path_str);
            match commit_changes(repo_path, &file_path, line_number, is_checked) {
                Ok(_) => logging::log!("Changes committed successfully for {:?}", path),
                Err(e) => {
                    // Log the error but don't necessarily fail the whole operation,
                    // as the file write was successful.
                    logging::error!("Failed to commit changes for {:?}: {}", path, e);
                    // Optionally, return a specific error or handle it differently
                    // For now, we just log it.
                    // return Err(ServerFnError::ServerError(format!("Failed to commit changes: {}", e)));
                }
            }
        } else {
            logging::log!("No change needed for line {} in {:?}", line_number, path);
        }
    } else {
        logging::error!("Invalid line number {} for file {:?}", line_number, path);
        return Err(ServerFnError::ServerError(format!(
            "Invalid line number: {}",
            line_number
        )));
    }

    Ok(())
}

// Helper function to commit changes using git2
fn commit_changes(
    repo_path: &Path,
    file_path_relative: &str,
    line_number: usize,
    is_checked: bool,
) -> Result<(), git2::Error> {
    // Open the repository
    let repo = Repository::open(repo_path)?;
    logging::log!("Opened git repository at: {:?}", repo_path);

    // Stage the file
    let mut index = repo.index()?;
    index.add_path(Path::new(file_path_relative))?;
    index.write()?; // Write the index changes to disk
    let oid = index.write_tree()?; // Write the tree object
    logging::log!("Staged file: {}", file_path_relative);

    // Create the commit
    let tree = repo.find_tree(oid)?;
    let parent_commit = find_head_commit(&repo)?;
    let signature = Signature::now("Git Note Taking App", "app@example.com")?; // TODO: Configure user/email

    let message = format!(
        "Update checkbox status in {} line {}: {}",
        file_path_relative,
        line_number,
        if is_checked { "checked" } else { "unchecked" }
    );

    repo.commit(
        Some("HEAD"),      // Point HEAD to our new commit
        &signature,        // Author
        &signature,        // Committer
        &message,          // Commit message
        &tree,             // Tree
        &[&parent_commit], // Parent commit
    )?;

    logging::log!("Committed changes with message: {}", message);

    Ok(())
}

// Helper function to find the HEAD commit
fn find_head_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
    obj.into_commit()
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}
