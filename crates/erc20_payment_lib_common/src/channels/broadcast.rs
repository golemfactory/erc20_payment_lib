use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

pub struct ZeroBroadcastSender<T>
where
    T: std::clone::Clone,
{
    channel: Arc<Mutex<ZeroBroadcastChannel<T>>>,
}

impl<T: std::clone::Clone> Drop for ZeroBroadcastReceiver<T> {
    fn drop(&mut self) {
        let mut rw = self.channel.lock().expect("poisoned");
        if rw.receivers_count == 0 {
            panic!("ZeroBroadcastSender dropped without any receivers");
        }
        rw.receivers_count -= 1;
        if rw.receivers_count == 0 {
            log::debug!("Last receiver dropped, dropping broadcast sender");
            rw.broadcast_sender = None;
        } else {
            log::debug!("Receiver dropped, {} receivers left", rw.receivers_count);
        }
    }
}

impl<T: std::clone::Clone + std::fmt::Debug> ZeroBroadcastSender<T> {
    pub fn subscribe(&mut self) -> ZeroBroadcastReceiver<T> {
        let mut channel_rw_guard = self.channel.lock().expect("poisoned");
        let new_receiver =
            if let Some(broadcast_sender) = channel_rw_guard.broadcast_sender.as_ref() {
                log::debug!(
                    "Reusing existing broadcast channel, adding receiver no {}",
                    channel_rw_guard.receivers_count + 1
                );
                let rec = broadcast_sender.subscribe();
                ZeroBroadcastReceiver {
                    receiver: rec,
                    channel: self.channel.clone(),
                }
            } else {
                log::debug!(
                "No receivers subscribed yet - creating broadcast channel, adding receiver no {}",
                channel_rw_guard.receivers_count + 1
            );
                let (tx, rec) = broadcast::channel(10);
                channel_rw_guard.broadcast_sender = Some(tx);

                ZeroBroadcastReceiver {
                    receiver: rec,
                    channel: self.channel.clone(),
                }
            };
        channel_rw_guard.receivers_count += 1;
        new_receiver
    }

    pub fn send(&mut self, msg: T) -> usize {
        if let Some(broadcast_sender) = self
            .channel
            .lock()
            .expect("poisoned")
            .broadcast_sender
            .as_ref()
        {
            match broadcast_sender.send(msg) {
                Ok(msg) => msg,
                Err(_err) => {
                    log::debug!("Broadcast sender dropped, dropping broadcast sender");
                    //A send operation can only fail if there are no active receivers
                    //that is ok in our case and it means, that broadcast sender will be dropped in the moment
                    0
                }
            }
        } else {
            0
        }
    }
}

pub struct ZeroBroadcastReceiver<T>
where
    T: std::clone::Clone,
{
    receiver: broadcast::Receiver<T>,
    channel: Arc<Mutex<ZeroBroadcastChannel<T>>>,
}

impl<T: std::clone::Clone> ZeroBroadcastReceiver<T> {
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        match self.receiver.recv().await {
            Ok(msg) => Ok(msg),
            Err(err) => Err(err),
        }
    }
}

pub struct ZeroBroadcastChannel<T> {
    broadcast_sender: Option<broadcast::Sender<T>>,
    receivers_count: i32,
}

impl<T: std::clone::Clone> ZeroBroadcastChannel<T> {
    pub fn channel() -> ZeroBroadcastSender<T> {
        ZeroBroadcastSender {
            channel: Arc::new(Mutex::new(ZeroBroadcastChannel {
                broadcast_sender: None,
                receivers_count: 0,
            })),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn payment_lib_broadcast_channel_test() -> Result<(), anyhow::Error> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    let mut sender = ZeroBroadcastChannel::<i32>::channel();

    log::debug!("Sender created");
    {
        let receiver1 = sender.subscribe();
        let _receiver2 = sender.subscribe();

        let tsk1 = tokio::task::spawn(async move {
            let mut receiver1 = receiver1;
            while let Ok(msg) = receiver1.recv().await {
                log::info!("Received message: {:?}", msg);
            }
            log::error!("Receiver1 finished");
        });

        for _i in 0..10 {
            println!("Sent to {} receivers", sender.send(0));
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tsk1.abort();
    }
    log::debug!("Sender finished");
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("Sent to {} receivers", sender.send(0));
    log::debug!("Sender created");
    {
        let receiver1 = sender.subscribe();

        tokio::task::spawn(async move {
            let mut receiver1 = receiver1;
            while let Ok(msg) = receiver1.recv().await {
                log::info!("Received message: {:?}", msg);
            }
        });

        for _i in 0..10 {
            println!("Sent to {} receivers", sender.send(0));
        }
    }
    log::debug!("Sender finished");
    Ok(())
}
