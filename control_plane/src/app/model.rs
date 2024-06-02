use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Workspace {
    pub name: String,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

//pub struct MessageStream {
//    pub id: i64,
//    pub origin: String,
//    pub stream_type: String,
//    pub stream: BoxStream<'static, Result<Vec<u8>>>,
//}
//
//// FIXME: rename clients do daemons
//#[derive(Serialize, Deserialize, Debug)]
//pub struct Clients {
//    pub clients: Vec<Client>,
//}
//
//// FIXME: rename clients to daemons
//#[derive(Serialize, Deserialize, Debug)]
//pub struct Client {
//    pub id: String,
//    pub display_name: String,
//    pub sources: Vec<Source>,
//    pub destinations: Vec<Destination>,
//}
