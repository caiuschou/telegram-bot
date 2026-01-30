//! # Telegram bot LLM layer
//!
//! [`TelegramBotLLM`] wraps an LLM client (e.g. [`llm_client::OpenAILlmClient`]) and @-mention parsing.
//! It provides `handle_message` and `handle_message_stream` for teloxide: when the bot is mentioned,
//! it extracts the question and calls the LLM, then sends the reply (or streamed chunks) back.

use anyhow::Result;
use llm_client::{LlmClient, OpenAILlmClient};
use prompt::ChatMessage;
use teloxide::{prelude::*, types::Message};
use tracing::{error, info, instrument};

/// Bot that responds to @-mentions with LLM-generated replies (single or streamed).
#[derive(Clone)]
pub struct TelegramBotLLM {
    /// Bot username used to detect @bot_username mentions.
    bot_username: String,
    /// Underlying LLM client (e.g. OpenAI).
    llm_client: OpenAILlmClient,
}

impl TelegramBotLLM {
    /// Creates a new bot that uses the given username for mention detection and the given LLM client.
    pub fn new(bot_username: String, llm_client: OpenAILlmClient) -> Self {
        Self {
            bot_username,
            llm_client,
        }
    }

    /// Handles one message: if the bot is @-mentioned, gets LLM reply and sends it in one message.
    #[instrument(skip(self, bot, msg))]
    pub async fn handle_message(&self, bot: Bot, msg: Message) -> Result<()> {
        let text = msg
            .text()
            .ok_or_else(|| anyhow::anyhow!("No text in message"))?;

        if self.is_bot_mentioned(text) {
            let user_question = self.extract_question(text);
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            info!(user_id = user_id, question = %user_question, "Bot mentioned by user");

            match self.get_llm_response(&user_question).await {
                Ok(response) => {
                    bot.send_message(msg.chat.id, response).await?;
                    info!(user_id = user_id, "Sent LLM response to user");
                }
                Err(e) => {
                    let error_msg = format!("Sorry, something went wrong processing your request: {}", e);
                    bot.send_message(msg.chat.id, error_msg).await?;
                    error!(user_id = user_id, error = %e, "LLM response error");
                }
            }
        }

        Ok(())
    }

    /// Handles one message with streamed reply: each chunk is sent as a separate Telegram message.
    #[instrument(skip(self, bot, msg))]
    pub async fn handle_message_stream(&self, bot: Bot, msg: Message) -> Result<()> {
        let text = msg
            .text()
            .ok_or_else(|| anyhow::anyhow!("No text in message"))?;

        if self.is_bot_mentioned(text) {
            let user_question = self.extract_question(text);
            let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            info!(user_id = user_id, question = %user_question, "Bot mentioned by user");

            let chat_id = msg.chat.id;

            let bot_clone = bot.clone();
            if let Err(e) = self
                .get_llm_response_stream(&user_question, |chunk| {
                    let bot = bot_clone.clone();
                    async move {
                        if !chunk.content.is_empty() {
                            if let Err(e) = bot.send_message(chat_id, chunk.content).await {
                                error!(error = %e, "Failed to send stream chunk");
                                return Err(anyhow::anyhow!("Failed to send message: {}", e));
                            }
                        }
                        Ok(())
                    }
                })
                .await
            {
                let error_msg = format!("Sorry, something went wrong processing your request: {}", e);
                bot.send_message(msg.chat.id, error_msg).await?;
                error!(user_id = user_id, error = %e, "LLM stream response error");
            } else {
                info!(user_id = user_id, "Sent LLM stream response to user");
            }
        }

        Ok(())
    }

    /// Returns true if the message text contains @bot_username.
    fn is_bot_mentioned(&self, text: &str) -> bool {
        let mention = format!("@{}", self.bot_username);
        text.contains(&mention)
    }

    /// Removes @bot_username from text and trims; used as the user question for the LLM.
    fn extract_question(&self, text: &str) -> String {
        let mention = format!("@{}", self.bot_username);
        text.replace(&mention, "").trim().to_string()
    }

    /// Single non-streamed LLM reply for the given question (one user message).
    #[instrument(skip(self))]
    pub async fn get_llm_response(&self, question: &str) -> Result<String> {
        self.llm_client
            .get_llm_response_with_messages(vec![ChatMessage::user(question)])
            .await
    }

    /// Non-streamed LLM reply for an arbitrary message list (e.g. for context/conversation).
    #[instrument(skip(self))]
    pub async fn get_llm_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.llm_client.get_llm_response_with_messages(messages).await
    }

    /// Streamed LLM reply for a single question; invokes callback for each chunk.
    #[instrument(skip(self, callback))]
    pub async fn get_llm_response_stream<F, Fut>(
        &self,
        question: &str,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(llm_client::StreamChunk) -> Fut + Send,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        self.llm_client
            .get_llm_response_stream_with_messages(vec![ChatMessage::user(question)], callback)
            .await
    }

    /// Streamed LLM reply for an arbitrary message list; invokes callback for each chunk.
    #[instrument(skip(self, messages, callback))]
    pub async fn get_llm_response_stream_with_messages<F, Fut>(
        &self,
        messages: Vec<ChatMessage>,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(llm_client::StreamChunk) -> Fut + Send,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        self.llm_client
            .get_llm_response_stream_with_messages(messages, callback)
            .await
    }
}
