import * as anchor from "@coral-xyz/anchor"
import { Keypair, PublicKey } from "@solana/web3.js";
import { BankrunProvider } from "anchor-bankrun";
import { startAnchor, BanksClient, ProgramTestContext, Clock } from "solana-bankrun";
import IDL from "../target/idl/vesting.json"
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { Vesting } from "anchor/target/types/vesting";
import { createMint, mintTo } from "spl-token-bankrun";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { BN } from "bn.js";

describe("Vesting Smart Contract Tests", () => {

    const companyName = "companyName"
    let beneficiary: Keypair;
    let context: ProgramTestContext;
    let provider: BankrunProvider;
    let program: anchor.Program<Vesting>;
    let banksClient: BanksClient;
    let employer: Keypair;
    let mint: PublicKey;
    let beneficiaryProvider: BankrunProvider;
    let program2: anchor.Program<Vesting>;
    let vestingAccountKey: PublicKey;
    let treasuryTokenAccount: PublicKey;
    let employeeAccount: PublicKey;

    beforeAll(async () => {
        beneficiary = new anchor.web3.Keypair();

        context = await startAnchor(
            "",
            [{ name: "vesting", programId: new PublicKey(IDL.address) }],
            [
                {
                    address: beneficiary.publicKey,
                    info: {
                        lamports: 1_000_000_000,
                        data: Buffer.alloc(0),
                        owner: SYSTEM_PROGRAM_ID,
                        executable: false,
                    }
                }
            ]
        );

        provider = new BankrunProvider(context);

        anchor.setProvider(provider);

        program = new anchor.Program<Vesting>(IDL as Vesting, provider);

        banksClient = context.banksClient;

        employer = provider.wallet.payer;

        //@ts-expect-error
        mint = await createMint(banksClient, employer, employer.publicKey, null, 9);

        beneficiaryProvider = new BankrunProvider(context);
        beneficiaryProvider.wallet = new NodeWallet(beneficiary);

        program2 = new anchor.Program<Vesting>(IDL as Vesting, beneficiaryProvider);

        [vestingAccountKey] = PublicKey.findProgramAddressSync(
            [Buffer.from(companyName)],
            program.programId,
        );

        [treasuryTokenAccount] = PublicKey.findProgramAddressSync(
            [Buffer.from("vesting_treasury"), Buffer.from(companyName)],
            program.programId,
        );

        [employeeAccount] = PublicKey.findProgramAddressSync(
            [Buffer.from("employee_vesting"),
            beneficiary.publicKey.toBuffer(),
            vestingAccountKey.toBuffer()],
            program.programId,
        );
    })

    it("Create a vesting account", async () => {
        const tx = await program.methods.createVestingAccount(companyName).accounts({
            signer: employer.publicKey,
            mint,
            tokenProgram: TOKEN_PROGRAM_ID,
        }).rpc({ commitment: "confirmed" });

        const vestingAccountData = await program.account.vestingAccount.fetch(
            vestingAccountKey,
            'confirmed'
        )

        console.log(vestingAccountData);
        console.log("Create vesting Account:", tx);
    })

    it("Fund the treasury token account", async () => {
        const amount = 10_000 * 10 ** 9;
        const mint_tx = await mintTo(
            //@ts-expect-error
            banksClient,
            employer,
            mint,
            treasuryTokenAccount,
            employer.publicKey,
            amount
        )
        console.log("mint treasury token account:", mint_tx);
    })

    it("Create employee vesting account", async () => {
        const tx2 = await program.methods.createEmployeeAccount(
            new BN(0),
            new BN(100),
            new BN(100),
            new BN(0),
        ).accounts({
            beneficiary: beneficiary.publicKey,
            vestingAccount: vestingAccountKey
        }).rpc({ commitment: 'confirmed', skipPreflight: true });

        console.log("Create employeee vesting acc tx:", tx2);
        console.log("Employee account: ", employeeAccount.toBase58());
    })

    it("claim the employee's vested tokens", async () => {
        await new Promise((resolve) => setTimeout(resolve, 1000));

        const currentClock = await banksClient.getClock();
        context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                1000n
            ))

        const tx3 = await program2.methods.claimToken(companyName).accounts({ tokenProgram: TOKEN_PROGRAM_ID }).rpc({ commitment: 'confirmed' })
        console.log("claim tokens tx", tx3);
    })
})