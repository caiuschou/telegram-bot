//! 简单 LLM 机器人：TelegramBotLLM 封装 LLM 调用（llm-client）与 @ 提及解析，提供 handle_message / handle_message_stream。
//! 与外部交互：teloxide Bot 发消息，llm_client::OpenAILlmClient 调 LLM。

use anyhow::Result;
use llm_client::{LlmClient, OpenAILlmClient};
use prompt::ChatMessage;
use teloxide::{prelude::*, types::Message};
use tracing::{error, info, instrument};

#[derive(Clone)]
pub struct TelegramBotLLM {
    bot_username: String,
    llm_client: OpenAILlmClient,
}

impl TelegramBotLLM {
    pub fn new(bot_username: String, llm_client: OpenAILlmClient) -> Self {
        Self {
            bot_username,
            llm_client,
        }
    }

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
                    let error_msg = format!("抱歉，处理您的请求时出错: {}", e);
                    bot.send_message(msg.chat.id, error_msg).await?;
                    error!(user_id = user_id, error = %e, "LLM response error");
                }
            }
        }

        Ok(())
    }

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
                let error_msg = format!("抱歉，处理您的请求时出错: {}", e);
                bot.send_message(msg.chat.id, error_msg).await?;
                error!(user_id = user_id, error = %e, "LLM stream response error");
            } else {
                info!(user_id = user_id, "Sent LLM stream response to user");
            }
        }

        Ok(())
    }

    fn is_bot_mentioned(&self, text: &str) -> bool {
        let mention = format!("@{}", self.bot_username);
        text.contains(&mention)
    }

    fn extract_question(&self, text: &str) -> String {
        let mention = format!("@{}", self.bot_username);
        text.replace(&mention, "").trim().to_string()
    }

    #[instrument(skip(self))]
    pub async fn get_llm_response(&self, question: &str) -> Result<String> {
        self.llm_client
            .get_llm_response_with_messages(vec![ChatMessage::user(question)])
            .await
    }

    #[instrument(skip(self))]
    pub async fn get_llm_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.llm_client.get_llm_response_with_messages(messages).await
    }

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
