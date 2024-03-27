use alloy_primitives::{keccak256, Address, U256};
use alloy_signer::{LocalWallet, Signature, SignerSync};
use alloy_sol_types::{sol, SolValue};
// use kinode_process_lib::println;

// SEPOLIA + OP mainnet + BASE: 0x4A3A2c0A385F017501544DcD9C6Eb3f6C63fc38b
sol! {
    event NFTPurchased(
        address indexed seller,
        address nftAddress,
        uint256 tokenId,
        address buyer,
        uint256 price
    );
}

/// Create a Sell offer, returning uid and signature buyer can use to transfer NFT out of escrow!
pub fn _create_offer(
    wallet: &LocalWallet,
    nft_address: &Address,
    nft_id: u64,
    buyer: &Address,
    price: U256,
    valid_until: u64,
) -> anyhow::Result<(u64, Signature)> {
    let uid = rand::random::<u64>();
    let encoded_packed = keccak256(
        (
            nft_address,
            U256::from(nft_id),
            price,
            U256::from(uid),
            U256::from(valid_until),
            buyer,
        )
            .abi_encode_packed(),
    );

    // left for debugging and learning...!
    // println!("encoded: {:?}", hex::encode(&encoded));
    // let keccakencoded = keccak256(&encoded);
    // println!("keccakencoded: {:?}", hex::encode(&keccakencoded));

    // let eth_hash = eip191_hash_message(&keccakencoded);
    // println!("eth_hash: {:?}", hex::encode(&eth_hash));
    // println!("encoded_packed: {:?}", hex::encode(&encoded_packed));
    let sig = wallet.sign_message_sync(&encoded_packed.to_vec())?;

    // println!("sig: {:?}", hex::encode(&sig.as_bytes()));
    Ok((uid, sig))
}
