use dioxus::prelude::*;

use crate::components::{logo::Logo, routing::Route};

#[component]
pub fn NavBar() -> Element {
    rsx! {
        header {
            div {
                //alternative for height here is min-h-16, which is 64 pixels in height, but that's 2 pixels less than in current console
                class: "flex min-h-[66px] text-stem-1 bg-night-1 select-none",
                div {
                    class: "flex-inital m-h-max ml-8 content-center pr-6",
                    Link{ to: Route::Index{}, Logo{} },
                }
                div {
                    class: "flex-inital m-h-max content-center px-4",
                    Link{ to: Route::Workspaces{}, "Workspaces" },
                }
                div {
                    class: "flex-inital m-h-max content-center px-4",
                    Link{ to: Route::Daemons{}, "Daemons" },
                }
            }
        }
        Outlet::<Route> {}
    }
}
