use std::{
    cmp::Ordering,
    collections::{BTreeMap, VecDeque},
    pin::pin,
    time::Duration,
};

use graph::Graph as GenericGraph;
use section::{
    command_channel::ReplyTo as _,
    prelude::{RootChannel as _, SinkExt},
    section::Section as _,
    SectionError, SectionMessage,
};
use sha2::{Digest, Sha256};
use stub::Stub;
use tokio::{
    sync::mpsc::{channel, unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;
use uuid::Uuid;

use crate::{
    runtime::Graph as RawGraph,
    runtime_error::RuntimeError,
    section_channel::{RootChannel, SectionRequest},
    sqlite_storage::{SqliteState, SqliteStorageHandle},
    Config, Result,
};

use tokio::sync::oneshot::{channel as oneshot_channel, Sender as OneshotSender};

type Graph = GenericGraph<Uuid, Config>;

/// Wrappings around tokio channel which allow Sender to behave as Sink and Receiver as a Stream
///
/// Channels allow to glue pipe sections together (both static and dynamic)
fn streaming_channel(
    buf_size: usize,
) -> (PollSender<SectionMessage>, ReceiverStream<SectionMessage>) {
    let (tx, rx) = channel(buf_size);
    (PollSender::new(tx), ReceiverStream::new(rx))
}

#[derive(Debug)]
pub struct Task {
    // FIXME: task id is a hash of whole graph and created locally
    id: String,
    storage_handle: SqliteStorageHandle,
    graph: Graph,
    status: TaskStatus,
    root_channel: RootChannel<SqliteState>,
    section_handles: BTreeMap<Uuid, JoinHandle<Result<(), SectionError>>>,
}

impl Task {
    fn new(id: String, graph: Graph, storage_handle: SqliteStorageHandle) -> Self {
        Self {
            id,
            storage_handle,
            graph,
            status: TaskStatus::New,
            root_channel: RootChannel::new(),
            section_handles: BTreeMap::new(),
        }
    }

    fn spawn(mut self) -> TaskHandle {
        let (tx, rx) = unbounded_channel();
        tokio::spawn(async move {
            let rx = rx;
            match self.enter_loop(rx).await {
                Err(e) => tracing::error!("task with id {} shutdown with error: {e}", self.id),
                Ok(()) => tracing::info!("task with id {} exited normally", self.id),
            }
        });
        TaskHandle { tx }
    }

    async fn enter_loop(&mut self, mut rx: UnboundedReceiver<TaskMessage>) -> Result<()> {
        tracing::info!("running task {}", self.id);
        self.status = TaskStatus::Starting;

        loop {
            // task init loop
            while self.status != TaskStatus::Running {
                tokio::select! {
                    res = self.start_sections() => {
                        match res {
                            Ok(()) => {
                                self.status = TaskStatus::Running;
                            },
                            Err(e) => {
                                tracing::error!("task with id {} failed to start: {e}", self.id);
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    },
                    msg = rx.recv() => {
                        let msg = match msg {
                            Some(msg) => msg,
                            None => Err(RuntimeError::ChannelRecvError)?
                        };
                        match msg {
                            TaskMessage::Shutdown {reply_to} => {
                                self.shutdown().await.ok();
                                reply_to.send(()).ok();
                            },
                            TaskMessage::Status {reply_to }=> {
                                reply_to.send(self.status).ok();
                            }
                        }
                    }
                }
            }

            // task loop
            loop {
                tokio::select! {
                    msg = self.root_channel.recv() => {
                        let msg = match msg {
                            Ok(msg) => msg,
                            Err(_) => {
                                self.shutdown().await?;
                                tracing::error!("root channel doesn't have any senders");
                                return Ok(())
                            }
                        };
                        match msg {
                            SectionRequest::Stopped{ id } => {
                                let reason = match self.section_handles.remove(&id) {
                                    Some(handle) => {
                                        match handle.await {
                                            Ok(reason) => reason,
                                            Err(_) => Err("<unavailable>".into())
                                        }
                                    },
                                    None => Err("<unavailable>".into())
                                };
                                let section_name = self.graph.get_node(id).map(|node| node.name()).unwrap_or("");
                                tracing::error!(
                                    "section '{section_name}' with id '{id}' stopped, reason: {:?}",
                                     reason.err().unwrap_or("ok".into())
                                );
                                self.shutdown().await?;
                                self.status = TaskStatus::Starting;
                                break
                            },
                            SectionRequest::RetrieveState { id, reply_to } => {
                                // FIXME: proper errors
                                reply_to.reply(
                                    self.storage_handle.retrieve_state(id).await.map_err(RuntimeError::StorageError)?
                                ).await.ok();
                            },
                            SectionRequest::StoreState { id, state, reply_to } => {
                                // FIXME: proper errors
                                reply_to.reply(
                                    self.storage_handle.store_state(id, state).await.map_err(RuntimeError::StorageError)?
                                ).await.ok();
                            },
                            _ => {}
                        }
                    },
                    msg = rx.recv() => {
                        let msg = match msg {
                            None => {
                                self.shutdown().await.ok();
                                return Ok(())
                            },
                            Some(msg) => msg,
                        };
                        match msg {
                            TaskMessage::Shutdown { reply_to } => {
                                self.shutdown().await?;
                                reply_to.send(()).ok();
                            } ,
                            TaskMessage::Status { reply_to } =>{
                                reply_to.send(self.status).ok();
                            },
                        }
                    }
                }
            }

            // sleep between restarts
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    async fn start_sections(&mut self) -> Result<()> {
        let mut stub_counter = 0..;
        // build reverse index of edges
        let mut index = BTreeMap::<Uuid, Vec<Uuid>>::new();
        for (from, to) in self.graph.iter_edges() {
            index
                .entry(to)
                .and_modify(|vec| vec.push(from))
                .or_insert(vec![from]);
        }
        tracing::info!("graph: {:?}", self.graph);
        tracing::info!("index: {index:?}");

        // edge case, when graph consists out of single ndoe
        if index.is_empty() {
            let id = self
                .graph
                .iter_nodes()
                .map(|(id, _)| id)
                .next()
                .ok_or(RuntimeError::MalformedGraph)?;
            index.insert(id, vec![]);
        }

        // find root node, root node is the only node which doesn't have output in graph
        let mut root_node = None;
        for (id, _) in self.graph.iter_nodes() {
            if self.graph.get_child_node(id).is_none() {
                root_node = Some(id);
                break;
            }
        }

        let root_node = match root_node {
            None => Err(RuntimeError::MalformedGraph)?,
            Some(node) => node,
        };

        // root node output will be redirected to stub
        let (tx, rx) = streaming_channel(1);
        // FIXME: what is there is a node with this id?
        let stub_id = Uuid::from_u128(stub_counter.next().unwrap());
        let section_channel = self
            .root_channel
            .add_section(stub_id)
            .map_err(|_| RuntimeError::SectionChannelAllocationError)?;
        let stub_handle = tokio::spawn(async move {
            Stub::<SectionMessage, SectionError>::new()
                .start(
                    Box::pin(rx),
                    Box::pin(Stub::<SectionMessage, SectionError>::new()),
                    section_channel,
                )
                .await
        });
        self.section_handles.insert(stub_id, stub_handle);

        let mut queue = VecDeque::from([(root_node, tx)]);
        while let Some((node_id, prev_tx)) = queue.pop_front() {
            let node = match self.graph.get_node(node_id) {
                Some(node) => node.clone(),
                None => Err(RuntimeError::MalformedGraph)?,
            };
            let (tx, rx) = streaming_channel(1);
            let section_channel = self
                .root_channel
                .add_section(node_id)
                .map_err(|_| RuntimeError::SectionChannelAllocationError)?;
            let handle =
                tokio::spawn(async move {
                    node.as_dyn_section()
                        .dyn_start(
                            Box::pin(rx),
                            Box::pin(prev_tx.sink_map_err(|_| {
                                "failed to send message to downstream sink".into()
                            })),
                            section_channel,
                        )
                        .await
                });
            self.section_handles.insert(node_id, handle);
            match index.get(&node_id) {
                Some(parents) => {
                    for parent in parents.iter() {
                        queue.push_back((*parent, tx.clone()));
                    }
                }
                None => {
                    // if node doesn't have input - stub it
                    let stub_id = Uuid::from_u128(stub_counter.next().unwrap());
                    let section_channel = self
                        .root_channel
                        .add_section(stub_id)
                        .map_err(|_| RuntimeError::SectionChannelAllocationError)?;
                    let stub_handle = tokio::spawn(async move {
                        Stub::<SectionMessage, SectionError>::new()
                            .start(
                                Box::pin(Stub::<SectionMessage, SectionError>::new()),
                                Box::pin(tx),
                                section_channel,
                            )
                            .await
                    });
                    self.section_handles.insert(stub_id, stub_handle);
                }
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        // send shutdown signal to all sections and removes all section handles from root channel
        self.root_channel.shutdown();
        let mut shutdown_timeout = pin!(tokio::time::sleep(Duration::from_secs(5)));
        while !self.section_handles.is_empty() {
            tokio::select! {
                _ = &mut shutdown_timeout => {
                    tracing::error!("task with id {} reached shutdown timeout, sections will be terminated", self.id);
                    let mut section_handles = BTreeMap::new();
                    std::mem::swap(&mut section_handles, &mut self.section_handles);
                    for (id, handle) in section_handles {
                        tracing::info!("terminating section with id '{id}'");
                        handle.abort();
                    }
                },
                msg = self.root_channel.recv() => {
                    let msg = match msg {
                        Ok(msg) => msg,
                        Err(e) => {
                            tracing::error!("task with id {} received error on root channel: {e}", self.id);
                            Err(RuntimeError::ChannelRecvError)?
                        }
                    };
                    if let SectionRequest::Stopped { id } = msg {
                        self.section_handles.remove(&id);
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
// FIXME:
#[allow(unused)]
pub enum TaskStatus {
    New,
    Starting,
    Running,
    Down,
}

pub struct TaskHandle {
    tx: UnboundedSender<TaskMessage>,
}

// FIXME:
#[allow(unused)]
impl TaskHandle {
    async fn status(&self) -> TaskStatus {
        let (reply_to, rx) = oneshot_channel();
        if self.tx.send(TaskMessage::Status { reply_to }).is_err() {
            return TaskStatus::Down;
        }
        match rx.await {
            Ok(status) => status,
            Err(_) => TaskStatus::Down,
        }
    }

    async fn shutdown(&self) {
        let (reply_to, rx) = oneshot_channel();
        self.tx.send(TaskMessage::Shutdown { reply_to }).ok();
        rx.await.ok();
    }
}

enum TaskMessage {
    Status { reply_to: OneshotSender<TaskStatus> },
    Shutdown { reply_to: OneshotSender<()> },
}

struct Scheduler {
    tasks: BTreeMap<String, TaskHandle>,
    storage_handle: SqliteStorageHandle,
}

impl Scheduler {
    fn spawn(mut self) -> SchedulerHandle {
        let (tx, mut rx) = unbounded_channel();
        tokio::spawn(async move {
            if let Err(e) = self.enter_loop(&mut rx).await {
                tracing::error!("scheduler down with: {e}")
            }
        });
        SchedulerHandle { tx }
    }

    async fn enter_loop(&mut self, rx: &mut UnboundedReceiver<SchedulerMessage>) -> Result<()> {
        while let Some(message) = rx.recv().await {
            match message {
                SchedulerMessage::Schedule {
                    raw_graph,
                    reply_to,
                } => {
                    reply_to.send(self.schedule(raw_graph).await).ok();
                }
                SchedulerMessage::Shutdown { reply_to } => {
                    {
                        self.shutdown().await;
                        reply_to.send(())
                    }
                    .ok();
                }
            }
        }
        Ok(())
    }

    /// Schedule graph assigned to daemon
    ///
    /// 1. build graph from incoming 'raw' graph
    /// 2. since incoming graph is a forrest - split graph into groups of connected nodes
    /// 3. each group should have unique and idempotent id, which will be calculated as a hash of sorted node ids / configs
    /// 4. calculate diff between previously spawned groups and new groups
    /// 5. spawn tasks for new groups, shutdown outdated groups, groups with actual id should be ignored
    //
    // FIXME: got large graph building and hashing can time some time, it would be nice to have yielding to allow scheduler to run other tasks
    async fn schedule(&mut self, raw_graph: RawGraph) -> Result<()> {
        tracing::info!("raw graph: {:?}", raw_graph);
        let mut graph = Graph::new();
        for node in raw_graph.nodes.into_iter() {
            graph.add_node(node.id, node.config);
        }
        for edge in raw_graph.edges.into_iter() {
            graph.add_edge_partial(edge.from_id, edge.to_id);
        }
        let mut tasks = BTreeMap::new();

        for graph in graph.get_subgraphs() {
            // graph uses btree under the hood, node id's and edges are sorted.
            let mut hasher = Sha256::new();
            for (id, config) in graph.iter_nodes() {
                hasher.update(id.as_bytes());
                for field in config.fields() {
                    hasher.update(field.name.as_bytes());
                    hasher.update(field.value.to_string().as_bytes());
                }
            }
            for (from, to) in graph.iter_edges() {
                hasher.update(from.as_bytes());
                hasher.update(to.as_bytes());
            }
            tasks.insert(format!("{:x}", hasher.finalize()), graph);
        }
        tracing::info!("tasks: {:?}", tasks);

        let mut to_delete = Vec::<String>::new();
        let mut to_add = Vec::<(String, Graph)>::new();

        let mut new_tasks = tasks.into_iter().peekable();
        let mut current_keys = self.tasks.keys().peekable();
        loop {
            match (new_tasks.peek(), current_keys.peek()) {
                (None, None) => break,
                (Some((new_key, _)), Some(old_key)) => {
                    match new_key.cmp(old_key) {
                        Ordering::Equal => {
                            // key is present both in old and new datasets
                            new_tasks.next();
                            current_keys.next();
                        }
                        Ordering::Greater => {
                            // new key is greater than old key, means old key is not present in task set
                            // and this task needs to be shutdown
                            to_delete.push(current_keys.next().unwrap().to_string());
                        }
                        Ordering::Less => {
                            // new key is less than old key, means that new key is not present in task set
                            // and needs to be added
                            to_add.push(new_tasks.next().unwrap());
                        }
                    }
                }
                (None, Some(_)) => to_delete.push(current_keys.next().unwrap().to_string()),
                (Some(_), None) => to_add.push(new_tasks.next().unwrap()),
            }
        }
        for id in to_delete {
            if let Some(task) = self.tasks.remove(&id) {
                task.shutdown().await;
            }
        }
        for (id, graph) in to_add {
            self.tasks.insert(
                id.clone(),
                Task::new(id, graph, self.storage_handle.clone()).spawn(),
            );
        }
        Ok(())
    }

    async fn shutdown(&mut self) {
        let mut tasks = BTreeMap::new();
        std::mem::swap(&mut tasks, &mut self.tasks);
        for (_, task_handle) in tasks.into_iter() {
            task_handle.shutdown().await;
        }
    }
}

// FIXME:
#[allow(unused)]
enum SchedulerMessage {
    Schedule {
        raw_graph: RawGraph,
        reply_to: OneshotSender<Result<()>>,
    },
    Shutdown {
        reply_to: OneshotSender<()>,
    },
}

#[derive(Debug)]
pub struct SchedulerHandle {
    tx: UnboundedSender<SchedulerMessage>,
}

impl SchedulerHandle {
    pub async fn schedule(&self, raw_graph: RawGraph) -> Result<()> {
        let (reply_to, rx) = oneshot_channel();
        let message = SchedulerMessage::Schedule {
            raw_graph,
            reply_to,
        };
        self.tx.send(message)?;
        rx.await?
    }

    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub fn new(storage_handle: SqliteStorageHandle) -> SchedulerHandle {
    Scheduler {
        tasks: BTreeMap::new(),
        storage_handle,
    }
    .spawn()
}
