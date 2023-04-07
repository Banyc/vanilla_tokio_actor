use async_trait::async_trait;
use derivative::Derivative;
use duplicate::duplicate_item;
use tokio::sync::mpsc;

#[duplicate_item(
     Actor        ActorState       ;
    [Actor]      [ActorState]      ;
    [ActorAsync] [ActorStateAsync] ;
)]
#[derive(Debug)]
struct Actor<M, S>
where
    S: ActorState<Message = M>,
{
    receiver: mpsc::Receiver<M>,
    state: S,
}

#[duplicate_item(
     Actor        ActorState       ;
    [Actor]      [ActorState]      ;
    [ActorAsync] [ActorStateAsync] ;
)]
impl<M, S> Actor<M, S>
where
    S: ActorState<Message = M>,
{
    fn new(receiver: mpsc::Receiver<M>, state: S) -> Self {
        Self { receiver, state }
    }

    fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }
}

#[duplicate_item(
     run_actor         Actor        ActorState        add_await(code) ;
    [run_actor]       [Actor]      [ActorState]      [code]           ;
    [run_actor_async] [ActorAsync] [ActorStateAsync] [code.await]     ;
)]
async fn run_actor<M, S>(mut actor: Actor<M, S>)
where
    S: ActorState<Message = M>,
{
    while let Some(msg) = actor.receiver.recv().await {
        let state = actor.state_mut();
        add_await([state.handle_message(msg)]);
    }
}

pub trait ActorState {
    type Message;
    fn handle_message(&mut self, msg: Self::Message);
}

#[async_trait]
pub trait ActorStateAsync {
    type Message;
    async fn handle_message(&mut self, msg: Self::Message);
}

#[duplicate_item(
    ActorHandle       ;
   [ActorHandle]      ;
   [ActorHandleAsync] ;
)]
#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct ActorHandle<M> {
    sender: mpsc::Sender<M>,
}

#[duplicate_item(
    ActorHandle        ActorState        Actor        run_actor        ;
   [ActorHandle]      [ActorState]      [Actor]      [run_actor]       ;
   [ActorHandleAsync] [ActorStateAsync] [ActorAsync] [run_actor_async] ;
)]
impl<M> ActorHandle<M>
where
    M: Send + 'static,
{
    pub fn new<S>(state: S, channel_buffer: usize) -> Self
    where
        S: ActorState<Message = M> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(channel_buffer);
        let actor = Actor::new(receiver, state);
        tokio::spawn(run_actor(actor));

        Self { sender }
    }

    pub async fn ask(&self, msg: M) -> Result<(), mpsc::error::SendError<M>> {
        self.sender.send(msg).await
    }
}
