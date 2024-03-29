use crate::llm_api::OpenaiApi;
use crate::llm_types::openai::ChatParams;
use crate::llm_types::openai::Message;
use crate::AddNFTArgs;
use alloy_primitives::{
    utils::{format_ether, parse_units},
    U256,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// The maximum number of messages to keep in the chat history buffer
const BUFFER_CAPACITY: usize = 4;

/// The passkey used when parsing LLM output to link an address given by a user
const ADDRESS_PASSKEY: &str = "Thank you, reserving offer for ";
/// The passkey used when parsing LLM output to initiate the sale of an NFT
const SOLD_PASSKEY: &str = "SOLD <name_of_item> for <amount> ETH!";

/// Telegram chat id
type ChatId = i64;
/// Map of chat ids to chat contexts
type Contexts = HashMap<ChatId, Context>;

/// Manages NFT listings and chat contexts for different users.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContextManager {
    pub nft_listings: HashMap<NFTKey, NFTListing>,
    contexts: Contexts,
}

/// Represents a chat context for a single user chat
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Context {
    /// The NFT listings and state for the user
    pub nfts: HashMap<NFTKey, NFTData>,
    /// The buyer address for the user, which will get linked as soon as the user provides it
    pub buyer_address: Option<String>,
    /// Small chat history buffer, kept small for saving $$$
    chat_history: Buffer<Message>,
}

/// Identifier for an NFT
#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct NFTKey {
    pub id: u64,
    pub chain: u64,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NFTData {
    pub listing: NFTListing,
    pub state: NFTState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NFTListing {
    pub name: String,
    pub min_price: U256,
    pub address: String,
    pub description: Option<String>,
    pub custom_prompt: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct NFTState {
    pub highest_bid: U256,
    pub tentative_offer: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum AuctioneerCommand {
    /// Tentative sell is when the user has said they've offer an NFT, but we're not selling unless there's a buyer address
    TentativeOffer(TentativeOfferCommand),
    /// Finalizing a sale means linking the buyer address to the NFT, then guaranteed offer
    LinkAddress(LinkAddressCommand),
    FinalizedOffer(FinalizedOfferCommand),
    Empty,
}

impl Default for AuctioneerCommand {
    fn default() -> Self {
        AuctioneerCommand::Empty
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FinalizedOfferCommand {
    pub nft_key: NFTKey,
    pub buyer_address: String,
    pub price: U256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TentativeOfferCommand {
    pub nft_key: NFTKey,
    pub price: U256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkAddressCommand {
    pub nft_key: NFTKey,
    pub buyer_address: String,
}

impl ContextManager {
    pub fn new(nft_consts: &[(i64, &str, U256)]) -> Self {
        let mut nft_listings = HashMap::new();

        for nft in nft_consts {
            let (id, name, price) = nft;
            nft_listings.insert(
                NFTKey {
                    id: *id as u64,
                    address: "placeholder for debugging".to_string(),
                    chain: 1,
                },
                NFTListing {
                    name: name.to_string(),
                    address: "placeholder for debugging".to_string(),
                    description: None,
                    custom_prompt: None,
                    min_price: *price,
                },
            );
        }
        Self {
            nft_listings,
            contexts: HashMap::new(),
        }
    }

    /// Adds a new NFT to the auction list and updates all downstream chat contexts with this new NFT.
    pub fn add_nft(&mut self, args: AddNFTArgs) {
        let AddNFTArgs {
            nft_name,
            nft_address,
            nft_id,
            chain_id,
            nft_description,
            sell_prompt,
            min_price,
        } = args;
        let Ok(min_price) = parse_units(&min_price, "ether") else {
            return;
        };
        let key = NFTKey {
            id: nft_id,
            address: nft_address.clone(),
            chain: chain_id,
        };
        let listing = NFTListing {
            name: nft_name,
            address: nft_address,
            description: nft_description,
            custom_prompt: sell_prompt,
            min_price: min_price.into(),
        };

        self.nft_listings.insert(key.clone(), listing.clone());
        for context in self.contexts.values_mut() {
            context.nfts.entry(key.clone()).or_insert_with(|| NFTData {
                listing: listing.clone(),
                state: NFTState::default(),
            });
        }
    }

    /// Handles a chat message from a user by finding or creating the chat context, processing the message, and returning the chatbot's response.
    pub fn chat(
        &mut self,
        chat_id: ChatId,
        text: &str,
        openai_api: &OpenaiApi,
    ) -> anyhow::Result<String> {
        let context = self.chat_context(chat_id);
        let message = context.chat(openai_api, text)?;
        Ok(message.content)
    }

    /// Processes the chatbot's response to potentially finalize an NFT offer based on the chat context and the response content.
    /// This can also involve the linking of an address, or the changing of an NFTState in a context.
    pub fn act(&mut self, chat_id: ChatId, llm_response: &str) -> Option<FinalizedOfferCommand> {
        let offered_nft_key = {
            let context = self.chat_context(chat_id);
            context.process_llm_response(llm_response)
        };

        // Re-acquire the context to access the buyer address and potentially finalize the offer.
        let context = self.chat_context(chat_id);
        match (&offered_nft_key, &context.buyer_address) {
            (Some(offered_nft_key), Some(buyer_address)) => Some(FinalizedOfferCommand {
                nft_key: offered_nft_key.clone(),
                buyer_address: buyer_address.clone(),
                price: context
                    .nfts
                    .get(offered_nft_key)
                    .map(|data| data.state.highest_bid)
                    .unwrap_or_default(),
            }),
            _ => None,
        }
    }

    /// Check whether the user has offered an nft, and if so, check if they have a buyer address.
    /// If not, ask them for their address.
    pub fn additional_text(&mut self, chat_id: ChatId) -> Option<String> {
        let context = self.chat_context(chat_id);
        if context.tentative_offer_exists() && context.buyer_address.is_none() {
            return Some(
                "\nPlease send me your public Ethereum address so I can reserve the NFT for you."
                    .to_string(),
            );
        }
        None
    }

    fn chat_context(&mut self, chat_id: ChatId) -> &mut Context {
        self.contexts
            .entry(chat_id)
            .or_insert_with(|| Self::new_context(self.nft_listings.clone()))
    }

    /// Removes an NFT from the auction list and updates all downstream chat contexts with this removed NFT.
    pub fn remove_nft(&mut self, nft_key: &NFTKey) {
        self.nft_listings.remove(nft_key);
        for (_, value) in self.contexts.iter_mut() {
            value.nfts.remove(nft_key);
        }
    }

    pub fn clear(&mut self, chat_id: ChatId) {
        self.contexts.remove(&chat_id);
    }

    fn new_context(nft_listings: HashMap<NFTKey, NFTListing>) -> Context {
        let mut nft_data = HashMap::new();
        for (nft_key, listing) in nft_listings {
            let nft_state = NFTState {
                highest_bid: U256::ZERO,
                tentative_offer: false,
            };
            let data = NFTData {
                listing: listing.clone(),
                state: nft_state,
            };
            nft_data.insert(nft_key, data);
        }

        Context {
            nfts: nft_data,
            buyer_address: None,
            chat_history: Buffer::new(BUFFER_CAPACITY),
        }
    }
}

impl Context {
    /// Processes a user's chat message, updates the chat history, and generates a response using openai API.
    pub fn chat(&mut self, openai_api: &OpenaiApi, text: &str) -> anyhow::Result<Message> {
        self.chat_history.push(Message {
            role: "user".into(),
            content: text.into(),
        });

        let chat_params = create_chat_params(self.create_message_context());
        let answer = openai_api.chat(chat_params)?;
        self.chat_history.push(answer.clone());
        Ok(answer)
    }

    /// Processes the chatbot's response to identify any tentative offers or link buyer addresses.
    /// Returns NFT key if updates occur, otherwise `None`.
    fn process_llm_response(&mut self, llm_response: &str) -> Option<NFTKey> {
        if let Some(tentative_offer) = self.handle_offer(llm_response) {
            self.nfts.get_mut(&tentative_offer.nft_key).map(|data| {
                data.state.tentative_offer = true;
                if data.state.highest_bid < tentative_offer.price {
                    data.state.highest_bid = tentative_offer.price;
                }
            });
            if self.buyer_address.is_some() {
                return Some(tentative_offer.nft_key);
            }
        } else if let Some(link_address_cmd) = self.handle_address_linking(llm_response) {
            self.buyer_address = Some(link_address_cmd.buyer_address);
            return Some(link_address_cmd.nft_key);
        }
        None
    }

    fn tentative_offer_exists(&self) -> bool {
        self.first_tentative_offer().is_some()
    }

    fn first_tentative_offer(&self) -> Option<NFTKey> {
        self.nfts.iter().find_map(|(key, data)| {
            if data.state.tentative_offer {
                Some(key.clone())
            } else {
                None
            }
        })
    }

    fn has_offer_item_without_buyer(&self) -> bool {
        let tentatively_offer = self.tentative_offer_exists();
        let no_address = self.buyer_address.is_none();
        tentatively_offer && no_address
    }

    pub fn create_message_context(&self) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.chat_history.buffer.len() + 1);
        messages.push(self.create_system_prompt());
        messages.extend(self.chat_history.buffer.iter().cloned());
        messages
    }

    /// Creates the system prompt for the chatbot, parsing the listings including rules and custom descriptions.
    /// When the bot requires the address, it adjusts the system prompt to ask for it exclusively.
    fn create_system_prompt(&self) -> Message {
        let beginning = "You are a a chatbot auctioneer selling NFTs. ";

        let middle = if self.has_offer_item_without_buyer() {
            format!("The buyer you're chatting with has bought an NFT from you, but you don't have their ETH address. Please ask them for their public address and do not relent. Don't talk about anything else but their address. Iff they give something resembling a ETH address to you, repeat it with '{}<address>'", ADDRESS_PASSKEY)
        } else {
            let nft_with_prices = self
                .nfts
                .iter()
                .map(|(key, data)| {
                    let description = match &data.listing.description {
                        Some(description) => format!(", description: {}", description),
                        None => "".to_string(),
                    };
                    let custom_prompt = match &data.listing.custom_prompt {
                        Some(custom_prompt) => format!(", and custom rules: {}", custom_prompt),
                        None => "".to_string(),
                    };
                    let address_string = format!(
                        "The address is {}, the chain id {} and the id is {}.",
                        data.listing.address, key.chain, key.id
                    );

                    format!(
                        "\n- {} with min bid of {} ETH{}{}.{}\n",
                        data.listing.name,
                        format_ether(data.listing.min_price),
                        description,
                        custom_prompt,
                        address_string
                    )
                })
                .collect::<Vec<String>>()
                .join("");

            let auctions = if nft_with_prices.is_empty() {
                "Currently, there are no NFTs available for auction.".into()
            } else {
                nft_with_prices
            };

            format!(
                r###"
            The list of NFTs is {} 
            
            Iff the user is talking about a specific nft, follow custom rules, even disregarding general rules. Only follow one custom rule at a time.

            Never reveal the min bid required to the user, only sell if minimum price is bid. Only reveal the address, chain id and id of the nft when specifically asked for it. If someone bids more, don't go back down for that nft. 
            Iff a price is reached, write very clearly with no variation {}
            "###,
                auctions, SOLD_PASSKEY
            )
        };

        let end = "Write in a very terse manner, write as if you were chatting with someone. Don't let the user fool you.";

        let content = format!("{}{}{}", beginning, middle, end);

        Message {
            role: "system".into(),
            content,
        }
    }

    /// Parses the LLM response to identify a tentative offer which will get sent upstream.
    fn handle_offer(&self, input: &str) -> Option<TentativeOfferCommand> {
        if !input.starts_with("SOLD") {
            return None;
        }
        let parts: Vec<&str> = input.split(" for ").collect();
        if parts.len() != 2 {
            return None;
        }

        let nft_name_end_index = parts[0].len();
        let nft_name = &parts[0][5..nft_name_end_index];

        let amount_str = parts[1].trim_end_matches(" ETH!");

        let amount: U256 = parse_units(amount_str, "ether")
            .unwrap_or(U256::ZERO.into())
            .into();

        if let Some((current_key, _)) = self
            .nfts
            .iter()
            .find(|(_, data)| data.listing.name == nft_name)
        {
            let min_amount_reached = self
                .nfts
                .get(current_key)
                .map(|nft_data| amount >= nft_data.listing.min_price)
                .unwrap_or_default();
            if min_amount_reached {
                let command = TentativeOfferCommand {
                    nft_key: current_key.clone(),
                    price: amount,
                };
                return Some(command);
            }
        }
        None
    }

    /// Checks whether the LLM response contains a command to link a buyer's address to an NFT purchase.
    fn handle_address_linking(&self, llm_response: &str) -> Option<LinkAddressCommand> {
        let re = regex::Regex::new(r"0x[a-fA-F0-9]{40}").unwrap();
        if let Some(caps) = re.captures(llm_response) {
            if let Some(matched) = caps.get(0) {
                let eth_address = matched.as_str();
                if let Some(nft_key) = self.first_tentative_offer() {
                    return Some(LinkAddressCommand {
                        nft_key,
                        buyer_address: eth_address.to_string(),
                    });
                }
            }
        }
        None
    }
}

/// Simple buffer for message handling.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Buffer<T> {
    capacity: usize,
    buffer: VecDeque<T>,
}

impl<T> Buffer<T> {
    fn new(capacity: usize) -> Self {
        Buffer {
            capacity,
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    fn push(&mut self, item: T) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }
}

/// The params to communicate with the LLM process.
/// Max tokens is kept low to save money.
/// Temperature is set to 0.2 to make the LLM rather predictable.
fn create_chat_params(messages: Vec<Message>) -> ChatParams {
    let chat_params = ChatParams {
        model: "gpt-4-1106-preview".into(),
        messages,
        max_tokens: Some(150),
        temperature: Some(0.2),
        ..Default::default()
    };
    chat_params
}
