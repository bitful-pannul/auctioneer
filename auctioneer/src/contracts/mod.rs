use alloy_primitives::{Address, FixedBytes, U256};
use alloy_signer::{LocalWallet, SignerSync};
use alloy_sol_types::{sol, Eip712Domain};
use std::str::FromStr;

/// Seaport cross-chain address constants
pub const SEAPORT: &str = "0x00000000000000ADc04C56Bf30aC9d3c0aAF14dC";
pub const VALIDATOR: &str = "0x00e5F120f500006757E984F1DED400fc00370000";
pub const CONDUIT: &str = "0x00000000F9490004C11Cef243f5400493c00Ad63";
pub const CONDUITKEY: &str = "0x0000007b02230091a7ed01230072f7006a004d60a8d4e71d599b8104250f0000";
pub const OPENSEAFEE: &str = "0x0000a26b00c1F0DF003000390027140000fAa719";

// create an order, with associated types.
pub fn create_listing(
    seller: &Address,
    wallet: &LocalWallet,
    nft_address: &Address,
    nft_id: u64,
    buyer: &Address,
    price: u64,
    chain_id: u64,
) -> anyhow::Result<Order> {
    let order = OrderParameters {
        offerer: *seller,
        zone: *buyer,
        offer: vec![OfferItem {
            itemType: ItemType::ERC721,
            token: *nft_address,
            identifierOrCriteria: U256::from(nft_id),
            startAmount: U256::from(price),
            endAmount: U256::from(price),
        }],
        consideration: vec![
            ConsiderationItem {
                itemType: ItemType::NATIVE,
                token: Address::default(),
                startAmount: U256::from(price),
                endAmount: U256::from(price),
                identifierOrCriteria: U256::from(nft_id),
                recipient: *seller,
            },
            // opensea wants its 2.5% cut off the price...
            ConsiderationItem {
                itemType: ItemType::NATIVE,
                token: Address::default(),
                startAmount: U256::from(price * 25 / 1000),
                endAmount: U256::from(price * 25 / 1000),
                identifierOrCriteria: U256::from(0),
                recipient: Address::from_str(OPENSEAFEE).unwrap(),
            },
        ],
        orderType: OrderType::FULL_RESTRICTED,
        startTime: U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ),
        endTime: U256::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 60 * 60 * 24, // 24h
        ),
        zoneHash: FixedBytes::from([0; 32]),
        salt: U256::from(0),
        conduitKey: FixedBytes::from_str(CONDUITKEY).unwrap(),
        totalOriginalConsiderationItems: U256::from(1),
    };

    let domain = Eip712Domain {
        name: Some("Seaport".into()),
        salt: None,
        version: Some("1.5".into()),
        chain_id: Some(U256::from(chain_id)),
        verifying_contract: Some(Address::from_str(SEAPORT).unwrap()),
    };

    let sig = wallet.sign_typed_data_sync(&order, &domain)?;
    Ok(Order {
        parameters: order,
        signature: sig.into(),
    })
}

// solidity types for interacting with the seaport contracts.
sol! {
        /**
     * @notice Validate an arbitrary number of orders, thereby registering their
     *         signatures as valid and allowing the fulfiller to skip signature
     *         verification on fulfillment. Note that validated orders may still
     *         be unfulfillable due to invalid item amounts or other factors;
     *         callers should determine whether validated orders are fulfillable
     *         by simulating the fulfillment call prior to execution. Also note
     *         that anyone can validate a signed order, but only the offerer can
     *         validate an order without supplying a signature.
     *
     * @param orders The orders to validate.
     *
     * @return validated A boolean indicating whether the supplied orders have
     *                   been successfully validated.
     */
    function validate(
        Order[] calldata orders
    ) external returns (bool validated);

    /**
     * @dev The full set of order components, with the exception of the counter,
     *      must be supplied when fulfilling more sophisticated orders or groups of
     *      orders. The total number of original consideration items must also be
     *      supplied, as the caller may specify additional consideration items.
     */
    struct OrderParameters {
        address offerer; // 0x00
        address zone; // 0x20
        OfferItem[] offer; // 0x40
        ConsiderationItem[] consideration; // 0x60
        OrderType orderType; // 0x80
        uint256 startTime; // 0xa0
        uint256 endTime; // 0xc0
        bytes32 zoneHash; // 0xe0
        uint256 salt; // 0x100
        bytes32 conduitKey; // 0x120
        uint256 totalOriginalConsiderationItems; // 0x140
        // offer.length                          // 0x160
    }

    /**
     * @dev Orders require a signature in addition to the other order parameters.
     */
    struct Order {
        OrderParameters parameters;
        bytes signature;
    }

        /**
     * @dev An offer item has five components: an item type (ETH or other native
     *      tokens, ERC20, ERC721, and ERC1155, as well as criteria-based ERC721 and
     *      ERC1155), a token address, a dual-purpose "identifierOrCriteria"
     *      component that will either represent a tokenId or a merkle root
     *      depending on the item type, and a start and end amount that support
     *      increasing or decreasing amounts over the duration of the respective
     *      order.
     */
    struct OfferItem {
        ItemType itemType;
        address token;
        uint256 identifierOrCriteria;
        uint256 startAmount;
        uint256 endAmount;
    }

        /**
     * @dev A consideration item has the same five components as an offer item and
     *      an additional sixth component designating the required recipient of the
     *      item.
     */
    struct ConsiderationItem {
        ItemType itemType;
        address token;
        uint256 identifierOrCriteria;
        uint256 startAmount;
        uint256 endAmount;
        address payable recipient;
    }


    enum OrderType {
        // 0: no partial fills, anyone can execute
        FULL_OPEN,

        // 1: partial fills supported, anyone can execute
        PARTIAL_OPEN,

        // 2: no partial fills, only offerer or zone can execute
        FULL_RESTRICTED,

        // 3: partial fills supported, only offerer or zone can execute
        PARTIAL_RESTRICTED,

        // 4: contract order type
        CONTRACT,
    }

    enum BasicOrderType {
        // 0: no partial fills, anyone can execute
        ETH_TO_ERC721_FULL_OPEN,

        // 1: partial fills supported, anyone can execute
        ETH_TO_ERC721_PARTIAL_OPEN,

        // 2: no partial fills, only offerer or zone can execute
        ETH_TO_ERC721_FULL_RESTRICTED,

        // 3: partial fills supported, only offerer or zone can execute
        ETH_TO_ERC721_PARTIAL_RESTRICTED,

        // 4: no partial fills, anyone can execute
        ETH_TO_ERC1155_FULL_OPEN,

        // 5: partial fills supported, anyone can execute
        ETH_TO_ERC1155_PARTIAL_OPEN,

        // 6: no partial fills, only offerer or zone can execute
        ETH_TO_ERC1155_FULL_RESTRICTED,

        // 7: partial fills supported, only offerer or zone can execute
        ETH_TO_ERC1155_PARTIAL_RESTRICTED,

        // 8: no partial fills, anyone can execute
        ERC20_TO_ERC721_FULL_OPEN,

        // 9: partial fills supported, anyone can execute
        ERC20_TO_ERC721_PARTIAL_OPEN,

        // 10: no partial fills, only offerer or zone can execute
        ERC20_TO_ERC721_FULL_RESTRICTED,

        // 11: partial fills supported, only offerer or zone can execute
        ERC20_TO_ERC721_PARTIAL_RESTRICTED,

        // 12: no partial fills, anyone can execute
        ERC20_TO_ERC1155_FULL_OPEN,

        // 13: partial fills supported, anyone can execute
        ERC20_TO_ERC1155_PARTIAL_OPEN,

        // 14: no partial fills, only offerer or zone can execute
        ERC20_TO_ERC1155_FULL_RESTRICTED,

        // 15: partial fills supported, only offerer or zone can execute
        ERC20_TO_ERC1155_PARTIAL_RESTRICTED,

        // 16: no partial fills, anyone can execute
        ERC721_TO_ERC20_FULL_OPEN,

        // 17: partial fills supported, anyone can execute
        ERC721_TO_ERC20_PARTIAL_OPEN,

        // 18: no partial fills, only offerer or zone can execute
        ERC721_TO_ERC20_FULL_RESTRICTED,

        // 19: partial fills supported, only offerer or zone can execute
        ERC721_TO_ERC20_PARTIAL_RESTRICTED,

        // 20: no partial fills, anyone can execute
        ERC1155_TO_ERC20_FULL_OPEN,

        // 21: partial fills supported, anyone can execute
        ERC1155_TO_ERC20_PARTIAL_OPEN,

        // 22: no partial fills, only offerer or zone can execute
        ERC1155_TO_ERC20_FULL_RESTRICTED,

        // 23: partial fills supported, only offerer or zone can execute
        ERC1155_TO_ERC20_PARTIAL_RESTRICTED,
    }

    enum BasicOrderRouteType {
        // 0: provide Ether (or other native token) to receive offered ERC721 item.
        ETH_TO_ERC721,

        // 1: provide Ether (or other native token) to receive offered ERC1155 item.
        ETH_TO_ERC1155,

        // 2: provide ERC20 item to receive offered ERC721 item.
        ERC20_TO_ERC721,

        // 3: provide ERC20 item to receive offered ERC1155 item.
        ERC20_TO_ERC1155,

        // 4: provide ERC721 item to receive offered ERC20 item.
        ERC721_TO_ERC20,

        // 5: provide ERC1155 item to receive offered ERC20 item.
        ERC1155_TO_ERC20,
    }

    enum ItemType {
        // 0: ETH on mainnet, MATIC on polygon, etc.
        NATIVE,

        // 1: ERC20 items (ERC777 and ERC20 analogues could also technically work)
        ERC20,

        // 2: ERC721 items
        ERC721,

        // 3: ERC1155 items
        ERC1155,

        // 4: ERC721 items where a number of tokenIds are supported
        ERC721_WITH_CRITERIA,

        // 5: ERC1155 items where a number of ids are supported
        ERC1155_WITH_CRITERIA,
    }

    enum Side {
        // 0: Items that can be spent
        OFFER,

        // 1: Items that must be received
        CONSIDERATION,
    }

}
