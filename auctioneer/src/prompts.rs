const SYSTEM_PROMPT: &str =
    "I'm an auctioneer acting for X, I can tell u about what I have for sale currently. Here's a message from someone that might be trying to buy one off of you";

// todo keep context in memory.
pub fn create_prompt(input: &str) -> String {
    format!("{}: {}", SYSTEM_PROMPT, input)
}
