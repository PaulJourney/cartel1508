from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]

# Rust integration clients: remove SystemProgram only from PurchaseAndDistribute blocks.
for path in (ROOT / 'integration-tests/tests').glob('*.rs'):
    text = path.read_text()
    pattern = re.compile(
        r'(\.accounts\(service_referral_protocol::accounts::PurchaseAndDistribute \{)(.*?)(\n\s*\}\))',
        re.S,
    )
    def fix_block(match):
        body = match.group(2)
        body = re.sub(r'^\s*system_program:\s*anchor_lang::system_program::ID,?\s*\n', '', body, flags=re.M)
        return match.group(1) + body + match.group(3)
    text = pattern.sub(fix_block, text)
    path.write_text(text)

# JS devnet client: remove SystemProgram only from purchaseAndDistribute account maps.
smoke_path = ROOT / 'scripts/devnet-transaction-smoke.mjs'
smoke = smoke_path.read_text()
pattern = re.compile(
    r'(\.purchaseAndDistribute\([^\n]+\).*?\.accounts\(\{)(.*?)(\n\s*\}\)\n\s*\.instruction\(\);)',
    re.S,
)
def fix_js(match):
    body = re.sub(r'^\s*systemProgram:\s*SystemProgram\.programId,?\s*\n', '', match.group(2), flags=re.M)
    return match.group(1) + body + match.group(3)
smoke, count = pattern.subn(fix_js, smoke)
if count < 2:
    raise SystemExit(f'expected at least two purchaseAndDistribute smoke blocks, found {count}')
smoke_path.write_text(smoke)

# Pre-mainnet gate: enforce current SELF + 9-upline traversal and rentless purchase ABI.
gate_path = ROOT / 'scripts/pre-mainnet-gate.py'
gate = gate_path.read_text()
gate = gate.replace(
    'blockers.append("final L2-L10 43% production schedule is not frozen")',
    'blockers.append("final nine-upline 43% production schedule is not frozen")',
)
gate = gate.replace(
    "    if 'for i in 0..9' not in core_lib:\n        blockers.append(\"production referral traversal is not capped at genealogical L10\")\n",
    "    if 'for i in 1..9' not in core_lib or 'pub upline_8:' not in core_lib or 'pub upline_9:' in core_lib:\n        blockers.append(\"production referral traversal is not frozen to sponsor plus eight ancestors\")\n    if 'pub struct UnitsPurchased' not in core_lib or 'emit!(UnitsPurchased' not in core_lib:\n        blockers.append(\"purchase unit ranges are not emitted as the final event-based audit trail\")\n    purchase_accounts = core_lib.split('pub struct PurchaseAndDistribute', 1)[1].split('pub struct SettleExpired', 1)[0]\n    if 'UnitBatch' in core_lib or 'pub batch:' in purchase_accounts or 'pub system_program' in purchase_accounts:\n        blockers.append(\"purchase still carries a per-purchase rent/account-creation surface\")\n",
)
gate_path.write_text(gate)

print('rentless purchase clients and pre-mainnet gate aligned')
