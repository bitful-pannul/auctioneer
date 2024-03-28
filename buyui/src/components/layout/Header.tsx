import { ReactNode } from 'react'

export const Header = ({ action }: { action?: ReactNode }) => {
  return (
    <div className="h-16">
      <div className="max-w-6xl m-auto h-full flex justify-between items-center sm:px-8 lt-sm:px-4">
        <div className="flex items-center">
          <h1 className='display mr-8'>Kinode<span className='text-xs'>&reg;</span></h1>
          <h1 className='text-3xl mr-8 font-normal'>/</h1>
          <h1 className='text-3xl mr-8'>Barter (Buy)</h1>
        </div>
        <div className="flex items-center gap-2">
          {action}
          <a
            href="https://github.com/bitful-pannul/auctioneer"
            target="_blank"
            className="flex-col-center text-primary transition-all hover:scale-95"
          >
            <span className="inline-flex w-8 h-8 i-carbon:logo-github"></span>
          </a>
        </div>
      </div>
    </div>
  )
}
