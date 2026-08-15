# Revenue Adapter V0.1 — Security Invariants

Status: draft implementation for adversarial review; not approved for mainnet.

## Immutable trust boundaries

1. The adapter has no update/admin instruction.
2. `AdapterConfig` is created once at a deterministic PDA and cannot be reconfigured.
3. The referral protocol address is frozen in `AdapterConfig` and every event must target exactly that executable program.
4. The qualification authority must equal the deterministic `qualified-revenue-authority` PDA derived under the frozen qualification-program ID. A normal wallet cannot substitute for it.
5. The adapter revenue source must equal the deterministic `revenue-authority` PDA owned by this adapter. Its canonical USDT/USDC ATA must already contain the full event amount before accounting begins.
6. Service-unit/activity purchase funds are not a valid qualification source.
7. USDT and USDC are separate rails; only the two frozen mint identities are accepted.

## Replay protection

Every revenue event uses a 32-byte unique event ID. The adapter creates `RevenueReceipt` at PDA seeds:

`["revenue-receipt", event_id]`

Because the receipt account is initialized exactly once, replaying the same event ID fails before a second downstream accounting event can be accepted.

The receipt records event ID, beneficiary account, mint, amount, accepted timestamp/slot and qualification authority.

## Atomicity

`submit_revenue_event` initializes the receipt and performs the CPI into `service_referral_protocol::record_qualified_revenue` in one Solana transaction. If the referral CPI fails for insufficient funding, bad ancestry, bad beneficiary, unsupported mint or any other downstream validation, the whole transaction rolls back, including receipt creation.

## Adversarial cases required before mainnet

- Same event ID submitted twice.
- Human keypair supplied as qualification authority.
- PDA from the wrong qualification program.
- Referral program substitution.
- Non-executable referral account.
- Treasury/deployer/user wallet substituted as revenue authority.
- Non-canonical source token account.
- Unsupported mint.
- Source ATA balance smaller than event amount.
- Beneficiary substitution.
- Upline/ancestry substitution.
- CPI failure confirms receipt rollback.
- Successful event confirms exact one-time receipt and 50/43/2/5 downstream conservation.
- Separate USDT and USDC events cannot cross-account balances.

## Mainnet boundary

This adapter does not define what economic event qualifies as real service revenue. That policy must live in the separate qualification program whose exact Program ID, authority PDA, bytecode and qualification rules are independently audited and frozen before either program becomes immutable.
