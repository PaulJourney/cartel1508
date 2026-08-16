from pathlib import Path

p = Path('programs/service_referral_protocol/src/lib.rs')
text = p.read_text()
old = '''    #[msg("Batch amount exceeds SPL token u64 capacity")]
    BatchTooLarge,
'''
new = '''    #[msg("Purchase amount exceeds SPL token u64 capacity")]
    PurchaseTooLarge,
'''
if text.count(old) != 1:
    raise SystemExit(f'expected exactly one legacy BatchTooLarge variant, found {text.count(old)}')
text = text.replace(old, new, 1)
p.write_text(text)
print('renamed BatchTooLarge -> PurchaseTooLarge')
