//! Graph manager
//!
//! Performs updates over workspaces, keeps state of the graph to validate operations

use std::{collections::BTreeMap, sync::Arc};

use super::db;

pub struct Graph;

pub struct GraphManager {
    db: Arc<dyn db::DbTrait>,
    cache: BTreeMap<String, Graph>,
}
