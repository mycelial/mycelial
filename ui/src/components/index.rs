pub use dioxus::prelude::*;

pub fn Index() -> Element {
    rsx! {
        div {
            class: "w-full",
            id: "index-container",   
            div {
                id: "welcome-message",
                class: "my-4 p-4 w-9/12 bg-forest-1 text-stem-1 shadow-lg rounded-sm mx-auto",
                h1 {
                    class: "text-2xl" ,
                    "Welcome to Mycelial!"
                }
                p {
                    class: "my-3 text-stem-2 text-md",
                    "If you're new to Mycelial, you may find it helpful to review some of the following resources:"
                    ul {
                        class: "my-3 ml-2",
                        li {
                            class: "mb-2",
                            a {
                                class: "underline",
                                href: "https://docs.mycelial.com/getting-started/basic-concepts-and-system-overview",
                                target: "_blank",
                                "Mycelial Core Concepts and Architecture"
                            }
                        }
                        li {
                            class: "mb-2",
                            a {
                                class: "underline",
                                href: "https://docs.mycelial.com/getting-started/tutorial",
                                target: "_blank",
                                "Mycelial Getting Started Tutorial"
                            }
                        }
                        li {
                            class: "mb-2",
                            a {
                                class: "underline",
                                href: "https://www.youtube.com/watch?v=LQCsAdPgVas",
                                target: "_blank",
                                "Centralizing Multi-Region Postgres Data with Mycelial"
                            }
                        }
                        li {
                            class: "mb-2",
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
                class: "my-2 p-4 w-9/12 bg-moss-3 text-black shadow-lg rounded-sm mx-auto",
                id: "daemon-install-instructions",
                p {
                    "To add your local daemon to this Mycelial network, simply install the
                    daemon using the instructions found "
                    a {
                        class: "underline",
                        href: "https://docs.mycelial.com/getting-started/CLI/",
                        target: "_blank",
                        "here"
                    }
                    " and copied below."
                }
            }
        }
        // <CommandLineTabs token={token} />
    }
}
