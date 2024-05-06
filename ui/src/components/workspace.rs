use dioxus::prelude::*;
use std::collections::BTreeMap;

// Simple graph
#[derive(Debug)]
struct Graph {
    nodes: BTreeMap<u64, Signal<NodeState>>,
    edges: BTreeMap<u64, u64>,
    counter: u64,
}

impl Graph {
    fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            counter: 0,
        }
    }

    fn get_id(&mut self) -> u64 {
        let id = self.counter;
        self.counter += 1;
        id
    }

    fn add_node(&mut self, id: u64, node: Signal<NodeState>) {
        self.add_edge(0, id);
        self.nodes.insert(id, node);
    }

    fn remove_node(&mut self, id: u64) {
        self.nodes.remove(&id);
    }

    fn add_edge(&mut self, _from_node: u64, _to_node: u64) {}

    fn remove_edge(&mut self, _from_node: u64) {}
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
    w: f64,
    h: f64,
    port_diameter: f64,
}

impl NodeState {
    fn new(id: u64, node_type: &'static str, x: f64, y: f64) -> Self {
        Self {
            id,
            node_type,
            x,
            y,
            w: 0.0,
            h: 0.0,
            port_diameter: 12.0,
        }
    }

    fn input_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset, self.y + self.h / 2.0 - offset)
    }

    fn output_pos(&self) -> (f64, f64) {
        let offset = self.port_diameter / 2.0;
        (self.x - offset + self.w, self.y + self.h / 2.0 - offset)
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
fn MenuItem(
    mut currently_dragged: Signal<Option<CurrentlyDragged>>,
    node_type: &'static str,
) -> Element {
    let mut rect_data = use_signal(|| (0.0, 0.0, 0.0, 0.0));
    rsx! {
        div {
            class: "min-w-32 min-h-24 border border-solid rounded grid grid-flow-rows p-2",
            draggable: true,
            onmounted: move |event| {
                spawn(async move {
                    match event.get_client_rect().await {
                        Ok(rect) => {
                            let (x, y) = (rect.origin.x, rect.origin.y);
                            let (w, h) = (rect.size.width, rect.size.height);
                            *rect_data.write() = (x, y, w, h)
                        },
                        Err(e) => tracing::error!("failed to read rect data: {e}"),
                    }
                });
            },
            ondragstart: move |event| {
                let (x, y, _, _) = *rect_data.read();
                let coords = event.client_coordinates();
                let (delta_x, delta_y) = (coords.x - x, coords.y - y);
                *currently_dragged.write() = Some(CurrentlyDragged::new(node_type, delta_x, delta_y));
            },
            ondragend: move |_event| {
                *currently_dragged.write() = None;
            },
            div {
                class: "grid grid-flow-col",
                p {
                    class: "uppercase inline",
                    "name of section with ID {node_type}"
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
                "Type: Type of connector goes here {node_type}"
            }
            div {
                "Daemon: Name of daemon goes here {node_type}"
            }
        }
    }
}

// section in viewport
#[component]
fn Node(id: u64, graph: Signal<Graph>, node: Signal<NodeState>) -> Element {
    // current node coordinates, coordinates on input and output ports
    let (node_type, x, y, w, _h, port_diameter, input_pos, output_pos) = {
        let node = &*node.read();
        (
            node.node_type,
            node.x,
            node.y,
            node.w,
            node.h,
            node.port_diameter,
            node.input_pos(),
            node.output_pos(),
        )
    };
    // state for tracking delta between cursor and top-left corner to adjust element position when dragging
    let mut delta = use_signal(|| (0.0, 0.0));
    let mut grabbed = use_signal(|| false);

    rsx! {
        div {
            class: "absolute min-w-32 min-h-24 border border-solid text-center content-center select-none",
            style: "left: {x}px; top: {y}px;",
            // recalculate positions on input/output nodes
            onmounted: move |event| {
                spawn(async move {
                    match event.get_client_rect().await {
                        Ok(rect) => {
                            let (x, y) = (rect.origin.x, rect.origin.y);
                            let (w, h) = (rect.size.width, rect.size.height);
                            let node = &mut* node.write();
                            node.x = x;
                            node.y = y;
                            node.w = w;
                            node.h = h;
                        },
                        Err(e) => tracing::error!("failed to read rect data: {e}"),
                    }
                });
            },
            onmousedown: move |event| {
                let coords = event.client_coordinates();
                let node = &*node.read();
                *delta.write() = (coords.x - node.x, coords.y - node.y);
                *grabbed.write() = true;
            },
            onmousemove: move |event| {
                if !*grabbed.read() {
                    return
                }
                let node = &mut *node.write();
                let coords = event.client_coordinates();
                let (delta_x, delta_y) = *delta.read();
                node.x = coords.x - delta_x;
                node.y = coords.y - delta_y;
            },
            onmouseout: move |_| {
                if *grabbed.read() {
                    *grabbed.write() = false;
                }
            },
            onmouseup: move |_| {
                *grabbed.write() = false;
            },
            "id: {id}, node_type: {node_type}"
        }

        // delete button
        div {
            onclick: move |_event| {
                // FIXME: popup
                graph.write().remove_node(id);
            },
            class: "absolute block bg-rose-500 text-center",
            style: "left: {x+w-10.0}px; top: {y}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
            "x"
        }
        // input node
        div {
            class: "absolute block rounded-full bg-sky-500",
            style: "left: {input_pos.0}px; top: {input_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
        },
        // output node
        div {
            class: "absolute block rounded-full bg-rose-500",
            style: "left: {output_pos.0}px; top: {output_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
        }
    }
}

// TODO: minimap
#[component]
fn ViewPort(
    view_port_state: Signal<ViewPortState>,
    dragged: Signal<Option<CurrentlyDragged>>,
    graph: Signal<Graph>,
) -> Element {
    let nodes = &graph.read().nodes;
    let mut grabbed = use_signal(|| false);
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


            // prevent_default + own ondragover enable drop area on current container
            prevent_default: "ondragover",
            ondragover: move |_event| {},

            ondrop: move |event| {
                if let Some(CurrentlyDragged{ node_type, delta_x, delta_y }) = *dragged.read() {
                    let graph = &mut*graph.write();
                    let id = graph.get_id();
                    let coords = event.client_coordinates();
                    let node_state = Signal::new(NodeState::new(
                        id, node_type, coords.x - delta_x, coords.y - delta_y
                    ));
                    graph.add_node(id, node_state);
                }
                *dragged.write() = None;
            },

            // panning funcs
          //onmousedown: move |_event| {
          //    *grabbed.write() = true;
          //},

          //onmousemove: move |_event| {
          //    if *grabbed.read() {

          //    }
          //},
          //onmouseup: move |_event| {
          //    *grabbed.write() = false;
          //},

            for (&id, &node) in nodes.iter() {
                Node{ id: id, graph: graph, node: node }
            }
        }
    }
}

#[component]
fn Edges(graph: Signal<Graph>) -> Element {
    let graph = &*graph.read();
    let iter = graph.nodes.iter().map(|(&_id, &node)| {
        let node = &*node.read();
        let (x, y) = node.input_pos();
        (x + node.port_diameter / 2.0, y + node.port_diameter / 2.0)
    });
    rsx! {
        svg {
            class: "absolute overflow-visible top-0 left-0 z-10 select-none",
            width: "1px",
            height: "1px",
            g{
                for (x, y) in iter {
                    path {
                        stroke_width: "1",
                        stroke: "red",
                        fill: "none",
                        //d: "M300,300 C300,300 400,400 500,500",
                        d: "M400,300 C{x},{y} {x},{y} {x},{y}"
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CurrentlyDragged {
    node_type: &'static str,
    delta_x: f64,
    delta_y: f64,
}

impl CurrentlyDragged {
    fn new(node_type: &'static str, delta_x: f64, delta_y: f64) -> Self {
        Self {
            node_type,
            delta_x,
            delta_y,
        }
    }
}

#[component]
pub fn Workspace(workspace: String) -> Element {
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
            style: "grid-template-columns: auto 1fr;", // exception to Tailwind only bc TW doesn't have classes to customize column widths
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
                    class: "justify-self-center mt-3",
                    "Pipeline Sections"
                }
                for node_type in menu_items.iter() {
                    MenuItem { currently_dragged: currently_dragged, node_type: node_type }
                }
            }
            // viewport
            div {
                class: "",
                ViewPort { view_port_state: view_port_state, dragged: currently_dragged, graph: graph }
            }
            // graph edges
            Edges{ graph: graph }
        }
    }
}
