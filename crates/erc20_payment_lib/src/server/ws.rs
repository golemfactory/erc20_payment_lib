use super::web::ServerData;
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, StreamHandler};
use actix_web::web::Data;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use erc20_payment_lib_common::DriverEvent;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

struct StopMessage {}

impl Message for StopMessage {
    type Result = ();
}

struct MainWebsocketActor {
    rx: Option<broadcast::Receiver<DriverEvent>>,
    second_actor: Option<Addr<SecondaryActor>>,
}

impl MainWebsocketActor {
    pub fn new(rec: broadcast::Receiver<DriverEvent>) -> Self {
        Self {
            rx: Some(rec),
            second_actor: None,
        }
    }
}

impl Actor for MainWebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<DriverEvent> for MainWebsocketActor {
    type Result = ();

    fn handle(&mut self, msg: DriverEvent, ctx: &mut Self::Context) {
        if let Ok(msg) = serde_json::to_string(&msg) {
            ctx.text(msg);
        } else {
            log::error!("Failed to serialize DriverEvent");
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
        self.second_actor = Some(
            SecondaryActor {
                rx: self.rx.take(),
                ws_actor: ctx.address(),
            }
            .start(),
        );
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        if let Some(addr) = self.second_actor.take() {
            addr.do_send(StopMessage {});
        }
        ctx.stop();
    }
}

struct SecondaryActor {
    rx: Option<broadcast::Receiver<DriverEvent>>,
    ws_actor: Addr<MainWebsocketActor>,
}

impl StreamHandler<Result<DriverEvent, BroadcastStreamRecvError>> for SecondaryActor {
    fn handle(
        &mut self,
        msg: Result<DriverEvent, BroadcastStreamRecvError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(event) => {
                self.ws_actor.try_send(event).unwrap_or_else(|_err| {
                    ctx.stop();
                });
            }
            Err(err) => {
                log::error!("SecondaryActor handle error: {:?}", err);
                ctx.stop();
            }
        }
    }
}

impl Handler<StopMessage> for SecondaryActor {
    type Result = ();

    fn handle(&mut self, _msg: StopMessage, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

impl Actor for SecondaryActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("SecondaryActor started");
        if let Some(rx) = self.rx.take() {
            let stream_wrapper: BroadcastStream<DriverEvent> = BroadcastStream::new(rx);
            Self::add_stream(stream_wrapper, ctx);
        } else {
            ctx.stop();
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("SecondaryActor stopped");
    }
}

pub async fn event_stream_websocket_endpoint(
    data: Data<Box<ServerData>>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    ws::start(
        MainWebsocketActor::new(data.payment_runtime.receiver.resubscribe()),
        &req,
        stream,
    )
}
