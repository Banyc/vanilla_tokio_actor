use tokio::sync::{mpsc, oneshot};

#[tokio::main]
#[allow(clippy::disallowed_names)]
async fn main() {
    let foo = FooActorHandle::new();
    let id = foo.get_unique_id().await;
    assert_eq!(id, 0);
    let id = foo.get_unique_id().await;
    assert_eq!(id, 1);

    let h = tokio::spawn({
        let foo = foo.clone();
        async move {
            let id = foo.get_unique_id().await;
            assert_eq!(id, 2);
        }
    });
    h.await.unwrap();

    let id = foo.get_unique_id().await;
    assert_eq!(id, 3);
}

struct FooActor {
    receiver: mpsc::Receiver<ActorMessage>,
    next_id: u32,
}
enum ActorMessage {
    GetUniqueId { respond_to: oneshot::Sender<u32> },
}

impl FooActor {
    fn new(receiver: mpsc::Receiver<ActorMessage>) -> Self {
        Self {
            receiver,
            next_id: 0,
        }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::GetUniqueId { respond_to } => {
                let id = self.next_id;
                self.next_id += 1;

                // This can happen if the handle decides not to wait for a response.
                let _ = respond_to.send(id);
            }
        }
    }
}

async fn run_foo_actor(mut actor: FooActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg);
    }
}

#[derive(Clone)]
pub struct FooActorHandle {
    sender: mpsc::Sender<ActorMessage>,
}

impl FooActorHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = FooActor::new(receiver);
        tokio::spawn(run_foo_actor(actor));

        Self { sender }
    }

    pub async fn get_unique_id(&self) -> u32 {
        let (send, recv) = oneshot::channel();

        // Ignore send errors.
        // If this send fails, so does the recv.await below.
        let _ = self
            .sender
            .send(ActorMessage::GetUniqueId { respond_to: send })
            .await;
        recv.await.expect("Actor is dead")
    }
}

impl Default for FooActorHandle {
    fn default() -> Self {
        Self::new()
    }
}
