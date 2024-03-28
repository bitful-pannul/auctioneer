import { ReactNode } from 'react'

export const Header = ({ action }: { action?: ReactNode }) => {
  return (
    <div className="h-16 border-b-1 border-white box-border">
      <div className="max-w-6xl m-auto h-full flex justify-between items-center sm:px-8 lt-sm:px-4">
        <h1 className="flex items-center font-bold cursor-pointer display">
          KinoShop
        </h1>
        <div className="flex items-center gap-2">
          {action}
          <a
            href="https://github.com/bitful-pannul/auctioneer"
            target="_blank"
            className="flex-col-center text-white transition-all hover:scale-95"
          >
            <span className="inline-flex w-8 h-8 i-carbon:logo-github"></span>
          </a>
        </div>
      </div>
    </div>
  )
}
