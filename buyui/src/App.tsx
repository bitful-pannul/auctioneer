import React, { useEffect, useState } from "react";
import { parseEther, parse } from "viem/utils";
import { useAccount, useReadContract, useWriteContract, useSwitchChain } from "wagmi";
import { erc721Abi, formatEther, parseUnits } from "viem";

import NFTEscrow from "./abis/NFTEscrow.json";
import { Header } from "./components/layout/Header";
import { NetworkSwitcher } from "./components/SwitchNetworks";
import { WalletModal } from "./components/WalletModal";
import Button from "antd/es/button";
import { shorten } from "@did-network/dapp-sdk";

const ESCROW_ADDRESS = "0x7b1431A0f20A92dD7E42A28f7Ba9FfF192F36DF3";

const App = () => {
  const { switchChain } = useSwitchChain();
  const { writeContractAsync, status, failureReason } = useWriteContract();

  const { address } = useAccount()

  const [show, setShow] = useState(false)

  const toggleModal = (e: boolean) => {
    setShow(e)
  }
  const [errorMessage, setErrorMessage] = useState("");

  const [searchParams, setSearchParams] = useState(new URLSearchParams(window.location.search));
  const [nftAddress, setNftAddress] = useState(searchParams.get("nft") || "");
  const [nftId, setNftId] = useState(searchParams.get("id") || "");
  const [price, setPrice] = useState(searchParams.get("price") || "");
  const [uid, setUid] = useState(searchParams.get("uid") || "");
  const [validUntil, setValidUntil] = useState(searchParams.get("valid") || "");
  const [signature, setSignature] = useState(searchParams.get("sig") || "");
  const [chainId, setChainId] = useState(searchParams.get("chain") || null);
  const [checkboxState, setCheckboxState] = useState(false);

  const [tokenURI, setTokenURI] = useState("");
  const [metadata, setMetadata] = useState(null);
  const [metadataVisible, setMetadataVisible] = useState(false);

  const { data: tokenURIdata, isError, isLoading } = useReadContract({
    address: nftAddress,
    abi: erc721Abi,
    functionName: "tokenURI",
    args: [BigInt(nftId)],
  });

  const { data: approvalData } = useReadContract({
    address: nftAddress,
    abi: erc721Abi,
    functionName: "getApproved",
    args: [BigInt(nftId)],
  });

  useEffect(() => {
    // Example of using switchNetwork and handling chainId
    if (chainId) {
      switchChain({ chainId: parseInt(chainId) });
    }
    console.log("useEffect triggered");
    console.log("tokenURIdata:", tokenURIdata);
    console.log("approvalData:", approvalData);
    console.log("nftAddress:", nftAddress);
    console.log("nftId:", nftId);

    const checkApproval = async () => {
      if (nftAddress && nftId && approvalData) {
        const isApproved = approvalData.toString();
        setCheckboxState(isApproved === ESCROW_ADDRESS);
      }
    }

    const fetchMetadata = async () => {
      // Ensure this runs only when tokenURIdata is available
      if (tokenURIdata) {
        console.log("Fetching metadata");
        try {
          let resolvedTokenURI = tokenURIdata.toString();
          if (resolvedTokenURI.startsWith("ipfs://")) {
            resolvedTokenURI = `https://ipfs.io/ipfs/${resolvedTokenURI.slice(7)}`;
          }

          const response = await fetch(resolvedTokenURI);
          const metadata = await response.json();
          console.log("Fetched metadata:", metadata);
          setMetadata(metadata);

          let imageURL = metadata.image;
          if (imageURL && imageURL.startsWith("ipfs://")) {
            imageURL = `https://ipfs.io/ipfs/${imageURL.slice(7)}`;
          }
          setTokenURI(imageURL || "");
        } catch (error) {
          console.error("Failed to fetch metadata:", error);
        }
      }
    };


    fetchMetadata();
    checkApproval();
  }, [nftAddress, nftId, chainId, switchChain, tokenURIdata, approvalData]);

  // 40000000 WEI 
  // 0.000004 ETH

  const handleBuyNFT = async () => {
    console.log('all values: ', nftAddress, nftId, price, uid, validUntil, signature);
    console.log('value...: ', parseUnits(price, -18));
    try {
      const result = await writeContractAsync({
        address: ESCROW_ADDRESS,
        abi: NFTEscrow,
        functionName: "buyNFT",
        args: [nftAddress, BigInt(nftId), BigInt(price), BigInt(uid), BigInt(validUntil), signature],
        value: parseUnits(price, -18),
      });
      console.log('fail reason is: ', failureReason);
      console.log('Transaction result: ', result);
    } catch (error) {
      console.log('fail reason is: ', failureReason);
      console.error('Transaction failed: ', error);
      setErrorMessage(error.message || "An unknown error occurred");
      setTimeout(() => setErrorMessage(""), 5000);
    }
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
                className="flex items-center cursor-pointer"
                onClick={() => setMetadataVisible(!metadataVisible)}
              >
                <span className="text-lg font-bold">NFT Metadata</span>
                <span className="ml-2">{metadataVisible ? "▼" : "▶"}</span>
              </div>
              {metadataVisible && metadata && (
                <div className="overflow-auto p-4 max-h-96 w-full">
                  <div className="max-w-full bg-white shadow-lg rounded-lg p-5">
                    <pre className="whitespace-pre-wrap text-sm">{JSON.stringify(metadata, null, 2)}</pre>
                  </div>
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
            <div>
              <input
                type="checkbox"
                checked={checkboxState}
                readOnly
              />
              <label>Escrow is allowed to transact NFT</label>
            </div>
            {!checkboxState && (
              <div className="px-4 py-2 my-2 text-white bg-red-500 rounded">
                This NFT might not be approved for the escrow. Please approve it before proceeding.
              </div>
            )}
            <button onClick={handleBuyNFT} className="w-full px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-700 disabled:bg-blue-300 transition duration-300 ease-in-out">Buy NFT</button>
          </div>
          {errorMessage && (
            <div className="px-4 py-2 my-2 text-white bg-red-500 rounded">
              {errorMessage}
            </div>
          )}
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