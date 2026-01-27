use anyhow::Result;
use openai_client::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, OpenAIClient,
};
use teloxide::{prelude::*, types::Message};
use tracing::{debug, error, info, instrument};

#[derive(Clone)]
pub struct TelegramBotAI {
    bot_username: String,
    openai_client: OpenAIClient,
    model: String,
}

impl TelegramBotAI {
    pub fn new(bot_username: String, openai_client: OpenAIClient) -> Self {
        Self {
            bot_username,
            openai_client,
            model: "gpt-3.5-turbo".to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
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

            match self.get_ai_response(&user_question).await {
                Ok(response) => {
                    bot.send_message(msg.chat.id, response).await?;
                    info!(user_id = user_id, "Sent AI response to user");
                }
                Err(e) => {
                    let error_msg = format!("抱歉，处理您的请求时出错: {}", e);
                    bot.send_message(msg.chat.id, error_msg).await?;
                    error!(user_id = user_id, error = %e, "AI response error");
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

            if let Err(e) = self
                .get_ai_response_stream(&user_question, |chunk| async {
                    if !chunk.content.is_empty() {
                        if let Err(e) = bot.send_message(chat_id, chunk.content).await {
                            error!(error = %e, "Failed to send stream chunk");
                            return Err(anyhow::anyhow!("Failed to send message: {}", e));
                        }
                    }
                    Ok(())
                })
                .await
            {
                let error_msg = format!("抱歉，处理您的请求时出错: {}", e);
                bot.send_message(msg.chat.id, error_msg).await?;
                error!(user_id = user_id, error = %e, "AI stream response error");
            } else {
                info!(user_id = user_id, "Sent AI stream response to user");
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
    pub async fn get_ai_response(&self, question: &str) -> Result<String> {
        debug!(question = %question, model = %self.model, "Getting AI response");

        let system_message: ChatCompletionRequestMessage =
            ChatCompletionRequestSystemMessageArgs::default()
                .content("你是一个有用的助手，用中文回答问题。不要使用Markdown格式，不要使用任何格式化符号（如*、_、`、#等），输出纯文本，适合Telegram消息。")
                .build()?
                .into();

        let user_message: ChatCompletionRequestMessage =
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into();

        let messages = vec![system_message, user_message];

        self.openai_client
            .chat_completion(&self.model, messages)
            .await
    }

    #[instrument(skip(self, callback))]
    pub async fn get_ai_response_stream<F, Fut>(
        &self,
        question: &str,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(openai_client::StreamChunk) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        debug!(question = %question, model = %self.model, "Getting AI stream response");

        let system_message: ChatCompletionRequestMessage =
            ChatCompletionRequestSystemMessageArgs::default()
                .content("你是一个有用的助手，用中文回答问题。不要使用Markdown格式，不要使用任何格式化符号（如*、_、`、#等），输出纯文本，适合Telegram消息。")
                .build()?
                .into();

        let user_message: ChatCompletionRequestMessage =
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into();

        let messages = vec![system_message, user_message];

        self.openai_client
            .chat_completion_stream(&self.model, messages, callback)
            .await
            .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
    }
}
