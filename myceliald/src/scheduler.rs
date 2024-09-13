use std::sync::Arc;

use graph::Graph as GenericGraph;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::{runtime::Graph as RawGraph, Config, Result};
use tokio::sync::oneshot::{
    channel as oneshot_channel, Sender as OneshotSender,
};

type Graph = GenericGraph<Uuid, Config>;

struct Scheduler {}

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
                    }.ok();
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
    async fn schedule(&mut self, raw_graph: RawGraph) -> Result<()> {
        let mut graph = Graph::new();
        for node in raw_graph.nodes.into_iter() {
            graph.add_node(node.id, Arc::from(node.config));
        }
        for edge in raw_graph.edges.into_iter() {
            graph.add_edge(edge.from_id, edge.to_id);
        }
        Ok(())
    }

    async fn shutdown(&mut self) {}
}

enum SchedulerMessage {
    Schedule {
        raw_graph: RawGraph,
        reply_to: OneshotSender<Result<()>>,
    },
    Shutdown {
        reply_to: OneshotSender<()>,
    },
}

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

pub fn new() -> SchedulerHandle {
    Scheduler {}.spawn()
}
