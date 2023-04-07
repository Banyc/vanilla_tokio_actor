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
    keep_alive: mpsc::Receiver<()>,
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
    fn new(receiver: mpsc::Receiver<M>, state: S, keep_alive: mpsc::Receiver<()>) -> Self {
        Self {
            receiver,
            state,
            keep_alive,
        }
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
    loop {
        tokio::select! {
            Some(msg) = actor.receiver.recv() => {
                let state = actor.state_mut();
                add_await([state.handle_message(msg)]);
            }
            opt = actor.keep_alive.recv() => {
                match opt {
                    Some(_) => panic!("Actor keep alive channel should never receive a message"),
                    None => break,
                }
            }
        }
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
    sender: ActorHandleWeak<M>,
    keep_alive: mpsc::Sender<()>,
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
    /// # Panics
    ///
    /// Panics if the buffer capacity is 0.
    pub fn new<S>(state: S, channel_buffer: usize) -> Self
    where
        S: ActorState<Message = M> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(channel_buffer);
        let (keep_alive, keep_alive_rx) = mpsc::channel(1);
        let actor = Actor::new(receiver, state, keep_alive_rx);
        tokio::spawn(run_actor(actor));

        let sender = ActorHandleWeak { sender };
        Self { sender, keep_alive }
    }

    pub async fn ask(&self, msg: M) -> Result<(), mpsc::error::SendError<M>> {
        self.sender.ask(msg).await
    }

    pub fn downgrade(&self) -> ActorHandleWeak<M> {
        self.sender.clone()
    }
}

#[derive(Debug, Derivative)]
#[derivative(Clone(bound = ""))]
pub struct ActorHandleWeak<M> {
    sender: mpsc::Sender<M>,
}

impl<M> ActorHandleWeak<M> {
    pub async fn ask(&self, msg: M) -> Result<(), mpsc::error::SendError<M>> {
        self.sender.send(msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyState;

    impl ActorState for EmptyState {
        type Message = ();
        fn handle_message(&mut self, _msg: Self::Message) {}
    }

    #[tokio::test]
    async fn weak() {
        let handle = ActorHandle::new(EmptyState, 1);
        let weak = handle.downgrade();
        handle.ask(()).await.unwrap();
        weak.ask(()).await.unwrap();
        handle.ask(()).await.unwrap();
        weak.ask(()).await.unwrap();
        drop(handle);
        assert!(weak.ask(()).await.is_err());
    }
}
