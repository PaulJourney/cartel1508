from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(rel: str) -> str:
    return (ROOT / rel).read_text()


def write(rel: str, text: str) -> None:
    (ROOT / rel).write_text(text)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


# --- Core instruction -------------------------------------------------------
lib_path = "programs/service_referral_protocol/src/lib.rs"
lib = read(lib_path)

lib = replace_once(
    lib,
    """    /// - credits eligible L1 direct sponsor with 50%;\n    /// - credits eligible genealogical L2-L10 with the frozen 43% schedule;\n""",
    """    /// - credits the buyer/self economic level with 50%;\n    /// - credits the immutable sponsor plus eight ancestors with the frozen 43% schedule;\n""",
    "purchase economics documentation",
)

start_marker = """        let (direct, levels, pioneer, service, rounding_remainder) =\n            split_purchase_amount(payment)?;\n"""
end_marker = "        let treasury_now = service\n"
start = lib.find(start_marker)
if start < 0:
    raise SystemExit("purchase allocation start marker not found")
end = lib.find(end_marker, start)
if end < 0:
    raise SystemExit("purchase allocation end marker not found")

new_allocation = """        let (self_reward, levels, pioneer, service, rounding_remainder) =\n            split_purchase_amount(payment)?;\n        let p = &mut ctx.accounts.protocol;\n        let mut unallocated: u64 = 0;\n        let mut expired_flow: u64 = 0;\n\n        // Pioneer index is advanced before activity settlement so an INACTIVE buyer\n        // or upline cannot leave their newly-created Pioneer entitlement stranded.\n        let pioneer_unassigned = accrue_pioneer(p, mint, pioneer)?;\n\n        // Economic L0/L1 SELF: the buyer always receives the 50% bucket. The buyer's\n        // activity has already been updated by this purchase. ACTIVE can claim; GRACE\n        // preserves the reward until requalification; INACTIVE is IC-A treasury expiry.\n        add_self_reward(&mut ctx.accounts.user, p, mint, self_reward)?;\n        let buyer_expired = expire_unclaimed_for_mint(&mut ctx.accounts.user, now, p, mint)?;\n        expired_flow = expired_flow\n            .checked_add(buyer_expired)\n            .ok_or(ProtocolError::ArithmeticOverflow)?;\n\n        // The 43% network pool contains exactly nine uplines. The immutable direct\n        // referrer/sponsor is network slot 1 (15%), followed by eight ancestors.\n        if ctx.accounts.direct_referrer.is_technical_root() {\n            for level in levels {\n                unallocated = unallocated\n                    .checked_add(level)\n                    .ok_or(ProtocolError::ArithmeticOverflow)?;\n            }\n        } else {\n            let prior_expired = expire_unclaimed_for_mint(\n                &mut ctx.accounts.direct_referrer,\n                now,\n                p,\n                mint,\n            )?;\n            expired_flow = expired_flow\n                .checked_add(prior_expired)\n                .ok_or(ProtocolError::ArithmeticOverflow)?;\n\n            match activity_status(&ctx.accounts.direct_referrer, now) {\n                ActivityStatus::Active => {\n                    add_network_claimable(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?\n                }\n                ActivityStatus::Grace => {\n                    add_network_pending(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?\n                }\n                ActivityStatus::Inactive => {\n                    mark_user_expired(&mut ctx.accounts.direct_referrer, p, mint, levels[0])?;\n                    expired_flow = expired_flow\n                        .checked_add(levels[0])\n                        .ok_or(ProtocolError::ArithmeticOverflow)?;\n                }\n            }\n\n            let uplines = [\n                ctx.accounts.upline_1.as_ref(),\n                ctx.accounts.upline_2.as_ref(),\n                ctx.accounts.upline_3.as_ref(),\n                ctx.accounts.upline_4.as_ref(),\n                ctx.accounts.upline_5.as_ref(),\n                ctx.accounts.upline_6.as_ref(),\n                ctx.accounts.upline_7.as_ref(),\n                ctx.accounts.upline_8.as_ref(),\n            ];\n            let mut expected_wallet = ctx.accounts.direct_referrer.referrer;\n\n            // levels[0] belongs to the sponsor; levels[1]..levels[8] belong to the\n            // next eight ancestors. No tenth upline account can receive value.\n            for i in 1..9 {\n                let ai = &uplines[i - 1];\n                let expected_key = Pubkey::find_program_address(\n                    &[b\"user\", expected_wallet.as_ref()],\n                    &crate::ID,\n                )\n                .0;\n                require!(ai.key() == expected_key, ProtocolError::InvalidUpline);\n                let mut upline: Account<UserState> = Account::try_from(ai)?;\n                require!(\n                    upline.wallet == expected_wallet,\n                    ProtocolError::InvalidUpline\n                );\n\n                if upline.is_technical_root() {\n                    for remaining in levels.iter().skip(i) {\n                        unallocated = unallocated\n                            .checked_add(*remaining)\n                            .ok_or(ProtocolError::ArithmeticOverflow)?;\n                    }\n                    break;\n                }\n\n                let previously_expired = expire_unclaimed_for_mint(&mut upline, now, p, mint)?;\n                expired_flow = expired_flow\n                    .checked_add(previously_expired)\n                    .ok_or(ProtocolError::ArithmeticOverflow)?;\n\n                match activity_status(&upline, now) {\n                    ActivityStatus::Active => {\n                        add_network_claimable(&mut upline, p, mint, levels[i])?\n                    }\n                    ActivityStatus::Grace => {\n                        add_network_pending(&mut upline, p, mint, levels[i])?\n                    }\n                    ActivityStatus::Inactive => {\n                        mark_user_expired(&mut upline, p, mint, levels[i])?;\n                        expired_flow = expired_flow\n                            .checked_add(levels[i])\n                            .ok_or(ProtocolError::ArithmeticOverflow)?;\n                    }\n                }\n                expected_wallet = upline.referrer;\n                upline.exit(&crate::ID)?;\n            }\n        }\n\n"""
lib = lib[:start] + new_allocation + lib[end:]

accounts_start = lib.find("    /// Direct sponsor = genealogical L1.\n")
accounts_end = lib.find("    #[account(\n        init,\n        payer = wallet,", accounts_start)
if accounts_start < 0 or accounts_end < 0:
    raise SystemExit("purchase account genealogy block not found")
new_accounts = """    /// Immutable direct sponsor = first network upline (15%).\n    #[account(mut)]\n    pub direct_referrer: Box<Account<'info, UserState>>,\n    /// CHECK: second network upline (9%); verified dynamically against ancestry.\n    #[account(mut)]\n    pub upline_1: UncheckedAccount<'info>,\n    /// CHECK: third network upline (6%).\n    #[account(mut)]\n    pub upline_2: UncheckedAccount<'info>,\n    /// CHECK: fourth network upline (4%).\n    #[account(mut)]\n    pub upline_3: UncheckedAccount<'info>,\n    /// CHECK: fifth network upline (2.5%).\n    #[account(mut)]\n    pub upline_4: UncheckedAccount<'info>,\n    /// CHECK: sixth network upline (2%).\n    #[account(mut)]\n    pub upline_5: UncheckedAccount<'info>,\n    /// CHECK: seventh network upline (1.5%).\n    #[account(mut)]\n    pub upline_6: UncheckedAccount<'info>,\n    /// CHECK: eighth network upline (1%).\n    #[account(mut)]\n    pub upline_7: UncheckedAccount<'info>,\n    /// CHECK: ninth network upline (2%).\n    #[account(mut)]\n    pub upline_8: UncheckedAccount<'info>,\n"""
lib = lib[:accounts_start] + new_accounts + lib[accounts_end:]

# The former "direct" accumulator is now explicitly the buyer's SELF reward bucket.
lib = lib.replace("add_direct", "add_self_reward")
lib = lib.replace("direct_accrued_usdt", "self_accrued_usdt")
lib = lib.replace("direct_accrued_usdc", "self_accrued_usdc")
write(lib_path, lib)

# --- State / constants / math ----------------------------------------------
state_path = "programs/service_referral_protocol/src/state.rs"
state = read(state_path)
state = state.replace("direct_accrued_usdt", "self_accrued_usdt")
state = state.replace("direct_accrued_usdc", "self_accrued_usdc")
write(state_path, state)

constants_path = "programs/service_referral_protocol/src/constants.rs"
constants = read(constants_path)
constants = constants.replace("DIRECT_BPS", "SELF_BPS")
constants = replace_once(
    constants,
    """// Frozen production genealogy:\n// L1 = direct sponsor and receives SELF_BPS only.\n// The 43% network pool is distributed over genealogical L2-L10.\n""",
    """// Frozen production economics:\n// SELF = buyer and receives SELF_BPS.\n// The 43% network pool is distributed over exactly nine uplines, starting with sponsor.\n""",
    "constants genealogy comment",
)
write(constants_path, constants)

math_path = "programs/service_referral_protocol/src/math.rs"
math = read(math_path)
math = math.replace("DIRECT_BPS", "SELF_BPS")
math = math.replace(
    """/// L1 is the direct sponsor and receives SELF_BPS only. `levels[0]` therefore\n/// corresponds to genealogical L2 and `levels[8]` to genealogical L10.\n""",
    """/// The buyer receives SELF_BPS. `levels[0]` is the immutable sponsor and\n/// `levels[8]` is the ninth network upline.\n""",
)
math = math.replace("let direct = mul_bps(amount, SELF_BPS)?;", "let self_reward = mul_bps(amount, SELF_BPS)?;")
math = math.replace("let allocated = direct\n", "let allocated = self_reward\n")
math = math.replace("Ok((direct, levels, pioneer, service, rounding_remainder))", "Ok((self_reward, levels, pioneer, service, rounding_remainder))")
math = math.replace("let (direct, levels, pioneer, service, remainder) =", "let (self_reward, levels, pioneer, service, remainder) =")
math = math.replace("assert_eq!(direct, 500_000);", "assert_eq!(self_reward, 500_000);")
math = math.replace("let total = (direct as u128)", "let total = (self_reward as u128)")
math = math.replace("assert!(direct <= amount);", "assert!(self_reward <= amount);")
write(math_path, math)

# --- Remove obsolete tenth network account from Rust clients/tests ----------
for path in (ROOT / "integration-tests").rglob("*.rs"):
    text = path.read_text()
    lines = [line for line in text.splitlines(True) if "upline_9:" not in line]
    text = "".join(lines)
    text = text.replace("direct_accrued_usdt", "self_accrued_usdt")
    text = text.replace("direct_accrued_usdc", "self_accrued_usdc")
    path.write_text(text)

smoke_path = ROOT / "scripts/devnet-transaction-smoke.mjs"
if smoke_path.exists():
    smoke = smoke_path.read_text()
    smoke = "".join(line for line in smoke.splitlines(True) if "upline9:" not in line)
    smoke = smoke.replace("directAccruedUsdt", "selfAccruedUsdt")
    smoke = smoke.replace("directAccruedUsdc", "selfAccruedUsdc")
    smoke = smoke.replace("direct_accrued_usdt", "self_accrued_usdt")
    smoke = smoke.replace("direct_accrued_usdc", "self_accrued_usdc")
    smoke_path.write_text(smoke)

# --- Reference model terminology -------------------------------------------
ref_path = "tests/reference-model.mjs"
ref = read(ref_path)
ref = ref.replace(
    "// Production genealogy: L1 is the direct sponsor (50% only).\n// The 43% network pool therefore spans L2-L10.\n",
    "// Production economics: buyer/SELF receives 50%; the 43% network pool spans\n// exactly nine uplines beginning with the immutable sponsor.\n",
)
ref = ref.replace("const direct = calc(5000);", "const selfReward = calc(5000);")
ref = ref.replace("const allocated = direct + pioneer + service", "const allocated = selfReward + pioneer + service")
ref = ref.replace("return { direct, levels, pioneer, service, remainder: amount - allocated };", "return { selfReward, levels, pioneer, service, remainder: amount - allocated };")
ref = ref.replace("  direct: 500_000,", "  selfReward: 500_000,")
ref = ref.replace("s.direct + s.pioneer", "s.selfReward + s.pioneer")
ref = ref.replace("V0.11 purchase-triggered invariants passed", "V0.13 SELF-plus-nine-uplines invariants passed")
write(ref_path, ref)

# --- Static gates -----------------------------------------------------------
gates_path = "scripts/static-gates.py"
gates = read(gates_path)
gates = gates.replace("DIRECT_BPS", "SELF_BPS")
gates = gates.replace("direct_accrued_usdt", "self_accrued_usdt")
gates = gates.replace("direct_accrued_usdc", "self_accrued_usdc")
gates = gates.replace("fn add_direct", "fn add_self_reward")
gates = gates.replace("'add_direct('", "'add_self_reward('")
gates = gates.replace(
    "    'sponsor is direct level one only': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'add_self_reward(&mut ctx.accounts.direct_referrer' in final_purchase,\n    'network starts at genealogical level two': 'let mut expected_wallet = ctx.accounts.direct_referrer.referrer' in final_purchase,\n    'production network stops at level ten': 'for i in 0..9' in final_purchase and all(f'upline_{i}' in production_accounts for i in range(1, 10)) and 'upline_10' not in production_accounts,\n",
    "    'buyer receives the 50 percent SELF bucket': 'add_self_reward(&mut ctx.accounts.user' in final_purchase and 'add_self_reward(&mut ctx.accounts.direct_referrer' not in final_purchase,\n    'network starts at immutable sponsor': 'direct_referrer.wallet == ctx.accounts.user.referrer' in final_purchase and 'levels[0]' in final_purchase and 'add_network_claimable(&mut ctx.accounts.direct_referrer' in final_purchase,\n    'network contains exactly nine uplines': 'for i in 1..9' in final_purchase and all(f'upline_{i}' in production_accounts for i in range(1, 9)) and 'upline_9' not in production_accounts,\n",
)
write(gates_path, gates)

print("SELF + nine-upline migration applied")
