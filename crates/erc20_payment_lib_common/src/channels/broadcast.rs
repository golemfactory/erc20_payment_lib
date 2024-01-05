use std::marker::PhantomData;
use tokio::sync::broadcast;
use crate::DriverEvent;


struct ZeroBroadcastSender<T>
where T: std::clone::Clone
{
    //phantom T
    t: PhantomData<T>,
    channel: ZeroBroadcastChannel<T>,
}

impl<T: std::clone::Clone> ZeroBroadcastSender<T> {
    fn subscribe(&mut self) -> ZeroBroadcastReceiver<T> {
        if self.channel.broadcast_sender.is_none() {
            let (tx, _) = broadcast::channel(10);
            self.channel.broadcast_sender = Some(tx);
        }

        return ZeroBroadcastReceiver {
            t: PhantomData
        };
    }
}

struct ZeroBroadcastReceiver<T> {
    t: PhantomData<T>
}


struct ZeroBroadcastChannel<T> {
    broadcast_sender: Option<broadcast::Sender<T>>,
}

impl<T: std::clone::Clone> ZeroBroadcastChannel<T> {


    fn channel() -> ZeroBroadcastSender<T> {
        return ZeroBroadcastSender {
            t: PhantomData,
            channel: ZeroBroadcastChannel {
                broadcast_sender: None,
            }
        };
    }
}


#[tokio::test(flavor = "multi_thread")]
async fn payment_lib_broadcast_channel_test() -> Result<(), anyhow::Error> {

    let mut sender = ZeroBroadcastChannel::<i32>::channel();

    let receiver1 = sender.subscribe();
    let receiver2 = sender.subscribe();

    Ok(())
}