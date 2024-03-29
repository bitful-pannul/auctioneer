import React, { useEffect, useState } from "react";
import { useChainId, useWriteContract, useAccount } from "wagmi";
import { erc721Abi } from 'viem';
import { Header } from "./components/layout/Header";
import { NetworkSwitcher } from "./components/SwitchNetworks";
import { WalletModal } from "./components/WalletModal";
import Button from "antd/es/button";
import { shorten } from "@did-network/dapp-sdk";
import classNames from "classnames";

const ESCROW_ADDRESS = "0x4A3A2c0A385F017501544DcD9C6Eb3f6C63fc38b";

interface ConfigData {
  openai_key: string,
  telegram_bot_api_key: string,
  wallet_pk: string,
  hosted_url: string,
}

interface NFT {
  id: number;
  chain: number;
  name: string;
  address: string;
  min_price: string;
  description?: string;
  custom_prompt?: string;
}

const InitialConfig: React.FC<{ onSubmit: (configData: ConfigData) => Promise<void> }> = ({ onSubmit }) => {
  const [openaiKey, setOpenaiKey] = useState("");
  const [telegramKey, setTelegramKey] = useState("");
  const [walletPk, setWalletPk] = useState("");
  const [hostedUrl, setHostedUrl] = useState("https://appattacc.xyz"); // add default hosted website.

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSubmit({
      openai_key: openaiKey,
      telegram_bot_api_key: telegramKey,
      wallet_pk: walletPk,
      hosted_url: hostedUrl,
    });
  };

  return (
    <main className="max-w-lg mx-auto mt-8 flex flex-col">
      <h1 className="text-2xl font-bold mb-4">Initial Configuration</h1>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="flex flex-col">
          <label htmlFor="openai-key" className="flex items-center self-stretch text-sm font-bold mb-2">
            OpenAI API Key
            <ExpandableSection className="ml-2">
              <ol className="list-decimal list-inside">
                <li>Visit OpenAI's signup page.</li>
                <li>Create an account and navigate to API section.</li>
                <li>Click "New API Key" to generate a key.</li>
                <li>Securely copy the API key displayed.</li>
                <li>Type 'y' to proceed with entering the API key.</li>
                <li>Paste the API key into the input.</li>
              </ol>
            </ExpandableSection>
          </label>
          <input
            type="text"
            id="openai-key"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="OpenAI API Key"
            value={openaiKey}
            onChange={e => setOpenaiKey(e.target.value)}
          />
        </div>

        <div className="flex flex-col">
          <label className="flex items-center text-sm font-bold mb-2">
            Telegram Bot API Key
            <ExpandableSection className="ml-2">
              <ol className="list-decimal list-inside">
                <li>Open Telegram and search for "@BotFather".</li>
                <li>Start a conversation and type `/newbot`.</li>
                <li>Follow prompts to create a new bot.</li>
                <li>Securely copy the API key displayed.</li>
                <li>Paste the bot API key into the input.</li>
              </ol>
            </ExpandableSection>
          </label>
          <input
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Telegram Bot API Key"
            value={telegramKey}
            onChange={e => setTelegramKey(e.target.value)}
          />
        </div>
        <div className="flex flex-col">
          <label className="flex items-center self-stretch text-sm font-bold mb-2">
            Private Wallet Key
            <ExpandableSection className="ml-2">
              <ol className="list-decimal list-inside">
                <li>Choose a wallet provider and create a new wallet.</li>
                <li>Securely go through the wallet creation process.</li>
                <li>Access your wallet to find your Ethereum address.</li>
                <ul className="list-disc list-inside ml-4">
                  <li>
                    <strong>Never share your private key.</strong>
                  </li>
                </ul>
                <li>Use your Ethereum address as directed.</li>
              </ol>
            </ExpandableSection>
          </label>
          <input
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Private Wallet Key"
            value={walletPk}
            onChange={e => setWalletPk(e.target.value)}
          />
        </div>
        <div className="flex flex-col">
          <label className="flex items-center self-stretch text-sm font-bold mb-2">
            Hosted URL (Optional)
            <ExpandableSection className="ml-2">
              <p>Where users will be redirected to buy the items. Defaults to a UI we host, but you can configure your own, even host it from your kinode too!</p>
            </ExpandableSection>
          </label>
          <input
            type="text"
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Hosted URL"
            value={hostedUrl}
            onChange={e => setHostedUrl(e.target.value)}
          />
        </div>

        <button
          type="submit"
          className="normal"
        >
          Submit
        </button>
      </form>
    </main>
  );
};

const ExpandableSection = ({ children, className }: { children: React.ReactNode, className?: string }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div className={classNames('relative', className)}>
      <button
        type="button"
        className="icon px-0 py-0 text-sm font-bold focus:outline-none flex items-center"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        {isExpanded ? "▼" : "?"}
      </button>
      {isExpanded && <div className="mt-2 absolute min-w-200px bg-black rounded p-2 z-10 text-sm whitespace-normal">{children}</div>}
    </div>
  );
};

const NFTManager: React.FC = () => {
  const [nftListings, setNftListings] = useState<NFT[]>([]);
  const [selectedNFTKey, setSelectedNFTKey] = useState<string | null>(null);
  const [nftName, setNftName] = useState("");
  const [nftAddress, setNftAddress] = useState("");
  const [nftId, setNftId] = useState("");
  const [nftDescription, setNftDescription] = useState("");
  const [sellPrompt, setSellPrompt] = useState("");
  const [minPrice, setMinPrice] = useState("");

  const { writeContractAsync } = useWriteContract()

  const chainId = useChainId();


  useEffect(() => {
    console.log('trying to fetch');
    listNFTs();
    console.log('did fetch');

  }, []);

  const listNFTs = async () => {
    const response = await fetch("/main:barter:appattacc.os/listnfts", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    console.log('nft response: ', response);
    const data: NFT[] = await response.json();
    console.log('post json data: ', data)

    setNftListings(data);
    console.log('successfully set? ', nftListings);
  };

  const handleSubmitNFT = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!nftName || !nftAddress || !nftId || !minPrice) {
      alert("Please fill out all fields before submitting.");
      return;
    }
    console.log("we on chain X prob should alert seller...");

    await writeContractAsync({
      abi: erc721Abi,
      address: nftAddress as any,
      functionName: "approve",
      args: [ESCROW_ADDRESS, BigInt(nftId)],
    });
    console.log("did approve!");

    await fetch("/main:barter:appattacc.os/addnft", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        nft_name: nftName,
        nft_address: nftAddress,
        nft_id: parseInt(nftId, 10),
        chain_id: chainId,
        nft_description: nftDescription,
        sell_prompt: sellPrompt,
        min_price: minPrice,
      }),
    });
    await listNFTs();
    setNftName("");
    setNftAddress("");
    setNftId("");
    setNftDescription("");
    setSellPrompt("");
    setMinPrice("");
  };

  const handleRemoveNFT = async (id: number, address: string, chain: number) => {
    await fetch("/main:barter:appattacc.os/removenft", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        id,
        address,
        chain,
      }),
    });
    await listNFTs();
  };

  return (
    <main className="max-w-lg mx-auto mt-8">
      <h1 className="text-2xl font-bold mb-4">NFT Manager</h1>
      <form onSubmit={handleSubmitNFT} className="space-y-4">
        <div className="flex flex-col">
          <label htmlFor="nft-name" className="flex items-center text-sm font-bold mb-2">
            NFT Name
            <ExpandableSection className="ml-2">
              The name of the NFT. This should be a unique and descriptive title for the NFT you're managing.
            </ExpandableSection>
          </label>
          <input
            id="nft-name"
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Name"
            value={nftName}
            onChange={e => setNftName(e.target.value)}
          />
        </div>

        <div className="flex flex-col">
          <label htmlFor="nft-address" className="flex items-center text-sm font-bold mb-2">
            Address
            <ExpandableSection className="ml-2">
              Smart contract address of the NFT. It uniquely identifies the contract that manages the NFTs you're
              declaring.
            </ExpandableSection>
          </label>
          <input
            id="nft-address"
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Contract Address"
            value={nftAddress}
            onChange={e => setNftAddress(e.target.value)}
          />
        </div>

        <div className="flex flex-col">
          <label htmlFor="chain-id" className="flex items-center text-sm font-bold mb-2">
            Chain ID
            <ExpandableSection className="ml-2">
              What network nft you're selling. Change it with the button on the top right.
            </ExpandableSection>
          </label>
          <input
            id="chain-id"
            type="text"
            disabled
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline bg-gray-200"
            value={chainId}
          />
        </div>

        <div className="flex flex-col">
          <label htmlFor="nft-id" className="flex items-center text-sm font-bold mb-2">
            NFT ID{" "}
            <ExpandableSection className="ml-2">The unique identifier for the specific NFT within its collection.</ExpandableSection>
          </label>
          <input
            id="nft-id"
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT ID"
            value={nftId}
            onChange={e => setNftId(e.target.value)}
          />
        </div>

        <div className="flex flex-col">
          <label htmlFor="min-price" className="flex items-center text-sm font-bold mb-2">
            Min Price
            <ExpandableSection className="ml-2">
              The minimum price for the NFT. No contract lower than that price will be generated, and the bot will try
              to get more than the price out of the auction.
            </ExpandableSection>
          </label>
          <input
            id="min-price"
            type="text"
            required
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Minimum Price"
            value={minPrice}
            onChange={e => setMinPrice(e.target.value)}
          />
        </div>

        <div className="flex flex-col">
          <label htmlFor="nft-description" className="flex items-center text-sm font-bold mb-2">
            Description of NFT (optional)
            <ExpandableSection className="ml-2">
              <p>
                Additional description you'll give to the bot for the sale of that NFT. Can be a backstory, or anything
                you want.
              </p>
            </ExpandableSection>
          </label>
          <textarea
            id="nft-description"
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Description"
            value={nftDescription}
            onChange={e => setNftDescription(e.target.value)}
          ></textarea>
        </div>

        <div className="flex flex-col">
          <label htmlFor="sell-prompt" className="flex items-center text-sm font-bold mb-2">
            Custom prompt on how to sell it (optional)
            <ExpandableSection className="ml-2">
              <p>Give instructions on what the bot should do to sell it. For example, being greedy vs lenient.</p>
            </ExpandableSection>
          </label>
          <textarea
            id="sell-prompt"
            className="appearance-none border rounded py-2 px-3 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Custom Selling Instructions"
            value={sellPrompt}
            onChange={e => setSellPrompt(e.target.value)}
          ></textarea>
        </div>

        <button
          type="submit"
          className="normal"
        >
          Submit NFT
        </button>
      </form>

      <div className="mt-8">
        {nftListings && Array.isArray(nftListings) && nftListings.map((nft, index) => (
          <div
            key={index}
            className={`border p-4 mb-4 rounded cursor-pointer ${selectedNFTKey === `${nft.id}:${nft.chain}` ? "bg-gray-100" : ""
              }`}
            onClick={() => setSelectedNFTKey(`${nft.id}:${nft.chain}`)}
          >
            <div className="flex justify-between items-center">
              <div>
                <p className="font-bold">Name: {nft.name}</p>
                <p>Address: {nft.address}</p>
                <p>Min Price: {nft.min_price}</p>
                <p>Description: {nft.description || "N/A"}</p>
                <p>Custom Prompt: {nft.custom_prompt || "N/A"}</p>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleRemoveNFT(nft.id, nft.address, nft.chain);
                }}
              >
                X
              </button>
            </div>
          </div>
        ))}
      </div>
    </main>
  );
};

const App: React.FC = () => {
  const [isConfigured, setIsConfigured] = useState(false);
  const { address } = useAccount();

  const [show, setShow] = useState(false)

  const toggleModal = (e: boolean) => {
    setShow(e)
  }
  useEffect(() => {
    fetchStatus();
  }, []);

  const fetchStatus = async () => {
    const response = await fetch("/main:barter:appattacc.os/status", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: "config" }),
    });
    const data = await response.json();
    setIsConfigured(data.status === "manage-nfts");
  };

  const handleConfigSubmit = async (configData: ConfigData) => {
    await fetch("/main:barter:appattacc.os/config", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(configData),
    });
    setIsConfigured(true);
  };

  return <>
    <Header
      action={
        <>
          <NetworkSwitcher />
          <WalletModal open={show} onOpenChange={toggleModal} close={() => setShow(false)}>
            {({ isLoading }) => (
              <Button className="mr-4 bg-white text-black border-2" style={{ fontFamily: 'OpenSans' }}>
                {isLoading && (
                  <span className="i-line-md:loading-twotone-loop inline-flex mr-1 w-4 h-4"></span>
                )}{' '}
                {address ? shorten(address) : 'Connect Wallet'}
              </Button>
            )}
          </WalletModal>
        </>
      }
    />
    <div>{!isConfigured ? <InitialConfig onSubmit={handleConfigSubmit} /> : <NFTManager />}</div>
  </>;
};

export default App;
