from pathlib import Path
import runpy

source = Path("programs/service_referral_protocol/src/lib.rs").read_text()
if (
    "pub fn settle_expired(ctx: Context<SettleExpired>)" in source
    and "fn validate_supported_token(" not in source
    and "NonCanonicalTokenAccount" in source
):
    print("V0.9B migration already persisted")
else:
    runpy.run_path("scripts/migrate-v0-9b.py", run_name="__main__")
