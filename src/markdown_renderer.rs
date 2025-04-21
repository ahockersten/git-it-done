use leptos::logging;
use leptos::prelude::event_target_checked;
use leptos::prelude::*;
use markdown::mdast;

use leptos::server::ServerAction;

use crate::update_server::UpdateCheckboxStatus;

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
pub(crate) struct MarkdownRenderer {
    // Ensure this uses the explicitly defined struct
    pub update_action: ServerAction<UpdateCheckboxStatus>,
}

impl MarkdownRenderer {
    pub fn render_node(&self, node: &mdast::Node, file_path: &str) -> AnyView {
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
