//! Binary: load env, generate messages, write JSON to stdout.

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let messages = seed_messages::generate_messages()?;
    let json = serde_json::to_string_pretty(&messages)?;
    println!("{}", json);
    Ok(())
}
