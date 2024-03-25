import { createConfig, http } from 'wagmi'
import { optimism, sepolia, base, arbitrum } from 'wagmi/chains'
import { walletConnect, metaMask, coinbaseWallet, injected, safe } from 'wagmi/connectors'

export const wagmiConfig = createConfig({
  chains: [sepolia, optimism, base, arbitrum],
  transports: {
    [optimism.id]: http(),
    [sepolia.id]: http(),
    [base.id]: http(),
    [arbitrum.id]: http(),
  },
  connectors: [
    walletConnect({
      projectId: 'f18c88f1b8f4a066d3b705c6b13b71a8',
    }),
    injected(),
  ],
})
