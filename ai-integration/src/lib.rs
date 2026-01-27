use async_trait::async_trait;
use dbot_core::Result;
use openai_client::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, OpenAIClient,
};

#[derive(Clone)]
pub struct TelegramBotAI {
    openai_client: OpenAIClient,
    model: String,
}

impl TelegramBotAI {
    pub fn new(openai_client: OpenAIClient) -> Self {
        Self {
            openai_client,
            model: "gpt-3.5-turbo".to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub async fn get_ai_response(&self, question: &str) -> Result<String> {
        let system_message: ChatCompletionRequestMessage =
            ChatCompletionRequestSystemMessageArgs::default()
                .content("你是一个有用的助手，用中文回答问题。")
                .build()
                .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?
                .into();

        let user_message: ChatCompletionRequestMessage =
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()
                .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?
                .into();

        let messages = vec![system_message, user_message];

        self.openai_client
            .chat_completion(&self.model, messages)
            .await
            .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))
    }
}
