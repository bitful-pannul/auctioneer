import{B as l,b as w,s as y,i as p,I as h,d as g,e as k,f as O,h as L,H as m,j as E}from"./index-agxrSU0P.js";class x extends l{constructor({callbackSelector:e,cause:t,data:o,extraData:c,sender:d,urls:a}){var i;super(t.shortMessage||"An error occurred while fetching for an offchain result.",{cause:t,metaMessages:[...t.metaMessages||[],(i=t.metaMessages)!=null&&i.length?"":[],"Offchain Gateway Call:",a&&["  Gateway URL(s):",...a.map(f=>`    ${w(f)}`)],`  Sender: ${d}`,`  Data: ${o}`,`  Callback selector: ${e}`,`  Extra data: ${c}`].flat()}),Object.defineProperty(this,"name",{enumerable:!0,configurable:!0,writable:!0,value:"OffchainLookupError"})}}class M extends l{constructor({result:e,url:t}){super("Offchain gateway response is malformed. Response data must be a hex value.",{metaMessages:[`Gateway URL: ${w(t)}`,`Response: ${y(e)}`]}),Object.defineProperty(this,"name",{enumerable:!0,configurable:!0,writable:!0,value:"OffchainLookupResponseMalformedError"})}}class R extends l{constructor({sender:e,to:t}){super("Reverted sender address does not match target contract address (`to`).",{metaMessages:[`Contract address: ${t}`,`OffchainLookup sender address: ${e}`]}),Object.defineProperty(this,"name",{enumerable:!0,configurable:!0,writable:!0,value:"OffchainLookupSenderMismatchError"})}}function $(n,e){if(!p(n,{strict:!1}))throw new h({address:n});if(!p(e,{strict:!1}))throw new h({address:e});return n.toLowerCase()===e.toLowerCase()}const v="0x556f1830",S={name:"OffchainLookup",type:"error",inputs:[{name:"sender",type:"address"},{name:"urls",type:"string[]"},{name:"callData",type:"bytes"},{name:"callbackFunction",type:"bytes4"},{name:"extraData",type:"bytes"}]};async function C(n,{blockNumber:e,blockTag:t,data:o,to:c}){const{args:d}=g({data:o,abi:[S]}),[a,i,f,r,s]=d;try{if(!$(c,a))throw new R({sender:a,to:c});const u=await A({data:f,sender:a,urls:i}),{data:b}=await k(n,{blockNumber:e,blockTag:t,data:O([r,L([{type:"bytes"},{type:"bytes"}],[u,s])]),to:c});return b}catch(u){throw new x({callbackSelector:r,cause:u,data:o,extraData:s,sender:a,urls:i})}}async function A({data:n,sender:e,urls:t}){var c;let o=new Error("An unknown error occurred.");for(let d=0;d<t.length;d++){const a=t[d],i=a.includes("{data}")?"GET":"POST",f=i==="POST"?{data:n,sender:e}:void 0;try{const r=await fetch(a.replace("{sender}",e).replace("{data}",n),{body:JSON.stringify(f),method:i});let s;if((c=r.headers.get("Content-Type"))!=null&&c.startsWith("application/json")?s=(await r.json()).data:s=await r.text(),!r.ok){o=new m({body:f,details:s!=null&&s.error?y(s.error):r.statusText,headers:r.headers,status:r.status,url:a});continue}if(!E(s)){o=new M({result:s,url:a});continue}return s}catch(r){o=new m({body:f,details:r.message,url:a})}}throw o}export{A as ccipFetch,C as offchainLookup,S as offchainLookupAbiItem,v as offchainLookupSignature};
