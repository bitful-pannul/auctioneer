import React from "react";

/**
 * Site footer
 */
export const Footer = () => {
  return (
    <div className="min-h-0 py-5 px-1 mb-11 lg:mb-0">
      <div></div>
      <div className="w-full">
        <ul className="menu menu-horizontal w-full">
          <div className="flex justify-center items-center gap-2 text-sm w-full">
            <div className="flex justify-center items-center gap-2">
              <p className="m-0 text-center">Running on</p>
              <a
                className="flex justify-center items-center gap-1"
                href="https://kinode.org/"
                target="_blank"
                rel="noreferrer"
              >
                <span className="link">Kinode</span>
              </a>
            </div>
            <span>·</span>
            <div className="text-center">
              <a href="https://github.com/bitful-pannul/auctioneer" target="_blank" rel="noreferrer" className="link">
                Github Repo
              </a>
            </div>
          </div>
        </ul>
      </div>
    </div>
  );
};
