from pathlib import Path

path = Path("programs/service_referral_protocol/src/lib.rs")
source = path.read_text()

source = source.replace(
    "constants::assert_percentages().map_err(|_| ProtocolError::InvalidPercentages)?;",
    "require!(constants::percentages_valid(), ProtocolError::InvalidPercentages);",
)
source = source.replace(
    "pub fn record_qualified_revenue(ctx: Context<RecordQualifiedRevenue>, amount: u64) -> Result<()> {",
    "pub fn record_qualified_revenue<'info>(ctx: Context<'info, RecordQualifiedRevenue<'info>>, amount: u64) -> Result<()> {",
)
source = source.replace(
    "ctx.accounts.token_program.to_account_info(),\n                Transfer {",
    "token::ID,\n                Transfer {",
)
source = source.replace(
    "token_program.to_account_info(),\n            Transfer {",
    "token::ID,\n            Transfer {",
)
for level in range(1, 11):
    source = source.replace(
        f"ctx.accounts.upline_{level}.to_account_info()",
        f"ctx.accounts.upline_{level}.as_ref()",
    )

path.write_text(source)
