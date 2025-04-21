use leptos::logging;
use leptos::prelude::event_target_checked;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use markdown::mdast;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize}; // Import Serialize and Deserialize

// Explicitly define the struct for the server function arguments
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateCheckboxStatus {
    file_path: String,
    line_number: usize,
    is_checked: bool,
}


#[server(UpdateCheckboxStatus)] // Reference the explicit struct
async fn update_checkbox_status(
    file_path: String,
    line_number: usize,
    is_checked: bool,
) -> Result<(), ServerFnError> {
    use std::fs::OpenOptions;
    use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};

    // Construct the full path relative to the server execution context
    // Ensure this logic matches how the file path is determined in HomePage
    let base_dir = env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let mut path = PathBuf::from(base_dir);
    path.push(&file_path); // Use the relative path passed from the client

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
        // Handle cases where canonicalization fails
        (Err(e), _) => { // Error canonicalizing data dir
            logging::error!(
                "Failed to canonicalize data directory path: {}",
                e
            );
            return Err(ServerFnError::ServerError(format!(
                "Data directory path validation failed: {}",
                e
            )));
        }
        (_, Err(e)) => { // Error canonicalizing file path
            logging::error!(
                "Failed to canonicalize file path ({:?}): {}",
                path,
                e
            );
            return Err(ServerFnError::ServerError(format!(
                "File path validation failed for {:?}: {}",
                path, e
            )));
        }
    }

    // Open the file for reading and writing
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| -> ServerFnError { // Specify the closure's return type
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
                    line.replace_range(pos..(pos + 5), "- [x] ");
                    modified = true;
                }
            }
        } else if trimmed_line.starts_with("- [x] ") {
            if !is_checked {
                if let Some(pos) = line.find("- [x] ") {
                    // Find the actual position in the original line
                    line.replace_range(pos..(pos + 5), "- [ ] ");
                    modified = true;
                }
            }
        } else {
            // Line doesn't seem to be a checkbox item, log and ignore for now
            logging::warn!(
                "Line {} in {:?} does not contain a checkbox pattern: {}",
                line_number,
                path,
                original_line
            );
            // Optionally return an error here if strict matching is required
            // return Err(ServerFnError::ServerError(format!("Line {} is not a checkbox item", line_number)));
        }

        if modified {
            // Write the modified lines back to the file
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


#[server]
async fn get_markdown_content(relative_path: String) -> Result<(String, String), ServerFnError> {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use leptos::logging;

    // Construct the full path relative to the server execution context
    let base_dir = env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let mut path = PathBuf::from(base_dir);
    path.push(&relative_path);

    logging::log!("Attempting to read file: {:?}", path);

    // Basic security check: Ensure the path stays within the data directory
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

    // Read the file content
    match fs::read_to_string(&path) {
        Ok(content) => Ok((relative_path, content)), // Return relative path and content
        Err(e) => {
            logging::error!("Error reading file {:?}: {}", path, e);
            Err(ServerFnError::ServerError(format!(
                "Error reading file {}: {}",
                path.to_string_lossy(),
                e
            )))
        }
    }
}


pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/git-note-taking.css"/>

        // sets the document title
        <Title text="Git note taking"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

// Helper function to extract text from a slice of AST nodes
fn get_text_from_children(children: &[mdast::Node]) -> String {
    children
        .iter()
        .filter_map(|child| match child {
            mdast::Node::Text(text) => Some(text.value.clone()),
            _ => None,
        })
        .collect::<String>()
}

// Helper struct to manage rendering state and recursion
struct MarkdownRenderer {
    // Ensure this uses the explicitly defined struct
    update_action: ServerAction<UpdateCheckboxStatus>,
}

impl MarkdownRenderer {
    // Recursive function to render markdown AST node to Leptos View
    fn render_node(&self, node: &mdast::Node, file_path: &str) -> AnyView {
        match node {
            mdast::Node::Root(root) => {
                let children = root
                    .children
                    .iter()
                    .map(|n| self.render_node(n, file_path)) // Pass file_path down
                    .collect::<Vec<_>>();
                view! { <>{children}</> }.into_any()
            }
            mdast::Node::Paragraph(paragraph) => {
                let children = paragraph
                    .children
                    .iter()
                    .map(|n| self.render_node(n, file_path))
                    .collect::<Vec<_>>();
                view! { <p>{children}</p> }.into_any()
            }
            mdast::Node::Heading(heading) => {
                let text = get_text_from_children(&heading.children);
                match heading.depth {
                    1 => view! { <h1>{text}</h1> }.into_any(),
                    2 => view! { <h2>{text}</h2> }.into_any(),
                    3 => view! { <h3>{text}</h3> }.into_any(),
                    4 => view! { <h4>{text}</h4> }.into_any(),
                    5 => view! { <h5>{text}</h5> }.into_any(),
                    _ => view! { <h6>{text}</h6> }.into_any(),
                }
            }
            mdast::Node::List(list) => {
                let children = list
                    .children
                    .iter()
                    .map(|n| self.render_node(n, file_path))
                    .collect::<Vec<_>>();
                if list.ordered {
                    view! { <ol>{children}</ol> }.into_any()
                } else {
                    view! { <ul>{children}</ul> }.into_any()
                }
            }
            mdast::Node::ListItem(list_item) => {
                let line_number = list_item.position.as_ref().map(|p| p.start.line);
                // Clone file_path only when needed for the event handler closure
                let file_path_owned = file_path.to_string();

                let children = list_item
                    .children
                    .iter()
                    .map(|n| self.render_node(n, &file_path_owned)) // Pass owned string slice
                    .collect::<Vec<_>>();

                let check_item = match list_item.checked {
                    Some(checked_state) => {
                        // Clone action for the closure
                        let update_action = self.update_action.clone();
                        let on_change = move |ev: web_sys::Event| {
                            if let Some(ln) = line_number {
                                let is_checked = event_target_checked(&ev);
                                logging::log!(
                                    "Checkbox changed: line {}, checked: {}",
                                    ln,
                                    is_checked
                                );
                                update_action.dispatch(UpdateCheckboxStatus {
                                    file_path: file_path_owned.clone(), // Use owned path
                                    line_number: ln,
                                    is_checked,
                                });
                            } else {
                                logging::error!(
                                    "Checkbox change event on list item without line number"
                                );
                            }
                        };
                        Some(view! { <input type="checkbox" checked=checked_state on:change=on_change /> }.into_any())
                    }
                    None => None,
                };
                view! { <li>{check_item}{children}</li> }.into_any()
            }
            mdast::Node::Text(text) => text.value.clone().into_any(),
            _ => {
                // Use type_name_of_val for logging the type, or simply remove the specific type logging
                logging::log!("Unhandled node type: {}", std::any::type_name_of_val(node));
                view! { <></> }.into_any()
            }
        }
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Action to update checkbox status on the server
    // Ensure this uses the explicitly defined struct
    let update_action = ServerAction::<UpdateCheckboxStatus>::new();

    // Signal to hold the current AST, re-parsed on action success
    // We need to re-read and re-parse the file when the action completes
    // to reflect the change visually.
    // Use a Resource that calls the server function to fetch content.
    let file_content_resource: Resource<_, Result<(String, String), ServerFnError>> = Resource::new(
        move || update_action.version().get(), // Re-run when action version changes
        |_| async move {
            // Always call the server function to get content
            get_markdown_content("Handla.md".to_string()).await
        },
    );

    view! {
         <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            { move || file_content_resource.get().map(|server_result| {
                // server_result is Result<(String, String), ServerFnError>
                match server_result {
                    Ok(Ok((file_path, content))) => { // Nested Result: Resource -> ServerFn
                        // Parse the markdown content received from the server
                        match markdown::to_mdast(&content, &markdown::ParseOptions::gfm()) {
                            Ok(ast) => {
                                // Create the renderer instance inside the reactive scope
                                let renderer = MarkdownRenderer {
                                    update_action: update_action.clone(), // Clone action for the renderer
                                };
                                // Call the rendering method
                                renderer.render_node(&ast, &file_path).into_any()
                            }
                            Err(e) => view! { <p>"Error parsing markdown: "{e.to_string()}</p> }.into_any(),
                        }
                    },
                    Ok(Err(e)) | Err(e) => { // Handle ServerFnError or Resource loading error
                         // Log the error for debugging
                         logging::error!("Error fetching/reading file: {:?}", e);
                         // Display a user-friendly error message
                         view! { <p>"Error loading content: "{e.to_string()}</p> }.into_any()
                    }
                }
            })}
         </Suspense>
         // Display pending state and errors from the action
         <Show when=move || update_action.pending().get()>
             <p>"Updating..."</p>
         </Show>
         {move || update_action.value().get().map(|res| {
             match res {
                 Ok(_) => view! { <p style="display:none">"Update successful"</p> }.into_any(), // Hide success message or show briefly
                 Err(e) => view! { <p style="color: red">"Error updating: "{e.to_string()}</p> }.into_any(),
             }
         })}
    }
}
