from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = [
    ROOT / 'integration-tests/tests/claim_activity.rs',
    ROOT / 'integration-tests/tests/grace_expiry.rs',
    ROOT / 'integration-tests/tests/deep_network.rs',
    ROOT / 'integration-tests/tests/batching_equivalence.rs',
]

for path in FILES:
    text = path.read_text()
    if 'use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};' in text:
        text = text.replace(
            'use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};',
            'use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};',
            1,
        )
    elif 'AnchorContext' not in text:
        raise SystemExit(f'{path.name}: unexpected anchor_litesvm import')
    text = text.replace('fn read_user(ctx: &AnchorLiteSVM,', 'fn read_user(ctx: &AnchorContext,')
    text = text.replace('fn read_protocol(ctx: &AnchorLiteSVM,', 'fn read_protocol(ctx: &AnchorContext,')
    path.write_text(text)

# LiteSVM 0.11 has assertion helpers for token balances but no get_token_account.
# Use exact known balances instead of unsupported helper reads.
grace = ROOT / 'integration-tests/tests/grace_expiry.rs'
text = grace.read_text()
old = '''    let treasury_before = ctx.svm.get_token_account(&treasury_usdc).expect("treasury token").amount;\n    let vault_before = ctx.svm.get_token_account(&vault_usdc).expect("vault token").amount;\n\n'''
if text.count(old) != 1:
    raise SystemExit(f'grace pre-balance block expected once, found {text.count(old)}')
text = text.replace(
    old,
    '''    ctx.svm.assert_token_balance(&treasury_usdc, 39_958_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 70_042_000);\n\n''',
    1,
)
old = '''    let treasury_after = ctx.svm.get_token_account(&treasury_usdc).expect("treasury token after").amount;\n    let vault_after = ctx.svm.get_token_account(&vault_usdc).expect("vault token after").amount;\n    assert_eq!(treasury_after - treasury_before, 20_022_000);\n    assert_eq!(vault_before - vault_after, 20_022_000);\n'''
if text.count(old) != 1:
    raise SystemExit(f'grace post-balance block expected once, found {text.count(old)}')
text = text.replace(
    old,
    '''    ctx.svm.assert_token_balance(&treasury_usdc, 59_980_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);\n''',
    1,
)
text = text.replace(
    '    ctx.svm.assert_token_balance(&treasury_usdc, treasury_after);\n    ctx.svm.assert_token_balance(&vault_usdc, vault_after);',
    '    ctx.svm.assert_token_balance(&treasury_usdc, 59_980_000);\n    ctx.svm.assert_token_balance(&vault_usdc, 50_020_000);',
    1,
)
grace.write_text(text)

print('LiteSVM AnchorContext/test balance harness fixed')
