use crate::llm_types::openai::Message;
use crate::llm_types::openai::ChatParams;

const SYSTEM_PROMPT: &str =
    "I'm an auctioneer acting for X, I can tell u about what I have for sale currently. Here's a message from someone that might be trying to buy one off of you";

// todo keep context in memory.
fn create_prompt(input: &str) -> Message {
    let prompt = format!("{}: {}", SYSTEM_PROMPT, input);
    let msg = Message {
        content: prompt,
        role: "user".into(), // TODO: Zen: This shouldn't be user
    };
    msg
}

pub fn create_chat_params(text: &str) -> ChatParams {
    let msg = create_prompt(&text);
    ChatParams {
        model: "gpt-3.5-turbo".into(),
        messages: vec![msg],
        max_tokens: Some(200),
        temperature: Some(1.25),
        ..Default::default()
    }
}