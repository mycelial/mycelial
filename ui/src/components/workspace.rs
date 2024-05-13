use crate::components::graph::Graph as GenericGraph;
use crate::components::node_state::NodeState;
use crate::components::node_config::NodeConfig;
use crate::components::icons::{Play, Pause, Restart, Edit, Delete};
use dioxus::prelude::*;

pub type Graph = GenericGraph<Signal<NodeState>>;

// representation of section in sections menu, which can be dragged into viewport container
#[component]
fn MenuItem(
    node_type: &'static str,
    mut dragged_menu_item: Signal<Option<DraggedMenuItem>>,
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
                *dragged_menu_item.write() = Some(DraggedMenuItem::new(node_type, delta_x, delta_y));
            },
            ondragend: move |_event| {
                *dragged_menu_item.write() = None;
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
                        class: "bg-moss-1 text-white rounded-full p-1 ml-1",
                        "Source"
                    }
                    // TODO: change background to grey if not dest
                    span {
                        class: "bg-forest-2 text-white rounded-full p-1 ml-1",
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
fn Node(
    mut graph: Signal<Graph>,
    mut dragged_node: Signal<Option<DraggedNode>>,
    mut dragged_edge: Signal<Option<DraggedEdge>>,
    mut node: Signal<NodeState>,
    mut selected_node: Signal<Option<Signal<NodeState>>>, 
) -> Element {
    // current node coordinates, coordinates on input and output ports
    let (id, node_type, x, y, w, _h, port_diameter, input_pos, output_pos) = {
        let node = &*node.read();
        (
            node.id,
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

    let mut is_playing = false;
    let mut is_being_edited = false;

    rsx! {
        div {
            class: "grid grid-flow-rows gap-2 absolute min-w-31 min-h-24 border border-solid bg-white rounded-sm px-2 z-[5] select-none",
            style: "left: {x}px; top: {y}px;",
            // recalculate positions on input/output nodes
            onmounted: move |event| {
                spawn(async move {
                    match event.get_client_rect().await {
                        Ok(rect) => {
                            let (w, h) = (rect.size.width, rect.size.height);
                            let node = &mut* node.write();
                            node.w = w;
                            node.h = h;
                        },
                        Err(e) => tracing::error!("failed to read rect data: {e}"),
                    }
                });
            },
            prevent_default: "onmouseover",
            onmouseover: move |_event| {
                let dragged = &mut *dragged_edge.write();
                if let Some(dragged) = dragged {
                    dragged.to_node = Some(id);
                }
            },
            prevent_default: "onmouseout",
            onmouseout: move |_event| {
                let dragged = &mut *dragged_edge.write();
                if let Some(dragged) = dragged {
                    dragged.to_node = None
                }
            },
            prevent_default: "onmousedown",
            onmousedown: move |event| {
                selected_node.set(Some(node));
                tracing::info!("selected node is {:?}", selected_node.read());

                if dragged_node.read().is_none() {
                    let coords = event.client_coordinates();
                    let (delta_x, delta_y) = {
                        let node = &*node.read();
                        (coords.x - node.x, coords.y - node.y)
                    };
                    *dragged_node.write() = Some(DraggedNode::new(node, delta_x, delta_y));
                }
            },
            prevent_default: "onmouseup",
            onmouseup: move |_event|  {
                let dragged = &mut *dragged_edge.write();
                if let Some(DraggedEdge{from_node, ..}) = dragged {
                    graph.write().add_edge(*from_node, id);
                    *dragged = None;
                }
            },
            div {
                class: "pt-5 uppercase",
                "Name of Section with id {id}"
            }
            div {
                class: "text-night-2",
                "Daemon: Name of Daemon"
            }
            div {
                class: "pb-3",
                "Section Type: {node_type}"
            }
            div {
                class: "grid grid-flow-col justify-items-center mb-2 border p-2 rounded bg-grey-bright",
                if is_playing {
                    span {
                        onclick: move |_event| {
                            is_playing = !is_playing
                        },
                        class: "",
                        Pause {} 
                    }
                } else {
                    span {
                        onclick: move |_event| {
                            is_playing = !is_playing
                        },
                        class: "",
                        Play {} 
                    }
                }
                Restart {} 
                span {
                    onclick: move |_event| {
                        is_being_edited = true;
                        tracing::info!{ "is_being_edited: {is_being_edited}"};
                    },
                    Edit {} 
                }
                span {
                    onclick: move |_event| {
                        // FIXME: popup
                        graph.write().remove_node(id);
                    },
                    Delete {}
                }
            }
        }

        // input node
        div {
            class: "absolute block rounded-full bg-moss-1 z-10",
            style: "left: {input_pos.0}px; top: {input_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
            prevent_default: "onmouseup",
            onmouseup: move |_event|  {
                let dragged = &mut *dragged_edge.write();
                if let Some(DraggedEdge{from_node, ..}) = dragged {
                    graph.write().add_edge(*from_node, id);
                    *dragged = None;
                }
            }
        },
        // output node
        div {
            class: "absolute block rounded-full bg-forest-2 z-10",
            style: "left: {output_pos.0}px; top: {output_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
            prevent_default: "onmousedown",
            onmousedown: move |event| {
                if dragged_edge.read().is_none() && dragged_node.read().is_none() {
                    let coords = event.client_coordinates();
                    *dragged_edge.write() = Some(DraggedEdge::new(id, coords.x, coords.y));
                }
            },
        }
    }
}

#[derive(Debug)]
struct ViewPortState {
    x: f64,
    y: f64,
    // position of the viewport div
    // required to compensate node placement
    delta_x: f64,
    delta_y: f64,
    grabbed: bool,
}

impl ViewPortState {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
            grabbed: false,
        }
    }

    fn delta_x(&self) -> f64 {
        self.delta_x + self.x
    }

    fn delta_y(&self) -> f64 {
        self.delta_y + self.y
    }
}

#[component]
fn ViewPort(
    mut graph: Signal<Graph>,
    mut view_port_state: Signal<ViewPortState>,
    mut dragged_menu_item: Signal<Option<DraggedMenuItem>>,
    dragged_node: Signal<Option<DraggedNode>>,
    dragged_edge: Signal<Option<DraggedEdge>>,
    selected_node: Signal<Option<Signal<NodeState>>>,
) -> Element {
    let mut grab_point = use_signal(|| (0.0, 0.0));
    let state_ref = &*view_port_state.read();
    rsx! {
        div {
            class: "h-full w-full bg-grey-bright overflow-hidden select-none scroll-none",

            // prevent_default + own ondragover enable drop area on current container
            prevent_default: "ondragover",
            ondragover: move |_event| {},

            // this callback required to store coordinates of div with 'content'
            // which will be used to properly drop nodes
            onmounted: move |event| {
                spawn(async move {
                    match event.get_client_rect().await {
                        Ok(rect) => {
                            let (x, y) = (rect.origin.x, rect.origin.y);
                            let view_port_state = &mut *view_port_state.write();
                            view_port_state.delta_x = x;
                            view_port_state.delta_y = y;
                            tracing::info!("delta_x: {x}, delta_y: {y}");
                        },
                        Err(e) => tracing::error!("failed to read rect data: {e}"),
                    }
                });
            },

            prevent_default: "ondrop",
            ondrop: move |event| {
                let dragged = *dragged_menu_item.read();
                if let Some(DraggedMenuItem{ node_type, delta_x, delta_y }) = dragged {
                    let graph = &mut*graph.write();
                    let id = graph.get_id();
                    let coords = event.client_coordinates();
                    let vps_ref = &*view_port_state.read();
                    let node_state = Signal::new(NodeState::new(
                        id,
                        node_type,
                        coords.x - delta_x - vps_ref.delta_x(),
                        coords.y - delta_y - vps_ref.delta_y(),
                    ));
                    graph.add_node(id, node_state);
                }
                *dragged_menu_item.write() = None;
            },

            prevent_default: "onmousedown",
            onmousedown: move |event| {
                // if node or edge is currently dragged - do nothing
                if dragged_edge.read().is_some() || dragged_node.read().is_some() {
                    return
                }
                let coords = event.client_coordinates();
                let state = &mut* view_port_state.write();
                grab_point.set((coords.x - state.x, coords.y - state.y));
                state.grabbed = true;
            },

            prevent_default: "onmousemove",
            onmousemove: move |event| {
                // if node or edge is currently dragged - do nothing
                if dragged_edge.read().is_some() || dragged_node.read().is_some() {
                    return
                }
                let state = &mut *view_port_state.write();
                if state.grabbed {
                    let grab_point = *grab_point.read();
                    let coords = event.client_coordinates();
                    state.x = coords.x - grab_point.0;
                    state.y = coords.y - grab_point.1;
                }
            },

            prevent_default: "onmouseup",
            onmouseup: move |_event| {
                view_port_state.write().grabbed = false;
            },

            div {
                style: "transform: translate({state_ref.x}px, {state_ref.y}px)",
                for (_, node) in (&*graph.read()).iter_nodes() {
                    Node{
                        graph: graph,
                        dragged_node: dragged_node,
                        dragged_edge: dragged_edge,
                        node: *node,
                        selected_node: selected_node,
                    }
                }
                // graph edges
                Edges { graph, view_port_state, dragged_edge }
            }
            if selected_node.read().is_some() {
                NodeConfig {
                    selected_node: selected_node,
                }
            }
        }
    }
}

#[component]
fn Edges(
    graph: Signal<Graph>,
    view_port_state: Signal<ViewPortState>,
    dragged_edge: Signal<Option<DraggedEdge>>,
) -> Element {
    let graph_ref = &*graph.read();
    let view_port_state_ref = view_port_state.read();
    let edges_iter = graph_ref.iter_edges().filter_map(|(from_node, to_node)| {
        let from_node = graph_ref.get_node(from_node);
        let to_node = graph_ref.get_node(to_node);
        match (from_node, to_node) {
            (Some(from_node), Some(to_node)) => {
                let from_node = from_node.read();
                let offset = from_node.port_diameter / 2.0;
                let output_pos = from_node.output_pos();
                let input_pos = to_node.read().input_pos();
                Some((
                    from_node.id,
                    output_pos.0 + offset,
                    output_pos.1 + offset,
                    input_pos.0 + offset,
                    input_pos.1 + offset,
                ))
            }
            _ => None,
        }
    });
    let mut dragged_edge_element = None;
    if let Some(DraggedEdge {
        from_node,
        to_node,
        x,
        y,
    }) = &*dragged_edge.read()
    {
        if let Some(node) = graph_ref.get_node(*from_node) {
            let offset = node.read().port_diameter / 2.0;
            let (x, y) = match to_node
                .map(|to_node| graph_ref.get_node(to_node))
                .unwrap_or(None)
            {
                Some(to_node) => {
                    let input_pos = to_node.read().input_pos();
                    (input_pos.0 + offset, input_pos.1 + offset)
                }
                None => (
                    *x - view_port_state_ref.delta_x(),
                    *y - view_port_state_ref.delta_y(),
                ),
            };
            let output_pos = node.read().output_pos();
            let (out_x, out_y) = (output_pos.0 + offset, output_pos.1 + offset);
            dragged_edge_element = rsx! {
                path {
                    stroke_width: "1",
                    stroke: "red",
                    fill: "none",
                    d: "M{out_x},{out_y} C{(out_x+x)/2.0},{out_y} {(out_x+x)/2.0},{y} {x},{y}",
                }
            };
        }
    };
    rsx! {
        svg {
            class: "absolute overflow-visible top-0 left-0 z-[0]",
            width: "1px",
            height: "1px",
            defs {
                marker{
                    id: "arrow",
                    view_box: "0 0 10 10",
                    ref_x: "11",
                    ref_y: "5",
                    marker_units: "strokeWidth",
                    marker_width: "10",
                    marker_height: "10",
                    orient: "auto",
                    path {
                        d: "M 0 0 L 5 5 L 0 10 z",
                        fill: "#f00"
                    }
                }
            },
            g{
                for (_, x0, y0, x1, y1) in edges_iter.clone() {
                    path {
                        stroke_width: "1",
                        stroke: "red",
                        fill: "none",
                        marker_end: "url(#arrow)",
                        d: if x0 < x1 {
                            format!(
                                "M{},{} C{},{} {},{} {},{}",
                                x0, y0, (x0 + x1) / 2.0, y0, (x0 + x1) / 2.0, y1, x1, y1
                            )
                        } else {
                            format!(
                                "M{},{} C{},{} {},{} {},{}",
                                x0, y0, (x0 * 2.0 - x1), y0, (x1 * 2.0 - x0), y1, x1, y1
                            )
                        }
                    }
                }
                { dragged_edge_element }
            }
        }
        for (from, x0, y0, x1, y1) in edges_iter {
            div {
                onclick: move |_event| {
                    graph.write().remove_edge(from);
                },
                class: "absolute select-none min-w-5 min-h-5 bg-grey-bright z-[1] text-center text-red-500",
                style: "left: {(x0+x1)/2.0}px; top: {(y0+y1)/2.0}px; transform: translate(-50%,-50%)",
                "x"
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DraggedMenuItem {
    node_type: &'static str,
    delta_x: f64,
    delta_y: f64,
}

impl DraggedMenuItem {
    fn new(node_type: &'static str, delta_x: f64, delta_y: f64) -> Self {
        Self {
            node_type,
            delta_x,
            delta_y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DraggedNode {
    node: Signal<NodeState>,
    delta_x: f64,
    delta_y: f64,
}

impl DraggedNode {
    fn new(node: Signal<NodeState>, delta_x: f64, delta_y: f64) -> Self {
        Self {
            node,
            delta_x,
            delta_y,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DraggedEdge {
    from_node: u64,
    to_node: Option<u64>,
    x: f64,
    y: f64,
}

impl DraggedEdge {
    fn new(from_node: u64, x: f64, y: f64) -> Self {
        Self {
            from_node,
            to_node: None,
            x,
            y,
        }
    }
}

#[component]
pub fn Workspace(workspace: String) -> Element {
    let graph: Signal<Graph> = use_signal(|| {
        let mut g = Graph::new();
      //g.add_node_type("one", rsx!{ "form for one" }),
      //g.add_node_type("two", rsx!{ "form for two" });
        g
    });
    let dragged_menu_item: Signal<Option<DraggedMenuItem>> = use_signal(|| None);
    let mut dragged_node: Signal<Option<DraggedNode>> = use_signal(|| None);
    let mut dragged_edge: Signal<Option<DraggedEdge>> = use_signal(|| None);
    let mut selected_node: Signal<Option<Signal<NodeState>>> = use_signal(|| None);
    let view_port_state = use_signal(ViewPortState::new);
    let menu_items = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];

    let _view_port_state_ref = &*view_port_state.read();
    rsx! {
        div {
            prevent_default: "onmouseup",
            onmouseup: move |_event|  {
                if dragged_node.read().is_some() {
                    *dragged_node.write() = None;
                }
                if dragged_edge.read().is_some() {
                    *dragged_edge.write() = None;
                }
            },

            prevent_default: "onmousemove",
            onmousemove: move |event| {
                let coords = event.client_coordinates();
                if let Some(mut dragged_node) = *dragged_node.write() {
                    let node = &mut* dragged_node.node.write();
                    node.x = coords.x - dragged_node.delta_x;
                    node.y = coords.y - dragged_node.delta_y;
                }
                if let Some(dragged_edge) = &mut *dragged_edge.write() {
                    dragged_edge.x = coords.x;
                    dragged_edge.y = coords.y;
                }
            },

            // TODO: implement if/then logic such that 3rd column appears (with node details) when node selected
            class: "grid",
            style: "grid-template-columns: auto 1fr;", // exception to Tailwind only bc TW doesn't have classes to customize column widths
            div {
                prevent_default: "onclick",
                onclick: move |_| {
                    selected_node.set(None);
                },
                class: "col-span-2 pl-2 py-4 text-stem-1 bg-night-2 grid grid-cols-2",
                h1 {
                    class: "text-lg justify-self-start",
                    "Workspace: {workspace}",
                }
                button {
                    class: "text-stem-1 px-4 py-2 rounded bg-forest-1 border border-forest-2 justify-self-end mr-5 uppercase hover:bg-forest-2 hover:text-white",
                    onclick: move |_event| {
                        // TODO: implement publish logic here
                        tracing::info!("Publish button clicked");
                    },
                    "Publish"
                }
            }
            // section menu
            div {
                class: "border border-solid overflow-y-scroll bg-white grid grid-flow-rows gap-y-3 md:px-2 z-[100] select-none",
                h2 {
                    class: "justify-self-center mt-3",
                    "Pipeline Sections"
                }
                for node_type in menu_items.iter() {
                    MenuItem { node_type, dragged_menu_item }
                }
            }
            // viewport
            div {
                class: "h-full w-full scroll-none overflow-hidden",
                ViewPort {
                    graph,
                    view_port_state,
                    dragged_menu_item,
                    dragged_node,
                    dragged_edge,
                    selected_node,
                }
            }
        }
    }
}
