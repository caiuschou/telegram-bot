use anyhow::Result;
use openai_client::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, OpenAIClient,
};
use prompt::{ChatMessage, MessageRole};
use teloxide::{prelude::*, types::Message};
use tracing::{error, info, instrument};

fn chat_message_to_openai(msg: &ChatMessage) -> anyhow::Result<ChatCompletionRequestMessage> {
    use openai_client::ChatCompletionRequestAssistantMessageArgs;
    let content = msg.content.clone();
    let openai_msg: ChatCompletionRequestMessage = match msg.role {
        MessageRole::System => ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .build()?
            .into(),
        MessageRole::User => ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
        MessageRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    };
    Ok(openai_msg)
}

#[derive(Clone)]
pub struct TelegramBotAI {
    bot_username: String,
    openai_client: OpenAIClient,
    model: String,
    /// 系统提示词；未设置时使用 DEFAULT_SYSTEM_CONTENT。可从 .env 的 AI_SYSTEM_PROMPT 注入。
    system_prompt: Option<String>,
}

impl TelegramBotAI {
    pub fn new(bot_username: String, openai_client: OpenAIClient) -> Self {
        Self {
            bot_username,
            openai_client,
            model: "gpt-3.5-turbo".to_string(),
            system_prompt: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// 设置系统提示词；未设置时使用内置默认。与外部交互：该内容会作为 OpenAI system 消息发送。
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// 使用可选系统提示词；为 None 时使用内置默认。
    pub fn with_system_prompt_opt(mut self, prompt: Option<String>) -> Self {
        self.system_prompt = prompt;
        self
    }

    fn system_content(&self) -> &str {
        self.system_prompt
            .as_deref()
            .unwrap_or(Self::DEFAULT_SYSTEM_CONTENT)
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
        self.get_ai_response_with_messages(vec![ChatMessage::user(question)])
            .await
    }

    /// 与外部交互：作为 OpenAI 的 system 消息，影响模型回复风格。
    const DEFAULT_SYSTEM_CONTENT: &'static str = "不要使用 Markdown 或任何格式化符号（如*、_、`、#等），只输出纯文本，适合在 Telegram 里直接发送。";

    /// Sends messages that map one-to-one to OpenAI `messages` (e.g. from `prompt::format_for_model_as_messages` or `Context::to_messages`).
    /// Prepends a system message (from .env AI_SYSTEM_PROMPT or DEFAULT_SYSTEM_CONTENT).
    #[instrument(skip(self, messages))]
    pub async fn get_ai_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let mut openai_messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_content())
                .build()?
                .into(),
        ];
        for msg in &messages {
            openai_messages.push(chat_message_to_openai(msg)?);
        }
        self.openai_client
            .chat_completion(&self.model, openai_messages)
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
        self.get_ai_response_stream_with_messages(vec![ChatMessage::user(question)], callback)
            .await
    }

    /// Stream variant of `get_ai_response_with_messages`: messages map one-to-one to OpenAI; system message is prepended.
    #[instrument(skip(self, messages, callback))]
    pub async fn get_ai_response_stream_with_messages<F, Fut>(
        &self,
        messages: Vec<ChatMessage>,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(openai_client::StreamChunk) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut openai_messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_content())
                .build()?
                .into(),
        ];
        for msg in &messages {
            openai_messages.push(chat_message_to_openai(msg)?);
        }
        self.openai_client
            .chat_completion_stream(&self.model, openai_messages, callback)
            .await
            .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
    }
}
