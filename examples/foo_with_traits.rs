use derivative::Derivative;
use tokio::sync::oneshot;
use vanilla_tokio_actor::{ActorHandle, ActorState};

const CHANNEL_SIZE: usize = 8;

#[tokio::main]
#[allow(clippy::disallowed_names)]
async fn main() {
    let foo = FooHandle::new();
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

#[derive(Debug)]
enum FooMessage {
    GetUniqueId { respond_to: oneshot::Sender<u32> },
}

struct FooState {
    pub unique_id: u32,
}

impl FooState {
    pub fn new() -> Self {
        Self { unique_id: 0 }
    }
}

impl ActorState for FooState {
    type Message = FooMessage;

    fn handle_message(&mut self, msg: Self::Message) {
        match msg {
            FooMessage::GetUniqueId { respond_to } => {
                let id = self.unique_id;
                self.unique_id += 1;
                let _ = respond_to.send(id);
            }
        }
    }
}

#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct FooHandle {
    handle: ActorHandle<FooMessage>,
}

impl FooHandle {
    pub fn new() -> Self {
        let state = FooState::new();
        let handle = ActorHandle::new(state, CHANNEL_SIZE);

        Self { handle }
    }

    pub async fn get_unique_id(&self) -> u32 {
        let (send, recv) = oneshot::channel();

        // Ignore send errors.
        // If this send fails, so does the recv.await below.
        let _ = self
            .handle
            .ask(FooMessage::GetUniqueId { respond_to: send })
            .await;

        recv.await.unwrap()
    }
}

impl Default for FooHandle {
    fn default() -> Self {
        Self::new()
    }
}
