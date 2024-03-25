import React, { useEffect, useState } from "react";
import { parseEther } from "viem/utils";
import { useAccount, useReadContract, useWriteContract, useSwitchChain } from "wagmi";
import { erc721Abi } from "viem";

import NFTEscrow from "./abis/NFTEscrow.json";
import { Header } from "./components/layout/Header";
import { NetworkSwitcher } from "./components/SwitchNetworks";
import { WalletModal } from "./components/WalletModal";
import Button from "antd/es/button";
import { shorten } from "@did-network/dapp-sdk";

const ESCROW_ADDRESS = "0x7b1431A0f20A92dD7E42A28f7Ba9FfF192F36DF3";

const App = () => {
  const { switchChain } = useSwitchChain();
  const { writeContract, status } = useWriteContract();
  const { address } = useAccount()

  const [show, setShow] = useState(false)

  const toggleModal = (e: boolean) => {
    setShow(e)
  }

  const [searchParams, setSearchParams] = useState(new URLSearchParams(window.location.search));
  const [nftAddress, setNftAddress] = useState(searchParams.get("nft") || "");
  const [nftId, setNftId] = useState(searchParams.get("id") || "");
  const [price, setPrice] = useState(searchParams.get("price") || "");
  const [uid, setUid] = useState(searchParams.get("uid") || "");
  const [validUntil, setValidUntil] = useState(searchParams.get("valid") || "");
  const [signature, setSignature] = useState(searchParams.get("sig") || "");
  const [chainId, setChainId] = useState(searchParams.get("chain") || null);

  const [tokenURI, setTokenURI] = useState("");
  const [metadata, setMetadata] = useState(null);
  const [metadataVisible, setMetadataVisible] = useState(false);

  const { data: tokenURIdata, isError, isLoading } = useReadContract({
    address: nftAddress as any,
    abi: erc721Abi,
    functionName: "tokenURI",
    args: [BigInt(nftId)],
  });

  useEffect(() => {
    // Example of using switchNetwork and handling chainId
    if (chainId) {
      switchChain({ chainId: parseInt(chainId) });
    }

    if (tokenURIdata) {
      const fetchMetadata = async () => {
        try {
          let resolvedTokenURI = tokenURIdata.toString();
          if (resolvedTokenURI.startsWith("ipfs://")) {
            resolvedTokenURI = `https://ipfs.io/ipfs/${resolvedTokenURI.slice(7)}`;
          }

          const response = await fetch(resolvedTokenURI);
          const metadata = await response.json();
          setMetadata(metadata);

          let imageURL = metadata.image;
          if (imageURL && imageURL.startsWith("ipfs://")) {
            imageURL = `https://ipfs.io/ipfs/${imageURL.slice(7)}`;
          }
          setTokenURI(imageURL || "");
        } catch (error) {
          console.error("Failed to fetch metadata:", error);
        }
      };
      fetchMetadata();
    }
  }, [nftAddress, nftId, chainId, switchChain]);

  const handleBuyNFT = () => {
    console.log('let us try');
    writeContract({
      address: ESCROW_ADDRESS,
      abi: NFTEscrow,
      functionName: "buyNFT",
      args: [nftAddress, BigInt(nftId), BigInt(price), BigInt(uid), BigInt(validUntil), signature],
      value: parseEther(price),
    });
    console.log('done, status: ', status);
  };


  return (
    <>
      <Header
        action={
          <>
            <NetworkSwitcher />
            <WalletModal open={show} onOpenChange={toggleModal} close={() => setShow(false)}>
              {({ isLoading }) => (
                <Button className="flex items-center mr-4">
                  {isLoading && (
                    <span className="i-line-md:loading-twotone-loop inline-flex mr-1 w-4 h-4 text-white"></span>
                  )}{' '}
                  {address ? shorten(address) : 'Connect Wallet'}
                </Button>
              )}
            </WalletModal>
          </>
        }
      />
      <div className="flex items-center flex-col flex-grow pt-10">
        <div className="px-5">
          <h1 className="text-center">
            <span className="block text-2xl mb-2"></span>
            <span className="block text-4xl font-bold"></span>
          </h1>
          {tokenURI && (
            <div className="my-4">
              <img src={tokenURI} alt="NFT Image" className="max-w-xs max-h-96" />
            </div>
          )}
          {tokenURI && (
            <div className="my-4">
              <div
                className="my-4 flex items-center cursor-pointer"
                onClick={() => setMetadataVisible(!metadataVisible)}
              >
                <span className="text-lg font-bold">NFT Metadata</span>
                <span className="ml-2">{metadataVisible ? "▼" : "▶"}</span>
              </div>
              {metadataVisible && metadata && (
                <div className="mt-2">
                  <pre>{JSON.stringify(metadata, null, 2)}</pre>
                </div>
              )}
            </div>
          )}

          {/* Form for NFT purchase details */}
          <div className="mt-8 space-y-4">
            <input type="text" value={nftAddress} onChange={(e) => setNftAddress(e.target.value)} placeholder="NFT Address" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input type="number" value={nftId} onChange={(e) => setNftId(e.target.value)} placeholder="NFT ID" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input type="text" value={price} onChange={(e) => setPrice(e.target.value)} placeholder="Price in WEI" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input type="text" value={uid} onChange={(e) => setUid(e.target.value)} placeholder="UID" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input type="number" value={validUntil} onChange={(e) => setValidUntil(e.target.value)} placeholder="Valid Until" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <input type="text" value={signature} onChange={(e) => setSignature(e.target.value)} placeholder="Signature" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500" />
            <button onClick={handleBuyNFT} className="w-full px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-700 disabled:bg-blue-300 transition duration-300 ease-in-out">Buy NFT</button>
          </div>
          {/* {txData && (
            <div>
              <h2>Transaction Data:</h2>
              <pre>{JSON.stringify(txData, null, 2)}</pre>
            </div>
          )} */}
        </div>
      </div>
    </>
  );

};

export default App;