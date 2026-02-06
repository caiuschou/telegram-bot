//! Handler trait and transport conversion traits.

use async_trait::async_trait;

use super::{
    message::Message,
    response::HandlerResponse,
    user::User,
};

/// Converts a transport-specific user type to core [`User`].
pub trait ToCoreUser: Send + Sync {
    fn to_core(&self) -> User;
}

/// Converts a transport-specific message type to core [`Message`].
pub trait ToCoreMessage: Send + Sync {
    fn to_core(&self) -> Message;
}

/// Single handler concept: optional before / handle / after. Chain runs all before → handle until Stop/Reply → all after (reverse).
#[async_trait]
pub trait Handler: Send + Sync {
    /// Runs before the handle phase. Return false to stop the chain.
    async fn before(&self, _message: &Message) -> crate::core::error::Result<bool> {
        Ok(true)
    }
    /// Processes the message. Return Stop or Reply to end the handle phase. Default: Continue.
    async fn handle(&self, _message: &Message) -> crate::core::error::Result<HandlerResponse> {
        Ok(HandlerResponse::Continue)
    }
    /// Runs after the handle phase (reverse order), with the final response.
    async fn after(
        &self,
        _message: &Message,
        _response: &HandlerResponse,
    ) -> crate::core::error::Result<()> {
        Ok(())
    }
}
