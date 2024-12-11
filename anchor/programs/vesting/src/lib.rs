#![allow(clippy::result_large_err)]

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount, Transfer},
};

declare_id!("6FuQ5pZHttiDZCnMXbjR1SGtM7UGRp33jrEGXJtgxg4d");

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
        let vault = &ctx.accounts.vault;
        let signer_ata = &ctx.accounts.signer_ata;
        let token_program = &ctx.accounts.token_program;

        require!(end_time > start_time, CustomError::EndBeforeStart);

        let transfer = Transfer {
            from: signer_ata.to_account_info(),
            to: vault.to_account_info(),
            authority: signer.to_account_info(),
        };
        let transfer_context = CpiContext::new(token_program.to_account_info(), transfer);
        token::transfer(transfer_context, amount)?;

        vault_info.mint = mint.key();
        vault_info.receiver = receiver;
        vault_info.amount = amount;
        vault_info.amount_unlocked = 0;
        vault_info.start_time = start_time;
        vault_info.end_time = end_time;
        vault_info.total_weeks = (end_time - start_time) / 604800;

        if vault_info.total_weeks == 0 {
            return Err(CustomError::InvalidVestingPeriod.into());
        }

        Ok(())
    }

    pub fn unlock(ctx: Context<Unlock>) -> Result<()> {
        let vault_info = &mut ctx.accounts.vault_info;
        let vault = &ctx.accounts.vault;
        let receiver_ata = &ctx.accounts.receiver_ata;
        let token_program = &ctx.accounts.token_program;

        let now: u64 = Clock::get()?.unix_timestamp.try_into().unwrap();

        require!(
            now > vault_info.start_time,
            CustomError::CliffPeriodNotPassed
        );

        let weeks_passed = (now - vault_info.start_time) / 604800;
        let total_entitled_weeks = std::cmp::min(weeks_passed, vault_info.total_weeks);
        let entitled_amount = (vault_info.amount * total_entitled_weeks / vault_info.total_weeks)
            .saturating_sub(vault_info.amount_unlocked);

        require!(entitled_amount > 0, CustomError::NoTokensToUnlock);

        let seeds = [b"vault".as_ref(), vault.mint.as_ref(), &[ctx.bumps.vault]];
        let signer = &[&seeds[..]];
        let transfer = Transfer {
            from: vault.to_account_info(),
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
        token::mint = mint,
        token::authority = vault,
        payer = signer,
        seeds=[b"vault".as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,

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
        constraint = vault.mint == mint.key(),
        seeds=[b"vault".as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,

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
pub struct VaultInfo {
    mint: Pubkey,
    receiver: Pubkey,
    amount: u64,
    amount_unlocked: u64,
    start_time: u64,
    end_time: u64,
    total_weeks: u64,
}

#[error_code]
pub enum CustomError {
    #[msg("End time cannot be less than start time")]
    EndBeforeStart,
    #[msg("Cliff period not yet passed")]
    CliffPeriodNotPassed,
    #[msg("No tokens to unlock")]
    NoTokensToUnlock,
    #[msg("Vesting period should be one week minimum")]
    InvalidVestingPeriod,
}
