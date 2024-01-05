use super::web::ServerData;
use actix::{Actor, StreamHandler};
use actix_web::web::Data;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use erc20_payment_lib_common::DriverEvent;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

struct MainWebsocketActor {
    rx: Option<broadcast::Receiver<DriverEvent>>,
}

impl MainWebsocketActor {
    pub fn new(rec: broadcast::Receiver<DriverEvent>) -> Self {
        Self { rx: Some(rec) }
    }
}

impl Actor for MainWebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<DriverEvent, BroadcastStreamRecvError>> for MainWebsocketActor {
    fn handle(
        &mut self,
        msg: Result<DriverEvent, BroadcastStreamRecvError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(event) => {
                ctx.text(serde_json::to_string(&event).expect("Failed to serialize DriverEvent"));
            }
            Err(BroadcastStreamRecvError::Lagged(n)) => {
                log::warn!("Websocket actor skipped {} messages", n);
            }
        }
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MainWebsocketActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(_text)) => {
                //ignore data from client
            }
            Ok(ws::Message::Binary(_bin)) => {
                //ignore data from client
            }
            _ => (),
        }
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        let stream_wrapper: BroadcastStream<DriverEvent> =
            BroadcastStream::new(self.rx.take().unwrap());
        <Self as StreamHandler<Result<DriverEvent, BroadcastStreamRecvError>>>::add_stream(
            stream_wrapper,
            ctx,
        );
    }
}

pub async fn event_stream_websocket_endpoint(
    data: Data<Box<ServerData>>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    if let Some(driver_broadcast_sender) = &data.payment_runtime.driver_broadcast_sender {
        ws::start(
            MainWebsocketActor::new(driver_broadcast_sender.subscribe()),
            &req,
            stream,
        )
    } else {
        Err(actix_web::error::ErrorInternalServerError(
            "Driver event sender not available",
        ))
    }
}
