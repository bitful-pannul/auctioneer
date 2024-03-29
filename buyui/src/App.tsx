import { useEffect, useState } from "react";
import { useAccount, useReadContract, useWriteContract, useSwitchChain, useChainId } from "wagmi";
import { erc721Abi, parseUnits } from "viem";

import NFTEscrow from "./abis/NFTEscrow.json";
import { Header } from "./components/layout/Header";
import { NetworkSwitcher } from "./components/SwitchNetworks";
import { WalletModal } from "./components/WalletModal";
import Button from "antd/es/button";
import { shorten } from "@did-network/dapp-sdk";

const ESCROW_ADDRESS = "0x4A3A2c0A385F017501544DcD9C6Eb3f6C63fc38b";

const App = () => {
  const { switchChainAsync } = useSwitchChain();
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

  const [txHash, setTxHash] = useState("");
  const [tokenURI, setTokenURI] = useState("");
  const [metadata, setMetadata] = useState(null);
  const [metadataVisible, setMetadataVisible] = useState(false);

  const [currentChain, setCurrentChain] = useState(null);

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
    const checkChain = async () => {
      if (chainId) {
        await switchChainAsync({ chainId: parseInt(chainId) });
      }
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

    checkChain();
    fetchMetadata();
    checkApproval();
  }, [nftAddress, nftId, chainId, switchChainAsync, tokenURIdata, approvalData]);

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
      console.log('result is: ', result);
      setTxHash(result);
      console.log('fail might be: ', failureReason);
    } catch (error) {
      console.log('fail reason is: ', failureReason);
      console.error('Transaction failed: ', error);
      setErrorMessage(error.message || "An unknown error occurred");
      setTimeout(() => setErrorMessage(""), 5000);
    }
  };

  const handleSwitchChain = async () => {
    try {
      if (chainId) {
        await switchChainAsync({ chainId: parseInt(chainId) });
      }
    } catch (error) {
      console.error("Failed to switch chain:", error);
    }
  }


  return (
    <>
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
            <input type="text" value={nftAddress} onChange={(e) => setNftAddress(e.target.value)} placeholder="NFT Address" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <input type="number" value={nftId} onChange={(e) => setNftId(e.target.value)} placeholder="NFT ID" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <input type="text" value={price} onChange={(e) => setPrice(e.target.value)} placeholder="Price in WEI" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <input type="text" value={uid} onChange={(e) => setUid(e.target.value)} placeholder="UID" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <input type="number" value={validUntil} onChange={(e) => setValidUntil(e.target.value)} placeholder="Valid Until" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <input type="text" value={signature} onChange={(e) => setSignature(e.target.value)} placeholder="Signature" className="w-full px-4 py-2 border border-gray-300 rounded focus:outline-none" />
            <div className="flex items-center">
              <input
                type="checkbox"
                checked={checkboxState}
                // readOnly
                className="mr-2"
              />
              <label>Escrow is allowed to transact NFT</label>
            </div>
            {!checkboxState && (
              <div className="px-4 py-2 my-2 text-white bg-red-500 rounded">
                This NFT might not be approved for the escrow. Please doublecheck if the item is still available.
              </div>
            )}
            {chainId && chainId !== useChainId().toString() && (
              <button onClick={handleSwitchChain} className="bg-orange font-[OpenSans] px-4 py-2 w-full">
                Switch to Correct Network
              </button>
            )}
            <button onClick={handleBuyNFT} className="bg-orange font-[OpenSans] px-4 py-2 w-full">Buy NFT</button>
          </div>
          {errorMessage && (
            <div className="px-4 py-2 my-2 text-white bg-red-500 rounded">
              {errorMessage}
            </div>
          )}
          {txHash && (
            <div>
              <h2>Transaction Data:</h2>
              <pre>{JSON.stringify(txHash, null, 2)}</pre>
            </div>
          )}
        </div>
      </div>
    </>
  );

};

export default App;