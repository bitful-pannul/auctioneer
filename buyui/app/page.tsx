"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { NextPage } from "next";
import { parseEther } from "viem/utils";
import { erc721ABI, useAccount, useContractRead, useContractWrite } from "wagmi";
import { Address, AddressInput, EtherInput, InputBase, IntegerInput } from "~~/components/scaffold-eth";
import NFTEscrow from "~~/contracts/NFTEscrow.json";
import { useScaffoldContractRead, useScaffoldContractWrite } from "~~/hooks/scaffold-eth";

const ESCROW_ADDRESS = "0x7b1431A0f20A92dD7E42A28f7Ba9FfF192F36DF3";

const Home: NextPage = () => {
  const { address: connectedAddress } = useAccount();
  const searchParams = useSearchParams();

  const [nftAddress, setNftAddress] = useState(searchParams.get("nft") || "");
  const [nftId, setNftId] = useState(searchParams.get("id") || "");
  const [price, setPrice] = useState(searchParams.get("price") || "");
  const [uid, setUid] = useState(searchParams.get("uid") || "");
  const [validUntil, setValidUntil] = useState(searchParams.get("valid") || "");
  const [signature, setSignature] = useState(searchParams.get("signature") || "");
  //
  const [chainId, setChainId] = useState(searchParams.get("chain_id") || "");

  const [tokenURI, setTokenURI] = useState("");
  const [metadata, setMetadata] = useState(null);
  const [metadataVisible, setMetadataVisible] = useState(false); // New state for toggling visibility

  const { data: tokenURIData, isLoading: isDataLoading } = useContractRead({
    abi: erc721ABI,
    address: nftAddress,
    functionName: "tokenURI",
    args: [BigInt(nftId)],
  });

  const { writeAsync, isLoading } = useContractWrite({
    address: ESCROW_ADDRESS,
    abi: NFTEscrow,
    functionName: "buyNFT",
    args: [nftAddress, nftId, price, uid, validUntil, signature],
    value: parseEther(price),
  });

  const buyy = async () => {
    //console.log('argsa: ', nftAddress, nftId, price, uid, validUntil, signature)
    await writeAsync();
  };

  useEffect(() => {
    if (tokenURIData) {
      const fetchMetadata = async () => {
        try {
          let resolvedTokenURI = tokenURIData;
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
  }, [tokenURIData]);

  return (
    <>
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

          <div>
            <label>
              NFT Address:
              <AddressInput value={nftAddress} onChange={e => setNftAddress(e)} />
            </label>
          </div>
          <div>
            <label>
              NFT ID:
              <IntegerInput value={nftId} onChange={e => setNftId(e.toString())} />
            </label>
          </div>
          <div>
            <label>
              Price:
              <IntegerInput value={price} onChange={e => setPrice(e.toString())} />
            </label>
          </div>
          <div>
            <label>
              UID:
              <IntegerInput value={uid} onChange={e => setUid(e.toString())} />
            </label>
          </div>
          <div>
            <label>
              Valid Until:
              <IntegerInput value={validUntil} onChange={e => setValidUntil(e.toString())} />
            </label>
          </div>
          <div>
            <label>
              Auctioneer Signature:
              <InputBase value={signature} onChange={e => setSignature(e)} />
            </label>
          </div>

          <div>---</div>
          <button onClick={buyy}>{"Buy NFT"}</button>
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

export default Home;
