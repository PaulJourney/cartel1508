from pathlib import Path
import runpy

path = Path("programs/service_referral_protocol/src/lib.rs")
source = path.read_text()

already_migrated = (
    "pub fn settle_expired(ctx: Context<SettleExpired>)" in source
    and "fn validate_supported_token(" not in source
)
if not already_migrated:
    runpy.run_path("scripts/migrate-v0-9b.py", run_name="__main__")
    source = path.read_text()

# The first V0.9B compiler pass exposed that the migration's presence check was
# too broad: error names appeared at call sites before they existed as enum variants.
needle = '    #[msg("Invalid upline account or ancestry")] InvalidUpline,\n'
errors = (
    needle
    + '    #[msg("Invalid beneficiary user PDA")] InvalidBeneficiary,\n'
    + '    #[msg("User state is not at its canonical PDA")] InvalidUserPda,\n'
    + '    #[msg("Token account is not the canonical associated token account")] NonCanonicalTokenAccount,\n'
    + '    #[msg("No expired pending reward to settle")] NothingToSettle,\n'
)
if '#[msg("Invalid beneficiary user PDA")] InvalidBeneficiary,' not in source:
    if needle not in source:
        raise SystemExit("ProtocolError insertion point not found")
    source = source.replace(needle, errors, 1)

# CpiContext in Anchor 1.1 uses token::ID directly, so the Program argument is
# intentionally retained only as an account constraint. Silence the helper warning.
source = source.replace(
    '    token_program: &Program<\'info, Token>,\n    amount: u64,\n) -> Result<()> {\n    let bump = [p.vault_authority_bump];',
    '    _token_program: &Program<\'info, Token>,\n    amount: u64,\n) -> Result<()> {\n    let bump = [p.vault_authority_bump];',
    1,
)

path.write_text(source)
print("V0.9B migration normalized")
