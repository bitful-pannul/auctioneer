use alloy_primitives::{Address, U256};
use alloy_signer::{LocalWallet, Signature, SignerSync};
use alloy_sol_types::SolValue;

// todo add list of deploymentss

/// Create a Sell offer, returning uid and signature buyer can use to transfer NFT out of escrow!
pub fn create_offer(
    wallet: &LocalWallet,
    nft_address: &Address,
    nft_id: u64,
    buyer: &Address,
    price: u64,
    valid_until: u64,
    // chain_id? EIP155?
) -> anyhow::Result<(u64, Signature)> {
    let uid = rand::random::<u64>();
    let encoded = (
        nft_address,
        U256::from(nft_id),
        U256::from(price),
        U256::from(uid),
        U256::from(valid_until),
        buyer,
    )
        .abi_encode_packed();

    let sig = wallet.sign_message_sync(&encoded)?;

    Ok((uid, sig))
}