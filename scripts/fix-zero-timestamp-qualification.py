from pathlib import Path

p = Path('programs/service_referral_protocol/src/lib.rs')
text = p.read_text()
old = '''    if user.qualification_progress_units > 0
        && user.qualification_window_started_at > 0
        && now
            > user
                .qualification_window_started_at
                .checked_add(ACTIVE_SECONDS)
                .ok_or(ProtocolError::ArithmeticOverflow)?
'''
new = '''    if user.qualification_progress_units > 0
        && now
            > user
                .qualification_window_started_at
                .checked_add(ACTIVE_SECONDS)
                .ok_or(ProtocolError::ArithmeticOverflow)?
'''
if text.count(old) != 1:
    raise SystemExit(f'activity-window sentinel block expected once, found {text.count(old)}')
text = text.replace(old, new, 1)
old = '''    if user.qualification_progress_units == 0 || user.qualification_window_started_at <= 0 {
        return false;
    }
'''
new = '''    if user.qualification_progress_units == 0 {
        return false;
    }
'''
if text.count(old) != 1:
    raise SystemExit(f'qualification_window_open sentinel expected once, found {text.count(old)}')
text = text.replace(old, new, 1)
p.write_text(text)
print('qualification window now uses progress, not timestamp sentinel')
