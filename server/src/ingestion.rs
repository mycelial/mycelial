use std::{convert::Infallible, pin::Pin, sync::Arc};
use axum::{
    body::{Body, Bytes},
    extract::State,
    response::IntoResponse,
    Json,
};
use futures::{stream::BoxStream, Stream, StreamExt};
use sqlx::Connection;
use crate::{model::MessageStream, App, AppError};

pub async fn ingestion(
    State(app): State<Arc<App>>,
    axum::extract::Path(topic): axum::extract::Path<String>,
    headers: axum::http::header::HeaderMap,
    body: Body,
) -> crate::Result<impl IntoResponse> {
    let origin = match headers.get("x-message-origin") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| anyhow::anyhow!("bad x-message-origin header value"))?,
        None => Err(anyhow::anyhow!("bad request"))?,
    };

    let stream_type = match headers.get("x-stream-type") {
        Some(origin) => origin
            .to_str()
            .map_err(|_| anyhow::anyhow!("bad x-message-origin header value"))?,
        None => "dataframe", // by default
    };

    let stream: BoxStream<crate::Result<Vec<u8>>> = Box::pin(
        body.into_data_stream()
            .map(|chunk| chunk
                 .map(|bytes| bytes.to_vec())
                 .map_err(|e| -> AppError { e.into() })
             )
    );
    // FIXME: do not touch db
    app.db.new_message(topic.as_str(), origin, stream_type, stream).await?;
    Ok(Json("ok"))
}

pub async fn get_message(
    State(app): State<Arc<App>>,
    axum::extract::Path((topic, offset)): axum::extract::Path<(String, u64)>,
) -> crate::Result<impl IntoResponse> {
    Err(anyhow::anyhow!("unimplemented"))?;
    Ok("")

  //let response = match app.database.get_message(&topic, offset).await? {
  //    None => {
  //        let stream: Pin<Box<dyn Stream<Item = _> + Send>> =
  //            Box::pin(futures::stream::empty::<Result<Vec<u8>, Infallible>>());
  //        (
  //            [
  //                ("x-message-id", offset.to_string()),
  //                ("x-message-origin", "".into()),
  //                ("x-stream-type", "".into()),
  //            ],
  //            Body::from_stream(stream),
  //        )
  //    }
  //    Some(MessageStream {
  //        id,
  //        origin,
  //        stream_type,
  //        stream,
  //    }) => (
  //        [
  //            ("x-message-id", id.to_string()),
  //            ("x-message-origin", origin),
  //            ("x-stream-type", stream_type),
  //        ],
  //        Body::from_stream(stream),
  //    ),
  //};
  //Ok(response)
}
