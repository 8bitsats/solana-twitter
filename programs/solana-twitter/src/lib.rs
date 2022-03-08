use anchor_lang::prelude::*;

declare_id!("H4FBVtcR7yKNWJWnwK6wwEtREYaF5Vi6w9R1uHZXRw7F");

#[program]
pub mod solana_twitter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
