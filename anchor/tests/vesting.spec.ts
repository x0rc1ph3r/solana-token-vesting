import * as anchor from '@coral-xyz/anchor'
import { Program } from '@coral-xyz/anchor'
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js'
import { Vesting } from '../target/types/vesting'
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from '@solana/spl-token'

describe('vesting', () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env()
  anchor.setProvider(provider)

  const connection = provider.connection;
  const payer = provider.wallet as anchor.Wallet;

  const program = anchor.workspace.Vesting as Program<Vesting>;

  let mint: PublicKey;

  const receiver = new anchor.Wallet(payer.payer)


  it('Lock tokens', async () => {

    mint = await createMint(
      connection,                 // Solana connection
      payer.payer,                // Fee payer
      payer.publicKey,    // Mint authority
      null,                       // Freeze authority (optional)
      9                           // Decimals
    );

    const signerAta = (await getOrCreateAssociatedTokenAccount(
      connection,
      payer.payer,                // Fee payer
      mint,                       // Mint address
      payer.publicKey          // Owner of the token account
    )).address;

    const mintAmount = 1_000_000_000_000; // 1,000 tokens (adjust for decimals)
    await mintTo(
      connection,
      payer.payer,                // Fee payer
      mint,                       // Mint address
      signerAta,       // Destination token account
      payer.publicKey,              // Authority to mint tokens
      mintAmount                  // Amount to mint
    );

    await program.methods
      .lock(
        receiver.publicKey,
        new anchor.BN(10000000000),
        new anchor.BN(1702288720),
        new anchor.BN(1733930920),
      )
      .accounts({
        signerAta,
        mint
      })
      .rpc()

    const [vaultInfo] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_info"), payer.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    )

    const vaultInfoData = await program.account.vaultInfo.fetch(vaultInfo);

    console.log("Locking:");
    console.table({
      "Mint address": vaultInfoData.mint.toBase58(),
      "Reciever address": vaultInfoData.receiver.toBase58(),
      "Amount Locked": vaultInfoData.amount.toNumber() / 10 ** 9,
      "Amount Withdrawn": vaultInfoData.amountUnlocked.toNumber() / 10 ** 9,
      "Total Weeks": vaultInfoData.totalWeeks.toNumber(),
      "Start time": vaultInfoData.startTime.toNumber(),
      "End time": vaultInfoData.endTime.toNumber(),
    });

    expect(vaultInfoData.receiver).toEqual(payer.publicKey);
    expect(vaultInfoData.mint).toEqual(mint);
  })

  it('Unlock tokens', async () => {
    await program.methods
      .unlock()
      .accounts({
        signer: receiver.publicKey,
        receiver: receiver.publicKey,
        mint
      })
      .signers([receiver.payer])
      .rpc()

    const [vaultInfo] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_info"), payer.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    )

    const vaultInfoData = await program.account.vaultInfo.fetch(vaultInfo);

    console.log("Unlocking:");
    console.table({
      "Mint address": vaultInfoData.mint.toBase58(),
      "Reciever address": vaultInfoData.receiver.toBase58(),
      "Amount Locked": vaultInfoData.amount.toNumber() / 10 ** 9,
      "Amount Withdrawn": vaultInfoData.amountUnlocked.toNumber() / 10 ** 9,
      "Total Weeks": vaultInfoData.totalWeeks.toNumber(),
      "Start time": vaultInfoData.startTime.toNumber(),
      "End time": vaultInfoData.endTime.toNumber(),
    });

    expect(vaultInfoData.amount.toNumber()).toEqual(vaultInfoData.amountUnlocked.toNumber());
  })

})
