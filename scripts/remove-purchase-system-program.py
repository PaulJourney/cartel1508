from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Rust clients: line-state parser, intentionally limited to PurchaseAndDistribute blocks.
changed = 0
for path in (ROOT / 'integration-tests/tests').glob('*.rs'):
    lines = path.read_text().splitlines(True)
    out = []
    inside = False
    for line in lines:
        if '.accounts(service_referral_protocol::accounts::PurchaseAndDistribute {' in line:
            inside = True
        if inside and 'system_program: anchor_lang::system_program::ID' in line:
            changed += 1
            continue
        out.append(line)
        if inside and line.strip() == '})':
            inside = False
    path.write_text(''.join(out))

if changed == 0:
    raise SystemExit('no Rust purchase SystemProgram fields removed')

# JS smoke: only remove SystemProgram inside account maps belonging to purchaseAndDistribute.
path = ROOT / 'scripts/devnet-transaction-smoke.mjs'
lines = path.read_text().splitlines(True)
out = []
seen_purchase = False
inside_accounts = False
js_changed = 0
for line in lines:
    if '.purchaseAndDistribute(' in line:
        seen_purchase = True
    if seen_purchase and '.accounts({' in line:
        inside_accounts = True
    if inside_accounts and 'systemProgram: SystemProgram.programId' in line:
        js_changed += 1
        continue
    out.append(line)
    if inside_accounts and line.strip() == '})':
        inside_accounts = False
        seen_purchase = False
path.write_text(''.join(out))

if js_changed == 0:
    raise SystemExit('no JS purchase SystemProgram fields removed')

print(f'removed {changed} Rust and {js_changed} JS purchase SystemProgram fields')
