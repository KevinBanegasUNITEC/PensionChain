use anchor_lang::prelude::*;

declare_id!("68vbvrrUcTARuM512d8bz6eAPuKaEjFhM22mB8tJJMx9");

#[program]
pub mod pension_chain {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
