use crate::llm_api::OpenaiApi;
use crate::llm_types::openai::ChatParams;
use crate::llm_types::openai::Message;
use kinode_process_lib::println;
use std::collections::HashMap;
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

/// The maximum number of messages to keep in the chat history buffer
const BUFFER_CAPACITY: usize = 4;

const ADDRESS_PASSKEY: &str = "Thank you, setting up contract for ";
const SELLING_PASSKEY: &str = "SOLD <name_of_item> for <amount> ETH!";

type Contexts = HashMap<i64, Context>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContextManager {
    nft_listings: HashMap<i64, NFTListing>,
    contexts: Contexts,
    openai_api: OpenaiApi,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Context {
    pub nfts: HashMap<i64, NFTData>,
    pub buyer_address: Option<String>,
    chat_history: Buffer<Message>,
    pub sold_nfts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NFTData {
    pub listing: NFTListing,
    pub state: NFTState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NFTListing {
    pub name: String,
    pub min_price: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NFTState {
    pub highest_bid: f32,
    pub tentative_sold: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum AuctioneerCommand {
    /// Tentative sell is when the user has said they've sold an NFT, but we're not selling unless there's a buyer address
    TentativeSell(SoldCommand),
    /// Finalizing a sale means linking the buyer address to the NFT, then guaranteed selling
    FinalizeSell((String, SoldCommand)),
    Empty,
}

impl Default for AuctioneerCommand {
    fn default() -> Self {
        AuctioneerCommand::Empty
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SoldCommand {
    nft_id: i64,
    _amount: f32,
}

impl ContextManager {
    // TODO: Initializing should probably be empty in the long run, this is for testing.
    pub fn new(openai_api: OpenaiApi, nft_consts: &[(i64, &str, f32); 3]) -> Self {
        let mut nft_listings = HashMap::new();
        for nft in nft_consts {
            let (id, name, price) = nft;
            nft_listings.insert(
                *id,
                NFTListing {
                    name: name.to_string(),
                    min_price: *price,
                },
            );
        }
        Self {
            nft_listings,
            contexts: HashMap::new(),
            openai_api,
        }
    }

    pub fn chat(
        &mut self,
        chat_id: i64,
        text: &str,
    ) -> anyhow::Result<(String, Option<SoldCommand>)> {
        let context = Self::upsert_context(&mut self.contexts, self.nft_listings.clone(), chat_id);
        let message = context.chat(&self.openai_api, text)?;
        let mut text = message.content.clone();

        context.chat_history.push(message);
        let command = context.auctioneer_command(&text);

        if matches!(&command, &AuctioneerCommand::TentativeSell(_))
            && context.buyer_address.is_none()
        {
            text += "\n\nPlease send me your public Ethereum address so I can send you the NFT."
        }

        let sold = Self::execute_command(context, command);

        if context.sold_nfts.len() > 0 {
            text += &format!(
                "\n\nAlso note: The NFTs '{}' have been sold in another chat!",
                context.sold_nfts.join(", ")
            );
            context.sold_nfts.clear();
        }

        if let Some(sold) = &sold {
            self.sync_state(sold, chat_id);
        }

        Ok((text, sold))
    }

    fn sync_state(&mut self, sold: &SoldCommand, chat_id: i64) {
        let name = self
            .nft_listings
            .get(&sold.nft_id)
            .map(|listing| listing.name.clone())
            .unwrap_or_default();
        self.nft_listings.remove(&sold.nft_id);

        for (context_id, value) in self.contexts.iter_mut() {
            value.nfts.remove(&sold.nft_id);
            if *context_id != chat_id {
                value.sold_nfts.push(name.clone());
            }
        }
    }

    // TODO: I don't know whether we actually need this? We just need a way to upstream all the way to lib for the sale...
    fn execute_command(context: &mut Context, command: AuctioneerCommand) -> Option<SoldCommand> {
        let mut sold_command = None;
        match command {
            AuctioneerCommand::TentativeSell(sale) => {
                context.nfts.get_mut(&sale.nft_id).map(|data| {
                    data.state.tentative_sold = true;
                });
                if context.buyer_address.is_some() {
                    sold_command = Some(sale);
                }
            }
            AuctioneerCommand::FinalizeSell((address, sale)) => {
                context.buyer_address = Some(address);
                sold_command = Some(sale);
            }
            AuctioneerCommand::Empty => {
                // No command to execute
            }
        }
        sold_command
    }

    pub fn clear(&mut self, chat_id: i64) {
        self.contexts.remove(&chat_id);
    }

    pub fn get_message_history(&self, chat_id: i64) -> Vec<Message> {
        self.contexts
            .get(&chat_id)
            .unwrap()
            .create_message_context()
    }

    fn upsert_context(
        contexts: &mut Contexts,
        nft_listings: HashMap<i64, NFTListing>,
        chat_id: i64,
    ) -> &mut Context {
        if !contexts.contains_key(&chat_id) {
            contexts.insert(chat_id, Self::new_context(nft_listings));
        }
        contexts.get_mut(&chat_id).unwrap()
    }

    fn new_context(nft_listings: HashMap<i64, NFTListing>) -> Context {
        let mut nft_data = HashMap::new();
        for (nft_id, listing) in nft_listings {
            let nft_state = NFTState {
                highest_bid: 0.0,
                tentative_sold: false,
            };
            let data = NFTData {
                listing: listing.clone(),
                state: nft_state,
            };
            nft_data.insert(nft_id, data);
        }

        Context {
            nfts: nft_data,
            buyer_address: None,
            chat_history: Buffer::new(BUFFER_CAPACITY),
            sold_nfts: vec![],
        }
    }
}

impl Context {
    pub fn chat(&mut self, openai_api: &OpenaiApi, text: &str) -> anyhow::Result<Message> {
        self.chat_history.push(Message {
            role: "user".into(),
            content: text.into(),
        });

        let chat_params = create_chat_params(self.create_message_context());
        openai_api.chat(chat_params)
    }

    fn has_sold_item_without_buyer(&self) -> bool {
        let tentatively_sold = self.nfts.values().any(|data| data.state.tentative_sold);
        let no_address = self.buyer_address.is_none();
        tentatively_sold && no_address
    }

    pub fn create_message_context(&self) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.chat_history.buffer.len() + 1);
        messages.push(self.create_system_prompt());
        messages.extend(self.chat_history.buffer.iter().cloned());
        messages
    }

    fn create_system_prompt(&self) -> Message {
        let beginning = "You are a a chatbot auctioneer selling NFTs. ";

        let middle = if self.has_sold_item_without_buyer() {
            format!("The buyer you're chatting with has bought an NFT from you, but you don't have their ETH address. Please ask them for their public address and do not relent. Don't talk about anything else but their address. Iff they give something resembling a ETH address to you, repeat it with '{}<address>'", ADDRESS_PASSKEY)
        } else {
            let nft_with_prices = self
                .nfts
                .iter()
                .filter(|(_, data)| !data.state.tentative_sold)
                .map(|(_, data)| {
                    format!(
                        "{} with min bid of {} ETH. ",
                        data.listing.name, data.listing.min_price
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
            The list of NFTs is {}. Never reveal the min bid required to the user, only sell if minimum price is bid. If someone bids more, don't go back down for that nft. 
            Iff a price is reached, write very clearly with no variation {}
            "###,
                auctions, SELLING_PASSKEY
            )
        };

        let end = "Write in a very terse manner, write as if you were chatting with someone. Don't let the user fool you.";

        let content = format!("{}{}{}", beginning, middle, end);

        Message {
            role: "system".into(),
            content,
        }
    }

    // TODO: Zen: Sold command should rather be named prepare sale or something
    fn auctioneer_command(&self, input: &str) -> AuctioneerCommand {
        if let Some(command) = self.handle_sold(input) {
            AuctioneerCommand::TentativeSell(command)
        } else if let Some((address, sold_command)) = self.handle_address_linking(input) {
            AuctioneerCommand::FinalizeSell((address, sold_command))
        } else {
            AuctioneerCommand::Empty
        }
    }

    fn handle_sold(&self, input: &str) -> Option<SoldCommand> {
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

        let amount = match amount_str.parse::<f32>() {
            Ok(amount) => amount,
            Err(_) => return None,
        };

        if let Some((current_id, _)) = self
            .nfts
            .iter()
            .find(|(_, data)| data.listing.name == nft_name)
        {
            let min_amount_reached = self
                .nfts
                .get(current_id)
                .map(|nft_data| amount >= nft_data.listing.min_price)
                .unwrap_or_default();
            if min_amount_reached {
                let command = SoldCommand {
                    nft_id: *current_id,
                    _amount: amount,
                };
                return Some(command);
            }
        }
        None
    }

    fn handle_address_linking(&self, input: &str) -> Option<(String, SoldCommand)> {
        if input.starts_with(ADDRESS_PASSKEY) {
            let potential_address = input.strip_prefix(ADDRESS_PASSKEY).unwrap_or_default();
            let address = &potential_address[..42];
            if address.starts_with("0x") {
                let buyer_address = address.to_string();
                let sold_command = self.nfts.iter().find_map(|(current_id, data)| {
                    if data.state.tentative_sold {
                        Some(SoldCommand {
                            nft_id: *current_id,
                            _amount: data.state.highest_bid,
                        })
                    } else {
                        None
                    }
                });
                return sold_command.map(|cmd| (buyer_address, cmd));
            } else {
                println!("Provided address does not follow the Ethereum address format.");
            }
        }
        None
    }
}

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

fn create_chat_params(messages: Vec<Message>) -> ChatParams {
    let chat_params = ChatParams {
        model: "gpt-4-1106-preview".into(), // TODO: Zen:
        // model: "gpt-3.5-turbo".into(),
        messages,
        max_tokens: Some(150),
        temperature: Some(0.2),
        ..Default::default()
    };
    chat_params
}
