use leptos::logging;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use std::env;

use crate::markdown_renderer::MarkdownRenderer;
use crate::update_server::UpdateCheckboxStatus;

#[server]
async fn get_markdown_content(relative_path: String) -> Result<(String, String), ServerFnError> {
    use leptos::logging;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

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

    match fs::read_to_string(&path) {
        Ok(content) => Ok((relative_path, content)),
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
        <Stylesheet id="leptos" href="/pkg/git-it-done.css"/>

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
    let file_content_resource: Resource<std::result::Result<(String, String), ServerFnError>> =
        Resource::new(
            move || update_action.version().get(),
            |_| async move {
                // Always call the server function to get content
                get_markdown_content("Handla.md".to_string()).await
            },
        );

    view! {
         <Suspense fallback=|| view! { <p>"Loading..."</p> }>
            { move || file_content_resource.get().map(|server_result: Result<(String, String), ServerFnError>| {
                match server_result {
                    Ok((file_path, content)) => {
                        match markdown::to_mdast(&content, &markdown::ParseOptions::gfm()) {
                            Ok(ast) => {
                                let renderer = MarkdownRenderer {
                                    update_action: update_action.clone()
                                };
                                renderer.render_node(&ast, &file_path).into_any()
                            }
                            Err(e) => view! { <p>"Error parsing markdown: "{e.to_string()}</p> }.into_any(),
                        }
                    },
                    Err(e) => {
                         logging::error!("Error fetching/reading file: {:?}", e);
                         view! { <p>"Error loading content: "{e.to_string()}</p> }.into_any()
                    }
                }
            })}
         </Suspense>
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
