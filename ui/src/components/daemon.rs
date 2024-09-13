use crate::components::card::Card;
use dioxus::prelude::*;

#[component]
pub fn Daemon(daemon: String) -> Element {
    let status = "HEALTHY";

    rsx! {
        div {
            class: "container mx-auto grid grid-cols-2 gap-3",
            div {
                class: "pt-5 font-bold self-center",
                h2 {
                    class: "text-xl",
                    "Daemon {daemon}"
                }
            }
            div {
                class: "pt-5 pr-3 justify-self-end self-center",
                if status == "HEALTHY" {
                    div {
                        class: "bg-moss-3 border border-forest-2 text-forest-1 px-4 py-3 rounded relative",
                        role: "alert",
                        strong {
                            class: "font-bold",
                            "Healthy"
                        }
                    }
                } else {
                    div {
                        class: "bg-toadstool-3 border border-toadstool-2 text-toadstool-1 px-4 py-3 rounded relative",
                        role: "alert",
                        strong {
                            class: "font-bold",
                            "Degraded"
                        }
                    }
                }

            }
            div {
                class: "drop-shadow justify-self-center",
                Card {
                    title: "Name: {daemon}",
                    subtitle: "ID: ID-1000001",
                    content: "Address: 100.100.121.883",
                }
            }
            div {
                class: "drop-shadow justify-self-center",
                Card {
                    title: "Last Seen: 2024-04-11",
                    subtitle: "Last Updated: 2024-04-10",
                    content: "Created At: 2023-01-12",
                }
            }
            div {
                class: "drop-shadow justify-self-center",
                Card {
                    title: "Current Pipelines: 4",
                    subtitle: "Current Sections: 8",
                    content: "Current Transforms: 2",
                }
            }
            div {
                class: "drop-shadow justify-self-stretch mx-auto",
                Card {
                    title: "myceliald version: 0.18.22",
                    subtitle: "Control Plane version: 0.18.22",
                    content: "Control Plane Address: 100.1.1.33",
                }
            }
        }
    }
}
