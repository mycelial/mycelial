use dioxus::prelude::*;

use crate::components::logo::Logo;

pub fn NavBar() -> Element {
    rsx! {
        header {
            div {
                class: "flex min-h-[66px] text-white", //alternative for height here is min-h-16, which is 64 pixels in height, but that's 2 pixels less than in current console
                style: "background-color:rgb(18,24,88)",
                div {
                    class: "flex-none m-h-max ml-8 content-center pr-6",
                    Logo{},
                }
                div {
                    class: "flex-initial m-h-max content-center px-4",
                    "Workspaces"
                }
                div {
                    class: "flex-inital m-h-max content-center px-4",
                    "Daemons"
                }
            }
        }
    }
}
