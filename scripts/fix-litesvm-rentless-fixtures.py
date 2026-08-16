from pathlib import Path

root = Path('.')

# batching_equivalence: token_program was accidentally removed with SystemProgram cleanup.
p = root / 'integration-tests/tests/batching_equivalence.rs'
t = p.read_text()
old = '''                upline_7: technical_root, upline_8: technical_root,
            })
'''
new = '''                upline_7: technical_root, upline_8: technical_root,
                token_program: spl_token::id(),
            })
'''
if t.count(old) != 1:
    raise SystemExit(f'batching_equivalence token_program insertion: expected 1, found {t.count(old)}')
p.write_text(t.replace(old, new, 1))

# grace_expiry: remove only the obsolete batch fields, preserve token_program.
p = root / 'integration-tests/tests/grace_expiry.rs'
t = p.read_text()
for old in [
    '            batch: sponsor_batch, token_program: spl_token::id(),\n',
    '            batch: buyer_batch, token_program: spl_token::id(),\n',
]:
    if t.count(old) != 1:
        raise SystemExit(f'grace_expiry obsolete batch field expected once: {old!r}, found {t.count(old)}')
    t = t.replace(old, '            token_program: spl_token::id(),\n', 1)
p.write_text(t)

# deep_network: only the activation purchase still carried the removed local batch.
p = root / 'integration-tests/tests/deep_network.rs'
t = p.read_text()
old = '''                upline_8: uplines[7],
                batch,
                token_program: spl_token::id(),
'''
new = '''                upline_8: uplines[7],
                token_program: spl_token::id(),
'''
if t.count(old) != 1:
    raise SystemExit(f'deep_network obsolete batch field expected once, found {t.count(old)}')
p.write_text(t.replace(old, new, 1))

print('fixed three rentless LiteSVM fixture compile errors')
