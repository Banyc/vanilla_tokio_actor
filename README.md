# vanilla_tokio_actor

This project faithfully follows [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/) and encapsulates template code into a library.

"Multiple handle structs for one actor" is not implemented.

## Usage

Steps:

1. Define the message type that the actor can handle.

   ```rust
   enum FooMessage {
       GetUniqueId { respond_to: oneshot::Sender<u32> },
   }
   ```

1. Define the actor struct.

   ```rust
   struct FooState {
       unique_id: u32,
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
   ```

1. Define the actor handle struct.

   ```rust
   #[derive(Debug, Clone)]
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

           recv.await.expect("Actor is dead")
       }
   }
   ```

1. Use the actor handle.

   ```rust
   let foo = FooHandle::new();
   let id = foo.get_unique_id().await;
   assert_eq!(id, 0);
   ```
