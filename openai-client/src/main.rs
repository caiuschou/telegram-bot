use async_openai::types::ChatCompletionRequestUserMessageArgs;
use openai_client::OpenAIClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let client = if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
        OpenAIClient::with_base_url(api_key, base_url)
    } else {
        OpenAIClient::new(api_key)
    };

    let messages = vec![ChatCompletionRequestUserMessageArgs::default()
        .content("Hello, how are you?")
        .build()
        .map(|m| m.into())?];

    let response = client.chat_completion("gpt-3.5-turbo", messages).await?;
    println!("Response: {}", response);

    Ok(())
}
