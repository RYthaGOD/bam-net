import { Connection, PublicKey } from '@solana/web3.js';

async function main() {
    console.log("Initializing Phase 0 Data Spike for Jito BAM Attestations...");
    
    // Using a public RPC for the spike to keep it "cheap" as requested
    const connection = new Connection("https://api.mainnet-beta.solana.com", "confirmed");

    // We know the Jito BAM Boost Program is: BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb
    // We will scan recent transactions to see if they contain attestation logs or data
    const bamBoostProgramId = new PublicKey("BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb");
    
    console.log(`Querying recent signatures for BAM-related Program: ${bamBoostProgramId.toBase58()}...`);
    
    try {
        const signatures = await connection.getSignaturesForAddress(bamBoostProgramId, { limit: 5 });
        
        if (signatures.length === 0) {
            console.log("No recent signatures found for this program. Data might not be stored directly here.");
            return;
        }

        console.log(`Found ${signatures.length} recent signatures. Fetching the latest transaction...`);
        
        const latestSig = signatures[0].signature;
        const tx = await connection.getTransaction(latestSig, {
            maxSupportedTransactionVersion: 0
        });

        if (!tx) {
            console.log(`Transaction ${latestSig} not found or parsing failed.`);
            return;
        }

        console.log(`\n--- Transaction Details: ${latestSig} ---`);
        console.log("Slot:", tx.slot);
        console.log("Block Time:", tx.blockTime);
        
        // Let's check the logs to see if there's any attestation proof emitted
        if (tx.meta && tx.meta.logMessages) {
            console.log("\nTransaction Logs:");
            tx.meta.logMessages.forEach(log => console.log("  " + log));
        }

        console.log("\nIf attestations are on-chain, they would likely appear as instruction data or log emissions.");
        console.log("If we don't see them here, they may be written to dedicated PDAs or stored off-chain (IPFS/Arweave) with just a hash on-chain.");

    } catch (e) {
        console.error("Error during spike:", e);
    }
}

main().catch(console.error);
