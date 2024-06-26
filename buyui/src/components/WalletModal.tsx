import { shorten } from '@did-network/dapp-sdk'
import { ReactNode, useMemo, useState } from 'react'
import { useAccount, useConnect, useDisconnect } from 'wagmi'

import { Button } from '../components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '../components/ui/dialog'

export function WalletModal(props: {
  children: ({ isLoading }: { isLoading?: boolean }) => ReactNode
  open: boolean
  onOpenChange: (open: boolean) => void
  close?: () => void
}) {
  const { connectAsync, connectors, isPending } = useConnect()
  const { address, isConnecting } = useAccount()
  const { disconnect } = useDisconnect()
  const [pendingConnectorId, setPendingConnectorId] = useState('')

  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogTrigger asChild>{props.children({ isLoading: isPending })}</DialogTrigger>
      <DialogContent className="sm:max-w-[425px] md:top-70 bg-black">
        <DialogHeader>
          <DialogTitle>Wallet</DialogTitle>
          <DialogDescription>connect to web3</DialogDescription>
        </DialogHeader>
        <div className="w-full">
          {address ? (
            <>
              <div className="flex-center my-3">{shorten(address)}</div>
              <Button
                onClick={(e) => {
                  disconnect()
                  props.close?.()
                }}
                className="normal flex-center w-full font-[OpenSans]"
              >
                disconnect <span className="i-carbon:cookie"></span>
              </Button>
            </>
          ) : (
            <div className="flex-col-center">
              {connectors.map((connector) => (
                <Button
                  key={connector.id}
                  onClick={async (e) => {
                    setPendingConnectorId(connector.id)
                    await connectAsync({
                      connector,
                    })
                    props.close?.()
                  }}
                  className="alt w-full mb-3 font-[OpenSans]"
                  size="lg"
                >
                  {connector.name}
                  {/* {!connector.ready && ' (unsupported)'} */}
                  {isConnecting && connector.id === pendingConnectorId && ' (connecting)'}
                </Button>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
