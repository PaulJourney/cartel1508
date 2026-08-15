from pathlib import Path

source = Path('programs/revenue_adapter/src/lib.rs').read_text()
cargo = Path('programs/revenue_adapter/Cargo.toml').read_text()

checks = {
    'adapter has no admin mutation surface': all(x not in source for x in [
        'set_admin', 'transfer_admin', 'set_verifier', 'set_referral_program',
        'set_revenue_authority', 'pause(', 'unpause('
    ]),
    'production verifier is fail closed': 'MAINNET_VERIFIER_PROGRAM' in source and '11111111111111111111111111111111' in source,
    'verifier must be executable at initialization': '#[account(executable)]' in source,
    'verifier authorization uses a program-scoped PDA': 'VERIFIER_AUTHORITY_SEED' in source and 'expected_verifier_authority' in source,
    'qualified source is a PDA with signer seeds': 'REVENUE_AUTHORITY_SEED' in source and 'CpiContext::new_with_signer' in source,
    'anti replay receipt is deterministic': 'RECEIPT_SEED' in source and 'event_id.as_ref()' in source and 'init,' in source,
    'receipt binds beneficiary mint amount and evidence': all(x in source for x in [
        'receipt.event_id = event_id', 'receipt.evidence_hash = evidence_hash',
        'receipt.beneficiary = ctx.accounts.beneficiary.wallet', 'receipt.mint = mint',
        'receipt.amount = amount'
    ]),
    'zero event and evidence hashes rejected': 'InvalidEventId' in source and 'InvalidEvidenceHash' in source,
    'source token must be revenue authority canonical ATA': all(x in source for x in [
        'source_token.owner', 'get_associated_token_address_with_program_id',
        'NonCanonicalRevenueAccount'
    ]),
    'source must be prefunded before liabilities': 'source_token.amount >= amount' in source and 'InsufficientQualifiedFunds' in source,
    'adapter CPI targets referral record qualified revenue': 'service_referral_protocol::cpi::record_qualified_revenue' in source,
    'adapter cannot select arbitrary referral program': 'Program<\'info, service_referral_protocol::program::ServiceReferralProtocol>' in source,
    'isolated staging workspace is explicit': '[workspace]' in cargo,
}

for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
