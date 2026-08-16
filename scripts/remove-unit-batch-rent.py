from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

# --- Core ------------------------------------------------------------------
lib_path = ROOT / 'programs/service_referral_protocol/src/lib.rs'
lib = lib_path.read_text()
lib = lib.replace('ProtocolError::BatchTooLarge', 'ProtocolError::PurchaseTooLarge')
lib = lib.replace('next_batch_index', 'next_purchase_index')

batch_write = '''        let batch = &mut ctx.accounts.batch;\n        batch.bump = ctx.bumps.batch;\n        batch.owner = ctx.accounts.wallet.key();\n        batch.batch_index = batch_index;\n        batch.mint = mint;\n        batch.units = units;\n        batch.first_unit_id = first_unit_id;\n        batch.last_unit_id = last_unit_id;\n        batch.purchased_at = now;\n\n'''
if lib.count(batch_write) != 1:
    raise SystemExit(f'UnitBatch write block expected once, found {lib.count(batch_write)}')
lib = lib.replace(batch_write, '', 1)
lib = lib.replace('        let batch_index = ctx\n            .accounts\n            .user\n            .next_purchase_index\n', '        let purchase_index = ctx\n            .accounts\n            .user\n            .next_purchase_index\n', 1)

account_block = '''    #[account(\n        init,\n        payer = wallet,\n        seeds = [b"batch", wallet.key().as_ref(), &user.next_purchase_index.to_le_bytes()],\n        bump,\n        space = UnitBatch::SPACE\n    )]\n    pub batch: Box<Account<'info, UnitBatch>>,\n    pub token_program: Program<'info, Token>,\n    pub system_program: Program<'info, System>,\n'''
if lib.count(account_block) != 1:
    raise SystemExit(f'Purchase batch account block expected once, found {lib.count(account_block)}')
lib = lib.replace(account_block, "    pub token_program: Program<'info, Token>,\n", 1)

metric_tail = '''        add_protocol_treasury_metrics(\n            p,\n            mint,\n            service,\n            unallocated,\n            rounding_remainder,\n            pioneer_unassigned,\n        )?;\n        Ok(())\n    }\n'''
if lib.count(metric_tail) != 1:
    raise SystemExit(f'purchase tail expected once, found {lib.count(metric_tail)}')
event_tail = '''        add_protocol_treasury_metrics(\n            p,\n            mint,\n            service,\n            unallocated,\n            rounding_remainder,\n            pioneer_unassigned,\n        )?;\n\n        emit!(UnitsPurchased {\n            buyer: ctx.accounts.wallet.key(),\n            mint,\n            purchase_index,\n            units,\n            first_unit_id,\n            last_unit_id,\n            purchased_at: now,\n        });\n        Ok(())\n    }\n'''
lib = lib.replace(metric_tail, event_tail, 1)

accounts_marker = '\n#[derive(Accounts)]\npub struct Initialize'
if lib.count(accounts_marker) != 1:
    raise SystemExit('Initialize accounts marker not unique')
event_struct = '''\n#[event]\npub struct UnitsPurchased {\n    pub buyer: Pubkey,\n    pub mint: Pubkey,\n    pub purchase_index: u64,\n    pub units: u64,\n    pub first_unit_id: u128,\n    pub last_unit_id: u128,\n    pub purchased_at: i64,\n}\n\n#[derive(Accounts)]\npub struct Initialize'''
lib = lib.replace(accounts_marker, event_struct, 1)
lib = lib.replace('    #[msg("Batch amount is too large")]\n    BatchTooLarge,', '    #[msg("Purchase amount is too large")]\n    PurchaseTooLarge,')
lib_path.write_text(lib)

# --- State -----------------------------------------------------------------
state_path = ROOT / 'programs/service_referral_protocol/src/state.rs'
state = state_path.read_text().replace('next_batch_index', 'next_purchase_index')
state = re.sub(
    r'\n#\[account\]\npub struct UnitBatch \{.*?\n\}\n\nimpl UnitBatch \{.*?\n\}\n',
    '\n',
    state,
    flags=re.S,
)
if 'UnitBatch' in state:
    raise SystemExit('UnitBatch remains in state.rs')
state_path.write_text(state)

# --- Rust integration tests -------------------------------------------------
for path in (ROOT / 'integration-tests/tests').glob('*.rs'):
    text = path.read_text()
    text = text.replace('next_batch_index', 'next_purchase_index')
    # Remove PDA derivations whose local variable contains "batch".
    text = re.sub(
        r'\n\s*let \([A-Za-z0-9_]*batch[A-Za-z0-9_]*, _\) = Pubkey::find_program_address\(.*?\n\s*\);',
        '',
        text,
        flags=re.S,
    )
    # Remove purchase account fields.
    text = re.sub(r'^\s*batch:\s*[A-Za-z0-9_]+,?\s*\n', '', text, flags=re.M)
    # Remove generated UnitBatch fetches/assert blocks if present.
    text = re.sub(r'^\s*let\s+[A-Za-z0-9_]*batch[A-Za-z0-9_]*\s*=.*?;\s*\n', '', text, flags=re.M)
    text = re.sub(r'^\s*assert_eq!\([^\n]*batch[^\n]*\);\s*\n', '', text, flags=re.M)
    path.write_text(text)

# --- Devnet smoke -----------------------------------------------------------
smoke_path = ROOT / 'scripts/devnet-transaction-smoke.mjs'
smoke = smoke_path.read_text()
smoke = re.sub(
    r'\nconst \[[A-Za-z0-9_]*Batch[A-Za-z0-9_]*\] = PublicKey\.findProgramAddressSync\(.*?\n\);',
    '',
    smoke,
    flags=re.S,
)
smoke = re.sub(r'^\s*batch:\s*[A-Za-z0-9_]+,?\s*\n', '', smoke, flags=re.M)
smoke = re.sub(r'^const\s+[A-Za-z0-9_]*Batch[A-Za-z0-9_]*\s*=\s*await program\.account\.unitBatch\.fetch\([^\n]+\);\s*\n', '', smoke, flags=re.M)
smoke = re.sub(r'^invariant\([^\n]*Batch[^\n]*\);\s*\n', '', smoke, flags=re.M)
smoke = re.sub(r'^\s*[A-Za-z0-9_]*Batch0:\s*[A-Za-z0-9_]*Batch0\.toBase58\(\),\s*\n', '', smoke, flags=re.M)
smoke = smoke.replace('batchIndex', 'purchaseIndex').replace('nextBatchIndex', 'nextPurchaseIndex')
smoke_path.write_text(smoke)

# --- Static gates -----------------------------------------------------------
gates_path = ROOT / 'scripts/static-gates.py'
gates = gates_path.read_text().replace('next_batch_index', 'next_purchase_index')
old = "    'service units use global monotonic Unit IDs': all(x in state for x in ['next_unit_id', 'first_unit_id', 'last_unit_id']) and 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)' in final_purchase and 'ctx.accounts.protocol.next_unit_id = next_unit_id' in final_purchase and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,\n"
new = "    'service units use global monotonic Unit IDs': 'next_unit_id' in state and all(x in source for x in ['pub struct UnitsPurchased', 'first_unit_id: u128', 'last_unit_id: u128', 'purchase_index: u64', 'emit!(UnitsPurchased', 'allocate_unit_range(ctx.accounts.protocol.next_unit_id, units)', 'ctx.accounts.protocol.next_unit_id = next_unit_id']) and 'first_local_unit_index' not in source and 'last_local_unit_index' not in source,\n    'purchase has no per-purchase rent account': 'UnitBatch' not in state and 'UnitBatch' not in source and 'pub batch:' not in production_accounts and 'payer = wallet' not in production_accounts and 'pub system_program' not in production_accounts,\n"
if gates.count(old) != 1:
    raise SystemExit(f'unit ID static gate expected once, found {gates.count(old)}')
gates = gates.replace(old, new, 1)
gates_path.write_text(gates)

# --- README ----------------------------------------------------------------
readme_path = ROOT / 'README.md'
readme = readme_path.read_text()
readme = readme.replace(
    'The same transaction transfers USDT/USDC into the protocol vault, creates the unit batch, updates activity and performs 50/43/2/5 accounting.',
    'The same transaction transfers USDT/USDC into the protocol vault, updates activity and performs 50/43/2/5 accounting. It creates no per-purchase rent account.',
)
insert_after = '- Expiration settlement: treasury-destined value can be physically settled permissionlessly; whoever submits that transaction pays its SOL fee. No permanent platform keeper is required.\n'
addition = insert_after + '- Purchase history: global logical Unit IDs, purchase index, mint, unit count and timestamp are emitted in the `UnitsPurchased` event. No `UnitBatch` PDA is created, so repeated/unlimited purchases do not accumulate per-purchase account rent.\n'
if readme.count(insert_after) != 1:
    raise SystemExit('README gas insertion marker missing')
readme = readme.replace(insert_after, addition, 1)
readme_path.write_text(readme)

print('UnitBatch rent account removed; UnitsPurchased event installed')
