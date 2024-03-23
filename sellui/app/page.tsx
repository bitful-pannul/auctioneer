"use client";

import React, { useEffect, useState } from "react";
import { erc721ABI, useChainId, useContractWrite } from "wagmi";

const ESCROW_ADDRESS = "0x7b1431A0f20A92dD7E42A28f7Ba9FfF192F36DF3";

interface ConfigData {
  openaiKey: string;
  telegramKey: string;
  walletAddress: string;
}

interface NFT {
  id: number;
  chain_id: number;
  name: string;
  address: string;
  min_price: number;
  description: string;
  custom_prompt: string;
}

const InitialConfig: React.FC<{ onSubmit: (configData: ConfigData) => Promise<void> }> = ({ onSubmit }) => {
  const [openaiKey, setOpenaiKey] = useState("");
  const [telegramKey, setTelegramKey] = useState("");
  const [walletAddress, setWalletAddress] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSubmit({ openaiKey, telegramKey, walletAddress });
  };

  return (
    <main className="max-w-lg mx-auto mt-8">
      <h1 className="text-2xl font-bold mb-4">Initial Configuration</h1>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="openai-key" className="block text-gray-700 text-sm font-bold mb-2">
            OpenAI API Key
          </label>
          <ExpandableSection>
            <ol className="list-decimal list-inside">
              <li>Visit OpenAI's signup page.</li>
              <li>Create an account and navigate to API section.</li>
              <li>Click "New API Key" to generate a key.</li>
              <li>Securely copy the API key displayed.</li>
              <li>Type 'y' to proceed with entering the API key.</li>
              <li>Paste the API key into the input.</li>
            </ol>
          </ExpandableSection>
          <input
            id="openai-key"
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="OpenAI API Key"
            value={openaiKey}
            onChange={e => setOpenaiKey(e.target.value)}
          />
        </div>

        <div className="relative">
          <label className="block text-gray-700 text-sm font-bold mb-2">
            Telegram Bot API Key
            <ExpandableSection>
              <ol className="list-decimal list-inside">
                <li>Open Telegram and search for "@BotFather".</li>
                <li>Start a conversation and type `/newbot`.</li>
                <li>Follow prompts to create a new bot.</li>
                <li>Securely copy the API key displayed.</li>
                <li>Type 'y' to proceed with entering the bot API key.</li>
                <li>Paste the bot API key into the input.</li>
              </ol>
            </ExpandableSection>
          </label>
          <input
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Telegram Bot API Key"
            value={telegramKey}
            onChange={e => setTelegramKey(e.target.value)}
          />
        </div>

        <div className="relative">
          <label className="block text-gray-700 text-sm font-bold mb-2">
            Private Wallet Address
            <ExpandableSection>
              <ol className="list-decimal list-inside">
                <li>Choose a wallet provider and create a new wallet.</li>
                <li>Securely go through the wallet creation process.</li>
                <li>Access your wallet to find your Ethereum address.</li>
                <ul className="list-disc list-inside ml-4">
                  <li>
                    <strong>Never share your private key.</strong>
                  </li>
                </ul>
                <li>Type 'y' to proceed with entering your Ethereum address.</li>
                <li>Use your Ethereum address as directed.</li>
              </ol>
            </ExpandableSection>
          </label>
          <input
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Private Wallet Address"
            value={walletAddress}
            onChange={e => setWalletAddress(e.target.value)}
          />
        </div>

        <button
          type="submit"
          className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
        >
          Submit
        </button>
      </form>
    </main>
  );
};

const ExpandableSection = ({ children }: { children: React.ReactNode }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div className="mb-4">
      <button
        type="button"
        className="text-gray-700 text-sm font-bold focus:outline-none flex items-center"
        onClick={() => setIsExpanded(!isExpanded)}
      >
        {isExpanded ? "▼" : "?"}
      </button>
      {isExpanded && <div className="mt-2 text-gray-700 text-sm whitespace-normal">{children}</div>}
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

  const { write, data, isLoading } = useContractWrite({
    address: nftAddress,
    abi: erc721ABI,
    functionName: "approve",
    args: [ESCROW_ADDRESS, BigInt(nftId)],
  });

  const chainId = useChainId();


  useEffect(() => {
    listNFTs();
  }, []);

  const listNFTs = async () => {
    const response = await fetch("/auctioneer:auctioneer:template.os/listnfts", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    const data = await response.json();
    setNftListings(data.nft_listings_values);
  };

  const handleSubmitNFT = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!nftName || !nftAddress || !nftId || !minPrice) {
      alert("Please fill out all fields before submitting.");
      return;
    }
    console.log("we on chain X prob should alert seller...");

    write();

    console.log("did approve!");

    await fetch("/auctioneer:auctioneer:template.os/addnft", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        nft_name: nftName,
        nft_address: nftAddress,
        nft_id: parseInt(nftId, 10),
        chain_id: chainId,
        nft_description: nftDescription,
        sell_prompt: sellPrompt,
        min_price: parseFloat(minPrice),
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

  const handleRemoveNFT = async () => {
    if (!selectedNFTKey) {
      alert("Please select an NFT to remove.");
      return;
    }
    const [nftId, chainId] = selectedNFTKey.split(":");
    await fetch("/auctioneer:auctioneer:template.os/removenft", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        nft_id: parseInt(nftId, 10),
        chain_id: parseInt(chainId, 10),
      }),
    });
    await listNFTs();
    setSelectedNFTKey(null);
  };

  return (
    <main className="max-w-lg mx-auto mt-8">
      <h1 className="text-2xl font-bold mb-4">NFT Manager</h1>
      <form onSubmit={handleSubmitNFT} className="space-y-4">
        <div>
          <label htmlFor="nft-name" className="block text-gray-700 text-sm font-bold mb-2">
            NFT Name
            <ExpandableSection>
              The name of the NFT. This should be a unique and descriptive title for the NFT you're managing.
            </ExpandableSection>
          </label>
          <input
            id="nft-name"
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Name"
            value={nftName}
            onChange={e => setNftName(e.target.value)}
          />
        </div>

        <div>
          <label htmlFor="nft-address" className="block text-gray-700 text-sm font-bold mb-2">
            Address
            <ExpandableSection>
              Smart contract address of the NFT. It uniquely identifies the contract that manages the NFTs you're
              declaring.
            </ExpandableSection>
          </label>
          <input
            id="nft-address"
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Contract Address"
            value={nftAddress}
            onChange={e => setNftAddress(e.target.value)}
          />
        </div>

        <div>
          <label htmlFor="nft-id" className="block text-gray-700 text-sm font-bold mb-2">
            NFT ID{" "}
            <ExpandableSection>The unique identifier for the specific NFT within its collection.</ExpandableSection>
          </label>
          <input
            id="nft-id"
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT ID"
            value={nftId}
            onChange={e => setNftId(e.target.value)}
          />
        </div>

        <div>
          <label htmlFor="min-price" className="block text-gray-700 text-sm font-bold mb-2">
            Min Price
            <ExpandableSection>
              The minimum price for the NFT. No contract lower than that price will be generated, and the bot will try
              to get more than the price out of the auction.
            </ExpandableSection>
          </label>
          <input
            id="min-price"
            required
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Minimum Price"
            value={minPrice}
            onChange={e => setMinPrice(e.target.value)}
          />
        </div>

        <div>
          <label htmlFor="nft-description" className="block text-gray-700 text-sm font-bold mb-2">
            Description of NFT (optional)
            <ExpandableSection>
              <p>
                Additional description you'll give to the bot for the sale of that NFT. Can be a backstory, or anything
                you want.
              </p>
            </ExpandableSection>
          </label>
          <textarea
            id="nft-description"
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="NFT Description"
            value={nftDescription}
            onChange={e => setNftDescription(e.target.value)}
          ></textarea>
        </div>

        <div>
          <label htmlFor="sell-prompt" className="block text-gray-700 text-sm font-bold mb-2">
            Custom prompt on how to sell it (optional)
            <ExpandableSection>
              <p>Give instructions on what the bot should do to sell it. For example, being greedy vs lenient.</p>
            </ExpandableSection>
          </label>
          <textarea
            id="sell-prompt"
            className="appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
            placeholder="Custom Selling Instructions"
            value={sellPrompt}
            onChange={e => setSellPrompt(e.target.value)}
          ></textarea>
        </div>

        <button
          type="submit"
          className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
        >
          Submit NFT
        </button>
      </form>

      <div className="mt-8">
        {nftListings.map((nft, index) => (
          <div
            key={index}
            className={`border p-4 mb-4 rounded cursor-pointer ${selectedNFTKey === `${nft.id}:${nft.chain_id}` ? "bg-gray-100" : ""
              }`}
            onClick={() => setSelectedNFTKey(`${nft.id}:${nft.chain_id}`)}
          >
            <p className="font-bold">Name: {nft.name}</p>
            <p>Address: {nft.address}</p>
            <p>Min Price: {nft.min_price.toFixed(2)}</p>
            <p>Description: {nft.description || "N/A"}</p>
            <p>Custom Prompt: {nft.custom_prompt || "N/A"}</p>
          </div>
        ))}
      </div>
      {selectedNFTKey && (
        <button
          onClick={handleRemoveNFT}
          className="bg-red-500 hover:bg-red-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
        >
          Remove
        </button>
      )}
    </main>
  );
};

const App: React.FC = () => {
  const [isConfigured, setIsConfigured] = useState(false);

  useEffect(() => {
    fetchStatus();
  }, []);

  const fetchStatus = async () => {
    const response = await fetch("/auctioneer:auctioneer:template.os/status", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: "config" }),
    });
    const data = await response.json();
    setIsConfigured(data.status === "manage-nfts");
  };

  const handleConfigSubmit = async (configData: ConfigData) => {
    await fetch("/auctioneer:auctioneer:template.os/config", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(configData),
    });
    setIsConfigured(true);
  };

  return <div>{!isConfigured ? <InitialConfig onSubmit={handleConfigSubmit} /> : <NFTManager />}</div>;
};

export default App;
