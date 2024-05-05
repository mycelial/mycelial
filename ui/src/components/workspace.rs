use dioxus::prelude::*;
use std::collections::HashMap;

// Simple graph
#[derive(Debug)]
struct Graph {
    nodes: HashMap<u64, Signal<NodeState>>,
    // FIXME:
    edges: (),
    counter: u64,
}

impl Graph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: (),
            counter: 0,
        }
    }

    fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }

    fn add_node(&mut self, id: u64, node: Signal<NodeState>) {
        self.nodes.insert(id, node);
    }

    fn remove_node(&mut self, id: u64) {
        self.nodes.remove(&id);
    }
}

// Node of the graph
//
// Each node has:
// 1. unique Id - UUID (for now)
// 2. set of coordinates
#[derive(Debug)]
struct NodeState {
    id: u64,
    node_type: &'static str,
    x: f64,
    y: f64,
}

impl NodeState {
    fn new(id: u64, node_type: &'static str, x: f64, y: f64) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
        }
    }
}

#[derive(Debug)]
struct ViewPortState {
    x: f64,
    y: f64,
}

impl ViewPortState {
    fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

// representation of section in sections menu, which can be dragged into viewport container
#[component]
fn MenuItem(mut currently_dragged: Signal<Option<&'static str>>, id: &'static str) -> Element {
    rsx! {
        div {
            class: "min-w-32 min-h-24 border border-solid rounded grid grid-flow-rows p-2",
            draggable: true,
            ondragstart: move |_event| {
                *currently_dragged.write() = Some(id);
                tracing::info!("dragged: {id}")
            },
            ondragend: move |_event| {
                *currently_dragged.write() = None;
                tracing::info!("drag end");
            },
            div {
                class: "grid grid-flow-col",
                p {
                    class: "uppercase inline",
                    "name of section with ID {id}"
                }
                span {
                    class: "justify-self-end",
                    // TODO: change background to grey if not source
                    span {
                        class: "bg-moss-1 text-night-1 rounded-full p-1 ml-1",
                        "Source"
                    }
                    // TODO: change background to grey if not dest
                    span {
                        class: "bg-forest-2 text-stem-2 rounded-full p-1 ml-1",
                        "Dest"
                    }
                }
            }
            div {
                "Type: Type of connector goes here {id}"
            }
            div {
                "Daemon: Name of daemon goes here {id}"
            }
        }
    }
}

// section in viewport
#[component]
fn Node(id: u64, node: Signal<NodeState>) -> Element {
    let (node_type, x, y) = {
        let node = &*node.read();
        (node.node_type, node.x, node.y)
    };
    let mut delta = use_signal(|| (0.0, 0.0));
    rsx! {
        div {
            class: "absolute min-w-32 min-h-24 border border-solid text-center content-center",
            //style: format!("transform: translate({x}px, {y}px)"),
            style: format!("left: {x}px; top: {y}px;"),
            draggable: true,
            ondragstart: move |event| {
                // delta between position of cursors and grabbing spot to compensate drag
                let coords = event.client_coordinates();
                let node = &*node.read();
                *delta.write() = (coords.x - node.x, coords.y - node.y);
                tracing::info!("delta: {delta:?}")
            },
            ondrag: move |event| {
                let node = &mut *node.write();
                let coords = event.client_coordinates();
                let (delta_x, delta_y) = *delta.read();
                node.x = coords.x - delta_x;
                node.y = coords.y - delta_y;
               // tracing::info!("{}/{} -> {}/{}", old_x, old_y, node.x, node.y);
            },
            ondragend: move |event| {
                tracing::info!("delta: {:?}", *delta.read());
                let node = &mut *node.write();
                let coords = event.client_coordinates();
                let (delta_x, delta_y) = *delta.read();
                node.x = coords.x - delta_x;
                node.y = coords.y - delta_y;
            },
            "id: {id}, node_type: {node_type}"
        }
    }
}

// TODO: minimap
#[component]
fn ViewPort(
    view_port_state: Signal<ViewPortState>,
    dragged: Signal<Option<&'static str>>,
    graph: Signal<Graph>,
) -> Element {
    let nodes = &graph.read().nodes;
    let mut grabbed = use_signal(|| false);
    let _icon = if *grabbed.read() {
        "cursor: grabbing;"
    } else {
        "cursor: grab"
    };
    rsx! {
        div {
            class: "min-h-screen bg-grey-bright overflow-hidden",
            // FIXME: move to class or smth
         // style: format!(r#"
         //     opacity: 0.3;
         //     background-image:  linear-gradient(#444cf7 1px, transparent 1px), linear-gradient(to right, #444cf7 1px, #e5e5f7 1px);
         //     background-size: 20px 20px;
         //     {icon}
         // "#),
         //style: icon,

            // prevent_default + own ondragover enable drop area on current container
            prevent_default: "ondragover",
            ondragover: move |_event| {},

            ondrop: move |event| {
                if let Some(node_type) = *dragged.read() {
                    let graph = &mut*graph.write();
                    let id = graph.get_id();
                    let coords = event.client_coordinates();
                    let node_state = Signal::new(NodeState::new(id, node_type, coords.x, coords.y));
                    graph.add_node(id, node_state);
                }
                *dragged.write() = None;
            },

            // panning funcs
            onmousedown: move |_event| {
                *grabbed.write() = true;
            },
            onmousemove: move |_event| {
             // if *grabbed.read() {
             // }
            },
            onmouseup: move |_event| {
                *grabbed.write() = false;
            },

            for (&id, &node) in nodes.iter() {
                Node{ id: id, node: node }
            }
        }
    }
}

#[component]
pub fn Workspace(workspace: String) -> Element {
    // this i
    let currently_dragged = use_signal(|| None);

    // placeholder for sections types in section menu
    let menu_items = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];

    // graph, represents set of placed nodes/edges in view port
    let graph = use_signal(Graph::new);

    // viewport state, current coordinates, scale, etc, synced with backend.
    // TODO:
    let view_port_state = use_signal(ViewPortState::new);

    rsx! {
        div {
            // TODO: implement if/then logic such that 3rd column appears (with node details) when node selected
            class: "grid",
            style: "grid-template-columns: 35% 65%;", // exception to Tailwind only bc TW doesn't have classes to customize column widths
            div {
                class: "col-span-2 pl-2 py-4 text-stem-1 bg-night-2",
                h1 {
                    class: "text-lg",
                    "Workspace: {workspace}"
                }
            }
            // section menu
            div {
                class: "border border-solid overflow-y-scroll bg-white grid grid-flow-rows gap-y-3 md:px-2",
                h2 {
                    class: "mt-3",
                    "Pipeline Sections"
                }
                for id in menu_items.iter() {
                    MenuItem { currently_dragged: currently_dragged, id: id }
                }
            }
            // viewport
            div {
                class: "",
                ViewPort { view_port_state: view_port_state, dragged: currently_dragged, graph: graph }
            }
        }
    }
}
