#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer},
};

declare_id!("GphCVK2aMjMHF9jFsubw37tMqni2dT4DQwv6WztrCYaw");

#[program]
pub mod vesting {
    use super::*;
    use anchor_spl::token;

    pub fn lock(
        ctx: Context<Lock>,
        receiver: Pubkey,
        amount: u64,
        start_time: u64,
        end_time: u64,
    ) -> Result<()> {
        let vault_info = &mut ctx.accounts.vault_info;
        let mint = &ctx.accounts.mint;
        let signer = &ctx.accounts.signer;
        let vault_ata = &ctx.accounts.vault_ata;
        let signer_ata = &ctx.accounts.signer_ata;
        let token_program = &ctx.accounts.token_program;

        require!(end_time > start_time, CustomError::EndBeforeStart);

        let transfer = Transfer {
            from: signer_ata.to_account_info(),
            to: vault_ata.to_account_info(),
            authority: signer.to_account_info(),
        };
        let transfer_context = CpiContext::new(token_program.to_account_info(), transfer);
        token::transfer(transfer_context, amount)?;

        vault_info.mint = mint.key();
        vault_info.receiver = receiver;
        vault_info.amount = amount;
        vault_info.amount_unlocked = 0;
        vault_info.start_time = end_time;
        vault_info.start_time = end_time;

        Ok(())
    }

    pub fn unlock(ctx: Context<Unlock>) -> Result<()> {
        let vault_info = &mut ctx.accounts.vault_info;
        let vault = &ctx.accounts.vault;
        let vault_ata = &ctx.accounts.vault_ata;
        let receiver_ata = &ctx.accounts.receiver_ata;
        let token_program = &ctx.accounts.token_program;

        let now: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

        require!(
            now > vault_info.start_time,
            CustomError::CliffPeriodNotPassed
        );

        let passed_seconds = now - vault_info.start_time;
        let total_seconds = vault_info.end_time - vault_info.start_time;

        let entitled_amount = if now >= vault_info.end_time {
            vault_info.amount - vault_info.amount_unlocked
        } else {
            (vault_info.amount * passed_seconds) / total_seconds - vault_info.amount_unlocked
        };

        let seeds = [b"vault".as_ref(), &[ctx.bumps.vault]];
        let signer = &[&seeds[..]];
        let transfer = Transfer {
            from: vault_ata.to_account_info(),
            to: receiver_ata.to_account_info(),
            authority: vault.to_account_info(),
        };
        let token_transfer_context =
            CpiContext::new_with_signer(token_program.to_account_info(), transfer, signer);
        token::transfer(token_transfer_context, entitled_amount)?;

        vault_info.amount_unlocked += entitled_amount;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(receiver: Pubkey)]
pub struct Lock<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init_if_needed,
        seeds=[b"vault"],
        bump,
        payer = signer,
        space = 8 + Vault::INIT_SPACE,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init_if_needed,
        associated_token::mint = mint,
        associated_token::authority = vault,
        payer = signer,
    )]
    pub vault_ata: Account<'info, TokenAccount>,

    #[account(
        init,
        seeds=[b"vault_info", receiver.key().as_ref(), mint.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + VaultInfo::INIT_SPACE,
    )]
    pub vault_info: Account<'info, VaultInfo>,

    #[account(
        mut,
        constraint = signer_ata.owner.key() == signer.key(),
        constraint = signer_ata.mint.key() == mint.key(),
    )]
    pub signer_ata: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unlock<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds=[b"vault"],
        bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        constraint = vault_ata.owner == vault.key(),
        constraint = vault_ata.mint == mint.key(),
    )]
    pub vault_ata: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        associated_token::mint = mint,
        associated_token::authority = receiver,
        payer = signer,
    )]
    pub receiver_ata: Account<'info, TokenAccount>,

    /// CHECK: Just a public key of a Solana account.
    pub receiver: AccountInfo<'info>,

    #[account(
        mut,
        seeds=[b"vault_info", receiver.key().as_ref(), mint.key().as_ref()],
        bump,
        constraint = vault_info.receiver.key() == receiver.key(),
        constraint = vault_info.mint.key() == mint.key(),
        constraint = vault_info.amount_unlocked < vault_info.amount,
    )]
    pub vault_info: Account<'info, VaultInfo>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {}

#[account]
#[derive(InitSpace)]
pub struct VaultInfo {
    mint: Pubkey,
    receiver: Pubkey,
    amount: u64,
    amount_unlocked: u64,
    start_time: u64,
    end_time: u64,
}

#[error_code]
pub enum CustomError {
    EndBeforeStart,
    CliffPeriodNotPassed,
}
