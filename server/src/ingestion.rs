use std::{pin::Pin, sync::Arc};

use axum::{
    body::StreamBody,
    extract::{BodyStream, State},
    response::IntoResponse,
    Json,
};
use futures::{Stream, StreamExt};
use reqwest::StatusCode;
use sqlx::Connection;

use crate::{error, App, MessageStream};

pub async fn ingestion(
    State(app): State<Arc<App>>,
    axum::extract::Path(topic): axum::extract::Path<String>,
    headers: axum::http::header::HeaderMap,
    mut body: BodyStream,
) -> Result<impl IntoResponse, error::Error> {
    let origin = match headers.get("x-message-origin") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| "bad x-message-origin header value")?,
        None => Err(StatusCode::BAD_REQUEST)?,
    };

    let stream_type = match headers.get("x-stream-type") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| "bad x-message-origin header value")?,
        None => "dataframe", // by default
    };

    let mut connection = app.database.get_connection().await;
    let mut transaction = connection.begin().await?;

    let message_id = app
        .database
        .new_message(&mut transaction, topic.as_str(), origin, stream_type)
        .await?;

    let mut stored = 0;
    while let Some(chunk) = body.next().await {
        // FIXME: accumulate into buffer
        let chunk = chunk?;
        app.database
            .store_chunk(&mut transaction, message_id, chunk.as_ref())
            .await?;
        stored += 1;
    }
    // don't store empty messages
    match stored {
        0 => transaction.rollback().await?,
        _ => transaction.commit().await?,
    };
    Ok(Json("ok"))
}

pub async fn get_message(
    State(app): State<Arc<App>>,
    axum::extract::Path((topic, offset)): axum::extract::Path<(String, u64)>,
) -> Result<impl IntoResponse, error::Error> {
    let response = match app.database.get_message(&topic, offset).await? {
        None => {
            let stream: Pin<Box<dyn Stream<Item = _> + Send>> =
                Box::pin(futures::stream::empty::<Result<Vec<u8>, error::Error>>());
            (
                [
                    ("x-message-id", offset.to_string()),
                    ("x-message-origin", "".into()),
                    ("x-stream-type", "".into()),
                ],
                StreamBody::new(stream),
            )
        }
        Some(MessageStream {
            id,
            origin,
            stream_type,
            stream,
        }) => (
            [
                ("x-message-id", id.to_string()),
                ("x-message-origin", origin),
                ("x-stream-type", stream_type),
            ],
            StreamBody::new(stream),
        ),
    };
    Ok(response)
}
