from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rw(rel):
    p = ROOT / rel
    return p, p.read_text()


def once(text, old, new, label):
    n = text.count(old)
    if n != 1:
        raise SystemExit(f"{label}: expected 1 match, found {n}")
    return text.replace(old, new, 1)


# purchase_distribution.rs
p, t = rw('integration-tests/tests/purchase_distribution.rs')
t = once(t, 'ctx.svm.assert_token_balance(&treasury_usdc, 9_998_000);', 'ctx.svm.assert_token_balance(&treasury_usdc, 4_998_000);', 'activation treasury')
t = once(t, 'ctx.svm.assert_token_balance(&vault_usdc, 2_000);', 'ctx.svm.assert_token_balance(&vault_usdc, 5_002_000);', 'activation vault')
start = t.index('    let direct = 50 * UNIT;\n')
end_marker = '    assert_eq!(purchase_treasury_delta + purchase_vault_liability, 100 * UNIT);\n'
end = t.index(end_marker, start) + len(end_marker)
new = '''    let self_reward = 50 * UNIT;\n    let sponsor_network = 15 * UNIT;\n    let network_unallocated = 28 * UNIT;\n    let service_fee = 5 * UNIT;\n    let pioneer_pool = 2 * UNIT;\n    let pioneer_per_slot = pioneer_pool / 100;\n    let pioneer_assigned = pioneer_per_slot * 2;\n    let pioneer_unassigned = pioneer_pool - pioneer_assigned;\n    let purchase_treasury_delta = network_unallocated + service_fee + pioneer_unassigned;\n    let purchase_vault_liability = self_reward + sponsor_network + pioneer_assigned;\n\n    assert_eq!(purchase_treasury_delta, 34_960_000);\n    assert_eq!(purchase_vault_liability, 65_040_000);\n    assert_eq!(purchase_treasury_delta + purchase_vault_liability, 100 * UNIT);\n'''
t = t[:start] + new + t[end:]
t = t.replace('9_998_000 + purchase_treasury_delta', '4_998_000 + purchase_treasury_delta')
t = t.replace('2_000 + purchase_vault_liability', '5_002_000 + purchase_vault_liability')
t = once(t, 'assert_eq!(sponsor_state.self_accrued_usdc, direct);', 'assert_eq!(sponsor_state.self_accrued_usdc, 5 * UNIT);', 'sponsor own SELF')
t = once(t, 'assert_eq!(sponsor_state.network_claimable_usdc, 0);', 'assert_eq!(sponsor_state.network_claimable_usdc, sponsor_network);', 'sponsor network')
t = once(t, 'assert_eq!(buyer_state.lifetime_service_units, 100);', 'assert_eq!(buyer_state.lifetime_service_units, 100);\n    assert_eq!(buyer_state.self_accrued_usdc, self_reward);', 'buyer SELF assertion')
t = t.replace('9_300_000 + network_unallocated as u128', '4_300_000 + network_unallocated as u128')
t = once(t, 'let sponsor_claim = direct + 22_000;', 'let sponsor_claim = 5 * UNIT + sponsor_network + 22_000;', 'sponsor claim')
t = once(t, 'ctx.svm.assert_token_balance(&vault_usdc, pioneer_per_slot);', 'ctx.svm.assert_token_balance(&vault_usdc, self_reward + pioneer_per_slot);', 'vault after sponsor claim')
t = once(t, 'ctx.svm.assert_token_balance(&buyer_usdc, pioneer_per_slot);', 'ctx.svm.assert_token_balance(&buyer_usdc, self_reward + pioneer_per_slot);', 'buyer claim amount')
p.write_text(t)

# deep_network.rs
p, t = rw('integration-tests/tests/deep_network.rs')
t = t.replace('fn full_genealogy_pays_l1_direct_and_l2_through_l10_but_never_l11()', 'fn full_genealogy_pays_self_and_nine_uplines_but_never_tenth_upline()')
t = t.replace('// 0=L11, 1=L10, ... 9=L2, 10=L1, 11=buyer.', '// 0=U11, 1=U10, 2=U9, ... 10=U1/sponsor, 11=buyer/SELF.')
t = t.replace('// Register a real chain L11 -> L10 -> ... -> L1 -> buyer.', '// Register a real ancestor chain ending at the buyer.')
t = t.replace('// Activate L10 through L2. L11 intentionally remains inactive; if the target\n    // purchase ever traversed an accidental eleventh level, its accounting would\n    // still change (expired), which we assert does not happen.\n    for i in 1..=9 {', '// Activate exactly the nine payable uplines U9 through U1. U10/U11 stay inactive.\n    for i in 2..=10 {')
t = t.replace('let mut uplines = [technical_root; 9];', 'let mut uplines = [technical_root; 8];')
t = t.replace('for slot in 0..9 {', 'for slot in 0..8 {')
t = t.replace('// buyer -> L1(index10) direct; network array is exactly L2(index9) ... L10(index1).', '// buyer receives SELF 50%; sponsor index10 is U1, then indices9..2 are U2..U9.')
old_assert = '''    // L1 receives exactly the direct 50% and no network-depth reward.\n    assert_eq!(\n        after[10].self_accrued_usdc - before[10].self_accrued_usdc,\n        50 * UNIT\n    );\n    assert_eq!(\n        after[10].network_claimable_usdc - before[10].network_claimable_usdc,\n        0\n    );\n\n    // Genealogical L2-L10 map to indices 9 down to 1.\n    let expected = [\n        15 * UNIT,\n        9 * UNIT,\n        6 * UNIT,\n        4 * UNIT,\n        2_500_000,\n        2 * UNIT,\n        1_500_000,\n        1 * UNIT,\n        2 * UNIT,\n    ];\n    for (offset, amount) in expected.iter().enumerate() {\n        let index = 9 - offset;\n        assert_eq!(\n            after[index].network_claimable_usdc - before[index].network_claimable_usdc,\n            *amount,\n            \"unexpected network delta at genealogical level {}\",\n            offset + 2\n        );\n    }\n\n    // L11 exists and is the real parent of L10, but it is not part of the supplied\n    // production account set. The target purchase must not alter any of its network\n    // buckets, including expired accounting while inactive.\n    assert_eq!(after[0].network_claimable_usdc, before[0].network_claimable_usdc);\n    assert_eq!(after[0].network_pending_usdc, before[0].network_pending_usdc);\n    assert_eq!(after[0].lifetime_expired_usdc, before[0].lifetime_expired_usdc);\n'''
new_assert = '''    // Buyer is SELF and receives exactly 50%; target purchase gives the sponsor no SELF delta.\n    assert_eq!(\n        after[11].self_accrued_usdc - before[11].self_accrued_usdc,\n        50 * UNIT\n    );\n    assert_eq!(\n        after[10].self_accrued_usdc - before[10].self_accrued_usdc,\n        0\n    );\n\n    // U1 sponsor gets 15%, followed by U2..U9 across indices 9 down to 2.\n    assert_eq!(\n        after[10].network_claimable_usdc - before[10].network_claimable_usdc,\n        15 * UNIT\n    );\n    let expected = [\n        9 * UNIT,\n        6 * UNIT,\n        4 * UNIT,\n        2_500_000,\n        2 * UNIT,\n        1_500_000,\n        1 * UNIT,\n        2 * UNIT,\n    ];\n    for (offset, amount) in expected.iter().enumerate() {\n        let index = 9 - offset;\n        assert_eq!(\n            after[index].network_claimable_usdc - before[index].network_claimable_usdc,\n            *amount,\n            \"unexpected network delta at upline {}\",\n            offset + 2\n        );\n    }\n\n    // U10 and U11 exist in the real ancestry but are outside the nine-upline cap.\n    for index in [0usize, 1usize] {\n        assert_eq!(after[index].network_claimable_usdc, before[index].network_claimable_usdc);\n        assert_eq!(after[index].network_pending_usdc, before[index].network_pending_usdc);\n        assert_eq!(after[index].lifetime_expired_usdc, before[index].lifetime_expired_usdc);\n    }\n'''
t = once(t, old_assert, new_assert, 'deep-network assertions')
p.write_text(t)

# grace_expiry.rs
p, t = rw('integration-tests/tests/grace_expiry.rs')
t = t.replace('fn grace_preserves_unclaimed_direct_temporarily_then_inactivity_expires_it()', 'fn grace_preserves_self_and_network_temporarily_then_inactivity_expires_them()')
needle = '            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,\n            batch:'
replacement = '            upline_4: technical_root, upline_5: technical_root, upline_6: technical_root,\n            upline_7: technical_root, upline_8: technical_root,\n            batch:'
if t.count(needle) != 2:
    raise SystemExit(f'grace account fixture: expected 2 matches, found {t.count(needle)}')
t = t.replace(needle, replacement)
t = once(t, 'assert_eq!(sponsor_grace.self_accrued_usdc, 50 * UNIT, "GRACE must temporarily preserve the L1 direct reward");', 'assert_eq!(sponsor_grace.self_accrued_usdc, 5 * UNIT, "GRACE preserves sponsor own SELF reward");\n    assert_eq!(sponsor_grace.network_pending_usdc, 15 * UNIT, "GRACE preserves sponsor U1 network reward as pending");', 'grace preserved buckets')
t = once(t, 'assert_eq!(read_user(&ctx, sponsor_pda).self_accrued_usdc, 50 * UNIT);', 'let still_grace = read_user(&ctx, sponsor_pda);\n    assert_eq!(still_grace.self_accrued_usdc, 5 * UNIT);\n    assert_eq!(still_grace.network_pending_usdc, 15 * UNIT);', 'grace failed claim state')
t = t.replace('// 50 direct plus sponsor\'s 0.022 Pioneer due (0.002 own activation + 0.020 buyer purchase).', '// 5 SELF + 15 pending U1 network + sponsor\'s 0.022 Pioneer due.')
t = t.replace('50_022_000u128', '20_022_000u128')
t = t.replace('treasury_after - treasury_before, 50_022_000', 'treasury_after - treasury_before, 20_022_000')
t = t.replace('vault_before - vault_after, 50_022_000', 'vault_before - vault_after, 20_022_000')
p.write_text(t)

# claim_activity.rs: replace the old sponsor-direct tail with SELF + IC-A assertions.
p, t = rw('integration-tests/tests/claim_activity.rs')
start = t.index('    // Sponsor never buys 10 units')
end = t.rfind('\n}')
new_tail = '''    // Sponsor never buys 10 units and is INACTIVE. Buyer buys 100 units, becomes\n    // ACTIVE, receives SELF 50%, while sponsor's U1 15% plus its current Pioneer\n    // entitlement are treasury-destined immediately under IC-A.\n    let purchase_ix = ctx\n        .program()\n        .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {\n            wallet: buyer.pubkey(),\n            protocol,\n            user: buyer_pda,\n            user_source: buyer_usdc,\n            vault_authority,\n            usdt_vault: vault_usdt,\n            usdc_vault: vault_usdc,\n            service_treasury_usdt: treasury_usdt,\n            service_treasury_usdc: treasury_usdc,\n            direct_referrer: sponsor_pda,\n            upline_1: technical_root,\n            upline_2: technical_root,\n            upline_3: technical_root,\n            upline_4: technical_root,\n            upline_5: technical_root,\n            upline_6: technical_root,\n            upline_7: technical_root,\n            upline_8: technical_root,\n            batch: buyer_batch,\n            token_program: spl_token::id(),\n            system_program: anchor_lang::system_program::ID,\n        })\n        .args(service_referral_protocol::instruction::PurchaseAndDistribute { units: 100 })\n        .instruction()\n        .expect(\"purchase ix\");\n    ctx.execute_instruction(purchase_ix, &[&buyer])\n        .expect(\"purchase tx\")\n        .assert_success();\n\n    let sponsor_after_purchase = read_user(&ctx, sponsor_pda);\n    let buyer_after_purchase = read_user(&ctx, buyer_pda);\n    let protocol_after_purchase = read_protocol(&ctx, protocol);\n    assert_eq!(sponsor_after_purchase.active_until, 0);\n    assert_eq!(sponsor_after_purchase.network_claimable_usdc, 0);\n    assert_eq!(sponsor_after_purchase.lifetime_expired_usdc, 15_020_000u128);\n    assert_eq!(buyer_after_purchase.self_accrued_usdc, 50 * UNIT);\n    assert!(buyer_after_purchase.active_until > 0);\n    assert_eq!(protocol_after_purchase.lifetime_expired_usdc, 15_020_000u128);\n\n    // 15.020 expired sponsor/Pioneer + 28 unallocated upper network + 5 service\n    // + 1.960 unassigned Pioneer = 49.980 treasury. Buyer SELF + buyer Pioneer\n    // remain collateralized in the vault.\n    ctx.svm.assert_token_balance(&buyer_usdc, 0);\n    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);\n    ctx.svm.assert_token_balance(&sponsor_usdc, 0);\n\n    let claim_ix = ctx\n        .program()\n        .accounts(service_referral_protocol::accounts::Claim {\n            wallet: sponsor.pubkey(),\n            protocol,\n            user: sponsor_pda,\n            vault_authority,\n            vault_token: vault_usdc,\n            destination: sponsor_usdc,\n            token_program: spl_token::id(),\n        })\n        .args(service_referral_protocol::instruction::Claim {})\n        .instruction()\n        .expect(\"claim ix\");\n    let claim_outcome = ctx\n        .execute_instruction(claim_ix, &[&sponsor])\n        .expect(\"inactive claim program result\");\n    assert!(!claim_outcome.is_success(), \"inactive sponsor must not claim\");\n\n    // The purchase already settled every whole-atomic sponsor entitlement, so a\n    // second permissionless settlement must fail rather than double-transfer value.\n    ctx.svm.expire_blockhash();\n    let settle_ix = ctx\n        .program()\n        .accounts(service_referral_protocol::accounts::SettleExpired {\n            settler: buyer.pubkey(),\n            protocol,\n            user: sponsor_pda,\n            vault_authority,\n            vault_token: vault_usdc,\n            service_treasury_token: treasury_usdc,\n            token_program: spl_token::id(),\n        })\n        .args(service_referral_protocol::instruction::SettleExpired {})\n        .instruction()\n        .expect(\"settle ix\");\n    let settle_outcome = ctx\n        .execute_instruction(settle_ix, &[&buyer])\n        .expect(\"settle result\");\n    assert!(!settle_outcome.is_success(), \"expired sponsor value must not settle twice\");\n    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);\n\n    let buyer_claim_ix = ctx\n        .program()\n        .accounts(service_referral_protocol::accounts::Claim {\n            wallet: buyer.pubkey(),\n            protocol,\n            user: buyer_pda,\n            vault_authority,\n            vault_token: vault_usdc,\n            destination: buyer_usdc,\n            token_program: spl_token::id(),\n        })\n        .args(service_referral_protocol::instruction::Claim {})\n        .instruction()\n        .expect(\"buyer claim ix\");\n    ctx.execute_instruction(buyer_claim_ix, &[&buyer])\n        .expect(\"buyer claim tx\")\n        .assert_success();\n\n    ctx.svm.assert_token_balance(&buyer_usdc, 50_020_000);\n    ctx.svm.assert_token_balance(&treasury_usdc, 49_980_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 0);\n    assert_eq!(50_020_000u64 + 49_980_000u64, 100 * UNIT);\n'''
t = t[:start] + new_tail + t[end:]
p.write_text(t)

print('SELF economics integration tests rewritten')
