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
            _ => None, // Ignore other node types for text extraction for now
        })
        .collect::<String>()
}

// Function to render markdown AST node to Leptos View
fn render_markdown_ast(node: &mdast::Node) -> AnyView {
    match node {
        mdast::Node::Root(root) => {
            let children = root
                .children
                .iter()
                .map(render_markdown_ast)
                .collect::<Vec<_>>();
            view! { <>{children}</> }.into_any()
        }
        mdast::Node::Paragraph(paragraph) => {
            let children = paragraph
                .children
                .iter()
                .map(render_markdown_ast)
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
                _ => view! { <h6>{text}</h6> }.into_any(), // Default to h6 for depth > 5
            }
        }
        mdast::Node::Text(text) => text.value.clone().into_any(),
        // Handle other node types as needed, returning an empty view for now
        _ => view! { <></> }.into_any(),
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> View<impl IntoView> {
    let content = move || -> AnyView {
        let data_dir = match env::var("DATA_DIR") {
            Ok(dir) => dir,
            // Default to "data" if not set
            Err(_) => "data".to_string(),
        };
        let mut path = PathBuf::from(data_dir);
        path.push("Handla.md");

        match fs::read_to_string(&path) {
            Ok(text) => {
                // Parse the markdown text to an AST
                match markdown::to_mdast(&text, &markdown::ParseOptions::default()) {
                    Ok(ast) => render_markdown_ast(&ast),
                    Err(e) => view! { <p>"Error parsing markdown: "{e.to_string()}</p> }.into_any(),
                }
            }
            Err(e) => view! { <p>"Error reading file "{path.to_string_lossy().to_string()}": "{e.to_string()}</p> }.into_any(),
        }
    };

    view! {
        <div>{content()}</div>
    }
}
