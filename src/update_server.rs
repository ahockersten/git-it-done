use leptos::logging;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;

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
