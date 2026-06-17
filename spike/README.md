# Phase-0 data spike (archived)

This folder holds the throwaway TypeScript probe used to answer the project's
original go/no-go question: **can an outsider fetch and parse a BAM
transaction-ordering attestation for free?**

[`spike.ts`](spike.ts) queries a public Solana RPC for recent transactions of
the program the marketing copy implied published attestations
(`BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb`). It found that program to be a
token **claim/vesting** program — not the BAM sequencer, and not a source of
ordering attestations.

That negative result, combined with the wider investigation (see the root
[README](../README.md) → *Investigation*), is why the project pivoted from
"index a public attestation feed" to wrapping the BAM network/stake data that
*is* public. The spike is kept here as a record of that due diligence.

## Running it

```bash
cd spike
npm install
node spike.ts          # Node 24+: runs TypeScript directly
```

> Note: `ts-node` is intentionally not used — version 10.x silently no-ops under
> TypeScript 6 / Node 24. Node 24's native type-stripping runs `.ts` directly.
