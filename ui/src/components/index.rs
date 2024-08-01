use crate::components::routing::Route;
pub use dioxus::prelude::*;

pub fn Index() -> Element {
    rsx! {
        div {
            class: "w-full",
            id: "index-container",
            div {
                id: "welcome-message",
                class: "my-4 p-4 w-9/12 bg-forest-1 text-stem-1 drop-shadow-md rounded-sm mx-auto grid grid-cols-2",
                h1 {
                    class: "col-span-2 text-2xl mb-3" ,
                    "Welcome to Mycelial!"
                }
                div {
                    class: "row-span-2 bg-forest-2 px-2 rounded-md",
                    p {
                        class: "my-3 text-stem-2 text-md",
                        "If you're new to Mycelial, you may find it helpful to review some of the following resources:"
                        ul {
                            class: "my-3 ml-2",
                            li {
                                class: "mb-2 hover:text-white",
                                a {
                                    class: "underline",
                                    href: "https://docs.mycelial.com/getting-started/basic-concepts-and-system-overview",
                                    target: "_blank",
                                    "Mycelial Core Concepts and Architecture"
                                }
                            }
                            li {
                                class: "mb-2 hover:text-white",
                                a {
                                    class: "underline",
                                    href: "https://docs.mycelial.com/getting-started/tutorial",
                                    target: "_blank",
                                    "Mycelial Getting Started Tutorial"
                                }
                            }
                            li {
                                class: "mb-2 hover:text-white",
                                a {
                                    class: "underline",
                                    href: "https://www.youtube.com/watch?v=LQCsAdPgVas",
                                    target: "_blank",
                                    "Centralizing Multi-Region Postgres Data with Mycelial"
                                }
                            }
                            li {
                                class: "mb-2 hover:text-white",
                                a {
                                    class: "underline",
                                    href: "https://www.youtube.com/watch?v=qoRvyiqWdEQ&t",
                                    target: "_blank",
                                    "Mycelial Edge Computer Vision Demonstration"
                                }
                            }
                        }
                    }
                }
                div {
                    class: "ml-2 row-span-2 px-2 bg-forest-2 px-2 rounded-md",
                    p {
                        class: "my-3 text-stem-2 text-md",
                        "Ready to dive in?"
                    }
                    p {
                        class: "mb-5",
                        "Create a Workspace and start building pipelines!"
                    }
                    Link{ // TODO: put this button in the middle or the right of the box.
                        class: "border-2 border-stem-1 p-2 rounded-md",
                        to: Route::Workspaces { },
                        children: rsx! { "Go to Workspaces »" }
                    }
                }
            }
        }
    }
}
