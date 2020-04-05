use teloxide::{dispatching::update_listeners, prelude::*};

use reqwest::StatusCode;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::mpsc;
use warp::Filter;

async fn handle_rejection(error: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    log::error!("Cannot process the request due to: {:?}", error);
    Ok(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn webhook<'a>(
    bot: Arc<Bot>,
    host: &str,
    port: u16,
) -> impl update_listeners::UpdateListener<Infallible> {
    let token = bot.token();
    let path = format!("bot{}", token);
    bot.set_webhook(format!("{}/{}", host, path))
        .send()
        .await
        .expect("Cannot setup a webhook");

    let (tx, rx) = mpsc::unbounded_channel();

    let server = warp::post()
        .and(warp::path(path)) // https://12345.ngrok.com/<token>
        .and(warp::body::json())
        .map(move |json: serde_json::Value| {
            let try_parse = match serde_json::from_str(&json.to_string()) {
                Ok(update) => Ok(update),
                Err(error) => {
                    log::error!(
                        "Cannot parse an update.\nError: {:?}\nValue: {}\n\
                       This is a bug in teloxide, please open an issue here: \
                       https://github.com/teloxide/teloxide/issues.",
                        error,
                        json
                    );
                    Err(error)
                }
            };
            if let Ok(update) = try_parse {
                tx.send(Ok(update))
                    .expect("Cannot send an incoming update from the webhook")
            }

            StatusCode::OK
        })
        .recover(handle_rejection);

    let serve = warp::serve(server);

    tokio::spawn(serve.run(([0, 0, 0, 0], port)));
    log::info!("Running on localhost:{}", port);
    rx
}
