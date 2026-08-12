from pathlib import Path
p = Path('programs/service_referral_protocol/src/lib.rs').read_text()
checks = {
 'no Ownable/admin state': 'owner_admin' not in p and 'set_admin' not in p,
 'no pause instruction': 'pub fn pause' not in p,
 'no treasury mutation instruction': 'set_treasury' not in p,
 'no referral mutation instruction': 'set_referrer' not in p,
 'service purchase isolated': 'purchase_service_units' in p and 'record_qualified_revenue' in p,
 '10 explicit upline accounts': all(f'upline_{i}' in p for i in range(1,11)),
 'pull claim exists': 'pub fn claim' in p,
}
for name, ok in checks.items():
    print(('PASS' if ok else 'FAIL'), name)
if not all(checks.values()):
    raise SystemExit(1)
