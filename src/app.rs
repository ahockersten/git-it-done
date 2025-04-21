use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

fn read_and_convert_markdown(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let html = markdown::to_html(&text);
            // Consider using a more robust HTML parser/manipulator if complexity grows
            let html = html.replace("<li>[ ] ", "<li><input type=\"checkbox\"> ");
            let html = html.replace("<li>[x] ", "<li><input type=\"checkbox\" checked> ");
            Ok(html)
        }
        Err(e) => Err(format!("Error reading file {:?}: {}", path, e)),
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let content = move || -> String {
        let data_dir = match env::var("DATA_DIR") {
            Ok(dir) => dir,
            // Default to "data" if not set
            Err(_) => "data".to_string(),
        };
        let mut path = PathBuf::from(data_dir);
        path.push("Handla.md");

        match read_and_convert_markdown(&path) {
            Ok(html) => html,
            Err(e) => e, // Display the error message directly
        }
    };

    view! {
        <div inner_html=content()></div>
    }
}
