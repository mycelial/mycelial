use std::rc::Rc;

use crate::components::app::AppState;
use crate::components::graph::Graph as GenericGraph;
use crate::components::icons::{Delete, Edit, Pause, Play, Restart};
use crate::components::node_state_form::NodeStateForm;
use crate::config_registry::ConfigMetaData;
use config::SectionIO;
use dioxus::prelude::*;
use serde::ser::SerializeMap;
use serde::Serialize;
use uuid::Uuid;

use super::graph::GraphOperation;
use super::node_state_form::NodeState;

pub type Graph = GenericGraph<Uuid, Signal<NodeState>>;

#[derive(Debug, Clone, Serialize)]
pub enum WorkspaceOperation {
    AddNode(NodeState),
    RemoveNode(Uuid),
    UpdateNode(NodeState),
    AddEdge{
        from: Uuid,
        to: Uuid
    },
    RemoveEdge{
        from: Uuid,
        to: Uuid
    },
    UpdatePan { x: f64, y: f64 },
}

impl From<GraphOperation<Uuid, Signal<NodeState>>> for WorkspaceOperation {
    fn from(value: GraphOperation<Uuid, Signal<NodeState>>) -> Self {
        match value {
            GraphOperation::AddNode(node) => Self::AddNode(node.read().clone()),
            GraphOperation::RemoveNode(node) => Self::RemoveNode(node.read().id.clone()),
            GraphOperation::AddEdge(from, to) => Self::AddEdge{from, to},
            GraphOperation::RemoveEdge(from, to) => Self::RemoveEdge{from, to},
        }
    }
}

impl From<GraphOperation<Uuid, Signal<NodeState>>> for Vec<WorkspaceOperation> {
    fn from(val: GraphOperation<Uuid, Signal<NodeState>>) -> Self {
        let workspace_op: WorkspaceOperation = val.into();
        vec![workspace_op]
    }
}

//
#[derive(Debug)]
pub struct WorkspaceUpdate {
    name: Rc<str>,
    operations: Vec<WorkspaceOperation>,
}

// Rc<str> doesn't implement Serialize
impl Serialize for WorkspaceUpdate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("name", &*self.name)?;
        map.serialize_entry("operations", &*self.operations)?;
        map.end()
    }
}

impl WorkspaceUpdate {
    fn new(name: &Rc<str>, operations: impl Into<Vec<WorkspaceOperation>>) -> Self {
        Self {
            name: Rc::clone(name),
            operations: operations.into(),
        }
    }
}

// representation of section in sections menu, which can be dragged into viewport container
#[component]
fn MenuItem(
    metadata: ConfigMetaData,
    mut dragged_menu_item: Signal<Option<DraggedMenuItem>>,
) -> Element {
    let mut rect_data = use_signal(|| (0.0, 0.0, 0.0, 0.0));
    let node_type: Rc<str> = Rc::clone(&metadata.ty);
    let source = match metadata.input {
        true => rsx! {
            span {
                class: "bg-moss-1 text-white rounded-full p-1 ml-1",
                "Source"
            }
        },
        false => None,
    };
    let destination = match metadata.output {
        true => rsx! {
            span {
                class: "bg-forest-2 text-white rounded-full p-1 ml-1",
                "Dest"
            }
        },
        false => None,
    };
    rsx! {
        div {
            class: "min-w-32 min-h-24 border border-solid rounded grid grid-flow-rows p-2 shadow",
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
                *dragged_menu_item.write() = Some(DraggedMenuItem::new(metadata.clone(), delta_x, delta_y));
            },
            ondragend: move |_event| {
                *dragged_menu_item.write() = None;
            },
            div {
                class: "grid grid-flow-col",
                p {
                    class: "uppercase inline",
                    "{node_type}"
                }
                span {
                    class: "justify-self-end",
                    {source}
                    {destination}
                }
            }
            div {
                "Type: {node_type}"
            }
        }
    }
}

// section in viewport
#[component]
fn Node(
    workspace: Rc<str>,
    mut app_state: Signal<AppState>,
    mut graph: Signal<Graph>,
    mut dragged_node: Signal<Option<DraggedNode>>,
    mut dragged_edge: Signal<Option<DraggedEdge>>,
    mut node: Signal<NodeState>,
    mut selected_node: Signal<Option<Signal<NodeState>>>,
) -> Element {
    let node_ref = &*node.read();
    let (id, x, y, _w, _h, port_diameter, config, input_pos, output_pos) = (
        node_ref.id,
        node_ref.x,
        node_ref.y,
        node_ref.w,
        node_ref.h,
        node_ref.port_diameter,
        &*(node_ref.config),
        node_ref.input_pos(),
        node_ref.output_pos(),
    );
    let node_type = config.name();

    let mut is_playing = use_signal(|| false);

    let input = match config.input() {
        SectionIO::None => None,
        _ => rsx! {
            div {
                class: "absolute block rounded-full bg-moss-1 z-10",
                style: "left: {input_pos.0}px; top: {input_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
                onmouseup: move |_event|  {
                    let dragged = &mut *dragged_edge.write();
                    if let Some(DraggedEdge{from_node, ..}) = dragged {
                        let op = graph.write().add_edge(*from_node, id);
                        *dragged = None;
                    }
                }
            }
        },
    };
    let output = match config.output() {
        SectionIO::None => None,
        _ => rsx! {
            div {
                class: "absolute block rounded-full bg-forest-2 z-10",
                style: "left: {output_pos.0}px; top: {output_pos.1}px; min-width: {port_diameter}px; min-height: {port_diameter}px;",
                onmousedown: move |event| {
                    if dragged_edge.read().is_none() && dragged_node.read().is_none() {
                        let coords = event.client_coordinates();
                        *dragged_edge.write() = Some(DraggedEdge::new(id, coords.x, coords.y));
                    }
                },
            }
        },
    };

    rsx! {
        div {
            class: "shadow min-w-60 max-w-60 grid grid-flow-rows gap-2 absolute min-h-24 border border-solid bg-white rounded-sm px-2 z-[5] select-none overflow-visible",
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
            onmouseover: move |_event| {
                // if node doesn't have input - do nothing
                if node.read().config.input().is_none() {
                    return
                }
                let dragged = &mut *dragged_edge.write();
                if let Some(dragged) = dragged {
                    dragged.to_node = Some(id);
                }
            },
            onmouseout: move |_event| {
                if let Some(dragged) = &mut *dragged_edge.write() {
                    dragged.to_node.take();
                }
            },
            onmousedown: move |event| {
                if dragged_node.read().is_none() {
                    let coords = event.client_coordinates();
                    let (delta_x, delta_y) = {
                        let node = &*node.read();
                        (coords.x - node.x, coords.y - node.y)
                    };
                    *dragged_node.write() = Some(DraggedNode::new(node, delta_x, delta_y));
                }
            },
            onmouseup: move |_event|  {
                // if node doesn't have input - do nothing
                if node.read().config.input().is_none() {
                    return
                }
                let dragged = &mut *dragged_edge.write();
                if let Some(DraggedEdge{from_node, ..}) = dragged.take() {
                    let op = graph.write().add_edge(from_node, id);
                    tracing::info!("add edge: {op:?}");
                }
            },
            div {
                class: "pt-5 uppercase",
                style: "font-size: 0.65rem",
                "{id}"
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
                if *is_playing.read() {
                    span {
                        onclick: move |_event| {
                            let current_value = *is_playing.read();
                            *is_playing.write() = !current_value;
                        },
                        class: "cursor-pointer hover:bg-stem-1",
                        Pause {}
                    }
                } else {
                    span {
                        onclick: move |_event| {
                            let current_value = *is_playing.read();
                            *is_playing.write() = !current_value;
                        },
                        class: "cursor-pointer hover:bg-stem-1",
                        Play {}
                    }
                }
                span {
                    class: "cursor-pointer hover:bg-stem-1",
                    Restart {}
                }
                span {
                    onclick: move |_event| {
                        selected_node.set(Some(node));
                    },
                    class: "cursor-pointer hover:bg-stem-1",
                    Edit {}
                }
                span {
                    onclick: move |_event| {
                        // FIXME: action warning
                        let ops: Vec<_> = graph.write().remove_node(id).into_iter().map(Into::<WorkspaceOperation>::into).collect();
                        app_state.write().update_workspace(WorkspaceUpdate::new(&workspace, ops));
                    },
                    class: "cursor-pointer hover:bg-stem-1",
                    Delete {}
                }
            }
        }
        {input}
        {output}
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
    workspace: Rc<str>,
    mut app_state: Signal<AppState>,
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
                        },
                        Err(e) => tracing::error!("failed to read rect data: {e}"),
                    }
                });
            },


            ondrop: move |event| {
                if let Some(DraggedMenuItem{ metadata, delta_x, delta_y }) = &*dragged_menu_item.read() {
                    let graph = &mut*graph.write();
                    let id = uuid::Uuid::now_v7();
                    let coords = event.client_coordinates();
                    let vps_ref = &*view_port_state.read();
                    let config = app_state.read().build_config(&metadata.ty);
                    let node_state = Signal::new(NodeState::new(
                        id,
                        coords.x - *delta_x - vps_ref.delta_x(),
                        coords.y - *delta_y - vps_ref.delta_y(),
                        config,
                    ));
                    let op = graph.add_node(id, node_state);
                    app_state.write().update_workspace(WorkspaceUpdate::new(&workspace, op));
                }
                *dragged_menu_item.write() = None;
            },

            onmousedown: move |event| {
                // if node or edge is currently dragged or node selected - do nothing
                if dragged_edge.read().is_some() || dragged_node.read().is_some() || selected_node.read().is_some() {
                    return
                }

                let coords = event.client_coordinates();
                let state = &mut* view_port_state.write();
                grab_point.set((coords.x - state.x, coords.y - state.y));
                state.grabbed = true;
            },

            onmousemove: move |event| {
                // if node or edge is currently dragged or node selected - do nothing
                if dragged_edge.read().is_some() || dragged_node.read().is_some() || selected_node.read().is_some() {
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

            onmouseup: move |_event| {
                view_port_state.write().grabbed = false;
            },

            if selected_node.read().is_none() {
                div {
                    class: "overflow-visible",
                    style: "transform: translate({state_ref.x}px, {state_ref.y}px)",
                    for (_, node) in (&*graph.read()).iter_nodes() {
                        Node{
                            workspace: Rc::clone(&workspace),
                            app_state,
                            graph,
                            dragged_node,
                            dragged_edge,
                            node: *node,
                            selected_node,
                        }
                    }
                    // graph edges
                    Edges { graph, view_port_state, dragged_edge }
                }
            }
            if selected_node.read().is_some() {
                NodeStateForm {
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
        // offset window scroll
        let (scroll_x_offset, scroll_y_offset) = DraggedEdge::get_scroll_xy();
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
                    *x - view_port_state_ref.delta_x() + scroll_x_offset,
                    *y - view_port_state_ref.delta_y() + scroll_y_offset,
                ),
            };
            let output_pos = node.read().output_pos();
            let (out_x, out_y) = (output_pos.0 + offset, output_pos.1 + offset);
            dragged_edge_element = rsx! {
                path {
                    stroke_width: "1",
                    stroke: "red",
                    fill: "none",
                    d: if out_x < x {
                        format!(
                            "M{},{} C{},{} {},{} {},{}",
                            out_x, out_y, (out_x + x) / 2.0, out_y, (out_x + x) / 2.0, y, x, y
                        )
                    } else {
                        format!(
                            "M{},{} C{},{} {},{} {},{}",
                            out_x, out_y, (out_x * 2.0 - x), out_y, (x * 2.0 - out_x), y, x, y
                        )
                    }
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

#[derive(Debug, Clone)]
struct DraggedMenuItem {
    metadata: ConfigMetaData,
    delta_x: f64,
    delta_y: f64,
}

impl DraggedMenuItem {
    fn new(metadata: ConfigMetaData, delta_x: f64, delta_y: f64) -> Self {
        Self {
            metadata,
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
    from_node: Uuid,
    to_node: Option<Uuid>,
    x: f64,
    y: f64,
}

impl DraggedEdge {
    fn new(from_node: Uuid, x: f64, y: f64) -> Self {
        Self {
            from_node,
            to_node: None,
            x,
            y,
        }
    }

    fn get_scroll_xy() -> (f64, f64) {
        let window = match web_sys::window() {
            Some(window) => window,
            None => return (0.0, 0.0),
        };
        (
            window.scroll_x().unwrap_or(0.0),
            window.scroll_y().unwrap_or(0.0),
        )
    }
}

#[component]
pub fn Workspace(workspace: String) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let workspace = Rc::from(workspace);
    let state_fetcher = use_resource({
        let workspace = Rc::clone(&workspace);
        move || {
            let workspace = Rc::clone(&workspace);
            async move {
                let _workspace_state = match app_state.read().fetch_workspace(&*workspace).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("failed to fetch state: {e}");
                        return
                    }
                };
            }
        }
    });
    if state_fetcher.read().is_none() {
        return None;
    }

    let graph: Signal<Graph> = use_signal(Graph::new);
    let dragged_menu_item: Signal<Option<DraggedMenuItem>> = use_signal(|| None);
    let mut dragged_node: Signal<Option<DraggedNode>> = use_signal(|| None);
    let mut dragged_edge: Signal<Option<DraggedEdge>> = use_signal(|| None);
    let mut selected_node: Signal<Option<Signal<NodeState>>> = use_signal(|| None);
    let view_port_state = use_signal(ViewPortState::new);
    let menu_items = app_state.read().menu_items().collect::<Vec<ConfigMetaData>>();

    rsx! {
        div {
            onmouseup: move |_event|  {
                if dragged_node.read().is_some() {
                    *dragged_node.write() = None;
                    // FIXME: add update operation
                }
                if dragged_edge.read().is_some() {
                    *dragged_edge.write() = None;
                }
            },

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

            class: "grid",
            style: "grid-template-columns: auto 1fr;", // exception to Tailwind only bc TW doesn't have classes to customize column widths
            div {
                onclick: move |_| {
                    selected_node.set(None);
                },
                class: "col-span-2 pl-2 py-4 text-stem-1 bg-night-2 grid grid-cols-2",
                h1 {
                    class: "text-lg justify-self-start",
                    "Workspace: {workspace}",
                }
                button {
                    class: "text-stem-1 font-bold px-4 py-2 rounded bg-forest-1 border border-forest-2 justify-self-end mr-5 uppercase hover:bg-forest-2 hover:text-white",
                    onclick: move |_event| async move {
                        if let Err(e) = app_state.write().publish_updates().await {
                            tracing::error!("failed to publish: {e}");
                        }
                    },
                    "Publish"
                }
            }
            // section menu
            div {
                div {
                    class: "h-[calc(100vh-200px)] overflow-none select-none z-[100] min-w-96 max-w-96",
                    div {
                        class: "border border-solid overflow-y-scroll bg-white grid grid-flow-rows gap-y-3 md:px-2 h-max-screen",
                        h2 {
                            class: "justify-self-center mt-3",
                            "Pipeline Sections"
                        }
                        for metadata in menu_items {
                            MenuItem { metadata, dragged_menu_item }
                        }
                    }
                }
            }
            // viewport
            div {
                id: "mycelial-viewport",
                class: "h-[calc(100vh-200px) w-full scroll-none overflow-hidden",
                ViewPort {
                    workspace,
                    app_state,
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
