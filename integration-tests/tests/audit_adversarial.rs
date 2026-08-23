//! Independent-audit adversarial scenarios (audit-only harness file).
//!
//! Authored during the external audit of frozen SHA
//! 36420887aad96b3c5b9680a35c6dc211b893c2bc. These pin down behavior the
//! project suite does not assert directly:
//!
//! 1. A qualification window opened during GRACE preserves the user's whole
//!    pre-existing SELF bucket across an INACTIVE period (documented repo
//!    behavior; narrower wording in the audit dossier §6.4).
//! 2. Exact inclusive boundary of the 7-day qualification window.
//! 3. network_pending vests into network_claimable on GRACE requalification.
//! 4. During a live INACTIVE partial window, SELF is preserved while the
//!    same purchase's Pioneer share expires to Treasury (exact atoms).
use anchor_lang::{prelude::*, AccountDeserialize};
use anchor_litesvm::{AnchorContext, AnchorLiteSVM, AssertionHelpers, TestHelpers};
use service_referral_protocol::{
    constants::PIONEER_SCALE,
    state::{ProtocolState, UserState},
    ID,
};
use solana_signer::Signer;
use solana_transaction::Transaction;

const PROGRAM_BYTES: &[u8] = include_bytes!("../../target/deploy/service_referral_protocol.so");
const UNIT: u64 = 1_000_000;
const WEEK: i64 = 7 * 24 * 60 * 60;

fn read_user(ctx: &AnchorContext, pda: Pubkey) -> UserState {
    let account = ctx.svm.get_account(&pda).expect("user state");
    let mut data = account.data.as_slice();
    UserState::try_deserialize(&mut data).expect("deserialize user")
}

fn read_protocol(ctx: &AnchorContext, pda: Pubkey) -> ProtocolState {
    let account = ctx.svm.get_account(&pda).expect("protocol state");
    let mut data = account.data.as_slice();
    ProtocolState::try_deserialize(&mut data).expect("deserialize protocol")
}

/// Shared boilerplate: initialize the protocol, create both vault ATAs and
/// treasury ATAs, and expose the common addresses as local bindings.
macro_rules! audit_setup {
    ($ctx:ident, $initializer:ident, $usdc_mint:ident, $protocol:ident,
     $vault_authority:ident, $root:ident, $vault_usdt:ident, $vault_usdc:ident,
     $treasury_usdt:ident, $treasury_usdc:ident) => {
        let mut $ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
        let $initializer = $ctx
            .svm
            .create_funded_account(90_000_000_000)
            .expect("initializer");
        let treasury = $ctx
            .svm
            .create_funded_account(10_000_000_000)
            .expect("treasury");
        let usdt_mint = $ctx.svm.create_token_mint(&$initializer, 6).expect("USDT");
        let $usdc_mint = $ctx.svm.create_token_mint(&$initializer, 6).expect("USDC");
        let ($protocol, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
        let ($vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
        let ($root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);

        let registration_open_at = $ctx.svm.get_sysvar::<Clock>().unix_timestamp;
        let init = $ctx
            .program()
            .accounts(service_referral_protocol::accounts::Initialize {
                initializer: $initializer.pubkey(),
                service_treasury: treasury.pubkey(),
                usdt_mint: usdt_mint.pubkey(),
                usdc_mint: $usdc_mint.pubkey(),
                protocol: $protocol,
                vault_authority: $vault_authority,
                technical_root: $root,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Initialize {
                registration_open_at,
            })
            .instruction()
            .expect("init ix");
        $ctx.execute_instruction(init, &[&$initializer])
            .expect("init tx")
            .assert_success();

        let $treasury_usdt = $ctx
            .svm
            .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
            .expect("treasury USDT");
        let $treasury_usdc = $ctx
            .svm
            .create_associated_token_account(&$usdc_mint.pubkey(), &treasury)
            .expect("treasury USDC");
        let $vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
            &$vault_authority,
            &usdt_mint.pubkey(),
            &spl_token::id(),
        );
        let $vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
            &$vault_authority,
            &$usdc_mint.pubkey(),
            &spl_token::id(),
        );
        let create_vaults = Transaction::new_signed_with_payer(
            &[
                spl_associated_token_account::instruction::create_associated_token_account(
                    &$initializer.pubkey(),
                    &$vault_authority,
                    &usdt_mint.pubkey(),
                    &spl_token::id(),
                ),
                spl_associated_token_account::instruction::create_associated_token_account(
                    &$initializer.pubkey(),
                    &$vault_authority,
                    &$usdc_mint.pubkey(),
                    &spl_token::id(),
                ),
            ],
            Some(&$initializer.pubkey()),
            &[&$initializer],
            $ctx.svm.latest_blockhash(),
        );
        $ctx.svm.send_transaction(create_vaults).expect("vaults");
    };
}

macro_rules! register_under {
    ($ctx:ident, $protocol:ident, $root:ident, $wallet:expr, $referrer_wallet:expr) => {{
        let referrer_wallet: Pubkey = $referrer_wallet;
        let referrer_pda = if referrer_wallet == Pubkey::default() {
            $root
        } else {
            Pubkey::find_program_address(&[b"user", referrer_wallet.as_ref()], &ID).0
        };
        let user_pda =
            Pubkey::find_program_address(&[b"user", $wallet.pubkey().as_ref()], &ID).0;
        $ctx.svm.expire_blockhash();
        let ix = $ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: $wallet.pubkey(),
                protocol: $protocol,
                referrer_wallet,
                referrer: referrer_pda,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction()
            .expect("register ix");
        $ctx.execute_instruction(ix, &[&$wallet])
            .expect("register tx")
            .assert_success();
        user_pda
    }};
}

macro_rules! buy {
    ($ctx:ident, $protocol:ident, $vault_authority:ident, $vault_usdt:ident,
     $vault_usdc:ident, $treasury_usdt:ident, $treasury_usdc:ident,
     $wallet:expr, $source:expr, $direct_referrer:expr, $uplines:expr, $units:expr) => {{
        let user_pda =
            Pubkey::find_program_address(&[b"user", $wallet.pubkey().as_ref()], &ID).0;
        let uplines: [Pubkey; 8] = $uplines;
        $ctx.svm.expire_blockhash();
        let ix = $ctx
            .program()
            .accounts(service_referral_protocol::accounts::PurchaseAndDistribute {
                wallet: $wallet.pubkey(),
                protocol: $protocol,
                user: user_pda,
                user_source: $source,
                vault_authority: $vault_authority,
                usdt_vault: $vault_usdt,
                usdc_vault: $vault_usdc,
                service_treasury_usdt: $treasury_usdt,
                service_treasury_usdc: $treasury_usdc,
                direct_referrer: $direct_referrer,
                upline_1: uplines[0],
                upline_2: uplines[1],
                upline_3: uplines[2],
                upline_4: uplines[3],
                upline_5: uplines[4],
                upline_6: uplines[5],
                upline_7: uplines[6],
                upline_8: uplines[7],
                token_program: spl_token::id(),
            })
            .args(service_referral_protocol::instruction::PurchaseAndDistribute {
                units: $units,
            })
            .instruction()
            .expect("purchase ix");
        $ctx.execute_instruction(ix, &[&$wallet])
            .expect("purchase tx")
            .assert_success();
    }};
}

macro_rules! warp_to {
    ($ctx:ident, $ts:expr) => {{
        let mut clock: Clock = $ctx.svm.get_sysvar();
        clock.unix_timestamp = $ts;
        $ctx.svm.set_sysvar(&clock);
        $ctx.svm.expire_blockhash();
    }};
}

macro_rules! claim_usdc {
    ($ctx:ident, $protocol:ident, $vault_authority:ident, $vault_usdc:ident,
     $wallet:expr, $destination:expr) => {{
        let user_pda =
            Pubkey::find_program_address(&[b"user", $wallet.pubkey().as_ref()], &ID).0;
        $ctx.svm.expire_blockhash();
        let ix = $ctx
            .program()
            .accounts(service_referral_protocol::accounts::Claim {
                wallet: $wallet.pubkey(),
                protocol: $protocol,
                user: user_pda,
                vault_authority: $vault_authority,
                vault_token: $vault_usdc,
                destination: $destination,
                token_program: spl_token::id(),
            })
            .args(service_referral_protocol::instruction::Claim {})
            .instruction()
            .expect("claim ix");
        $ctx.execute_instruction(ix, &[&$wallet])
            .expect("claim tx")
            .assert_success();
    }};
}

/// Audit scenario 1: SELF accrued during ACTIVE weeks survives an INACTIVE
/// period when a qualification window opened during GRACE is still live, and
/// becomes claimable again after requalification. This matches SECURITY.md /
/// AUDIT_HANDOFF.md ("SELF only is provisional") but is broader than the
/// dossier §6.4 wording ("SELF accrued during that live window").
#[test]
fn grace_opened_window_preserves_prior_active_self_through_inactive() {
    audit_setup!(ctx, initializer, usdc_mint, protocol, vault_authority, root,
        vault_usdt, vault_usdc, treasury_usdt, treasury_usdc);
    let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
    let user_pda = register_under!(ctx, protocol, root, user, Pubkey::default());
    let src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &user)
        .expect("user ATA");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &src, &initializer, 20 * UNIT)
        .expect("fund user");

    // Week 1: 10 units -> ACTIVE, SELF 5.0 accrued and deliberately unclaimed.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 10);
    let week1 = read_user(&ctx, user_pda);
    assert_eq!(week1.self_accrued_usdc, 5 * UNIT);
    assert_eq!(week1.active_weeks_started, 1);

    // Enter GRACE and open a qualification window with 1 unit.
    warp_to!(ctx, week1.active_until + 1);
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 1);
    let in_grace = read_user(&ctx, user_pda);
    assert_eq!(in_grace.qualification_progress_units, 1);
    assert_eq!(in_grace.self_accrued_usdc, 5 * UNIT + UNIT / 2);
    assert_eq!(in_grace.active_weeks_started, 1, "1 unit must not requalify");

    // Become INACTIVE while the 7-day window is still live.
    warp_to!(ctx, week1.grace_until + 1);

    // Complete the week-2 requirement (10) from INACTIVE. The pre-existing
    // ACTIVE-era SELF must be preserved, not expired.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 9);
    let reactivated = read_user(&ctx, user_pda);
    assert_eq!(reactivated.active_weeks_started, 2);
    assert_eq!(reactivated.current_week_units, 10);
    assert_eq!(
        reactivated.self_accrued_usdc,
        10 * UNIT,
        "5.0 ACTIVE-era + 0.5 GRACE + 4.5 reactivation SELF must all survive"
    );
    assert_eq!(
        reactivated.lifetime_expired_usdc, 0,
        "nothing may expire while the GRACE-opened window is live"
    );

    // Full claim including the pre-INACTIVE SELF.
    claim_usdc!(ctx, protocol, vault_authority, vault_usdc, user, src);
    let claimed = read_user(&ctx, user_pda);
    assert_eq!(claimed.lifetime_claimed_usdc, (10 * UNIT) as u128);
    ctx.svm.assert_token_balance(&src, 10 * UNIT);
    ctx.svm.assert_token_balance(&vault_usdc, 0);
    ctx.svm.assert_token_balance(&treasury_usdc, 10 * UNIT);
}

/// Audit scenario 2: the 7-day qualification window deadline is inclusive.
/// At exactly window_start + 7d progress and SELF survive; one second later
/// stale progress is discarded and unprotected SELF expires to Treasury.
#[test]
fn qualification_window_boundary_is_inclusive_and_stale_self_expires_after() {
    audit_setup!(ctx, initializer, usdc_mint, protocol, vault_authority, root,
        vault_usdt, vault_usdc, treasury_usdt, treasury_usdc);
    let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
    let user_pda = register_under!(ctx, protocol, root, user, Pubkey::default());
    let src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &user)
        .expect("user ATA");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &src, &initializer, 10 * UNIT)
        .expect("fund user");

    // LiteSVM genesis clock is 0; move to a realistic epoch so the window
    // timestamp is observable (existence is tracked via progress > 0 anyway).
    warp_to!(ctx, 1_800_000_000);

    // Registered but never ACTIVE: user is INACTIVE from the start.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 1);
    let opened = read_user(&ctx, user_pda);
    assert_eq!(opened.qualification_progress_units, 1);
    let window_start = opened.qualification_window_started_at;
    assert!(window_start > 0);
    assert_eq!(opened.self_accrued_usdc, UNIT / 2);

    // Exactly at the deadline: window still live, progress kept, SELF kept.
    warp_to!(ctx, window_start + WEEK);
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 1);
    let at_deadline = read_user(&ctx, user_pda);
    assert_eq!(
        at_deadline.qualification_progress_units, 2,
        "progress must survive at exactly +7d"
    );
    assert_eq!(
        at_deadline.qualification_window_started_at, window_start,
        "window must not restart at exactly +7d"
    );
    assert_eq!(at_deadline.self_accrued_usdc, UNIT);
    assert_eq!(at_deadline.lifetime_expired_usdc, 0);

    // One second past the deadline: stale progress discarded, a new window
    // starts, and the previously provisional SELF is expired to Treasury.
    warp_to!(ctx, window_start + WEEK + 1);
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, user, src, root, [root; 8], 1);
    let past_deadline = read_user(&ctx, user_pda);
    assert_eq!(
        past_deadline.qualification_progress_units, 1,
        "stale progress must reset to this purchase only"
    );
    assert!(past_deadline.qualification_window_started_at > window_start);
    assert_eq!(
        past_deadline.self_accrued_usdc,
        UNIT / 2,
        "only the fresh purchase SELF may remain provisional"
    );
    assert_eq!(
        past_deadline.lifetime_expired_usdc,
        UNIT as u128,
        "the stale-window SELF (2 x 0.5) must expire to Treasury"
    );
}

/// Audit scenario 3: network value earned while an upline is in GRACE is
/// pending, and vests into claimable when the upline requalifies during GRACE.
#[test]
fn grace_requalification_vests_pending_network_into_claimable() {
    audit_setup!(ctx, initializer, usdc_mint, protocol, vault_authority, root,
        vault_usdt, vault_usdc, treasury_usdt, treasury_usdc);
    let sponsor = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("sponsor");
    let buyer = ctx.svm.create_funded_account(10_000_000_000).expect("buyer");
    let sponsor_pda = register_under!(ctx, protocol, root, sponsor, Pubkey::default());
    let _buyer_pda = register_under!(ctx, protocol, root, buyer, sponsor.pubkey());
    let sponsor_src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &sponsor)
        .expect("sponsor ATA");
    let buyer_src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &buyer)
        .expect("buyer ATA");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &sponsor_src, &initializer, 20 * UNIT)
        .expect("fund sponsor");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &buyer_src, &initializer, 100 * UNIT)
        .expect("fund buyer");

    // Sponsor week 1.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, sponsor, sponsor_src, root, [root; 8], 10);
    let sponsor_week1 = read_user(&ctx, sponsor_pda);

    // Sponsor slips into GRACE; downline buys 100 -> U1 15% becomes pending.
    warp_to!(ctx, sponsor_week1.active_until + 1);
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, buyer, buyer_src, sponsor_pda, [root; 8], 100);
    let sponsor_grace = read_user(&ctx, sponsor_pda);
    assert_eq!(sponsor_grace.network_pending_usdc, 15 * UNIT);
    assert_eq!(sponsor_grace.network_claimable_usdc, 0);

    // Sponsor requalifies during GRACE: pending must vest into claimable.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, sponsor, sponsor_src, root, [root; 8], 10);
    let sponsor_week2 = read_user(&ctx, sponsor_pda);
    assert_eq!(sponsor_week2.active_weeks_started, 2);
    assert_eq!(sponsor_week2.network_pending_usdc, 0, "pending must vest");
    assert_eq!(sponsor_week2.network_claimable_usdc, 15 * UNIT);
    assert_eq!(sponsor_week2.self_accrued_usdc, 10 * UNIT);

    // Everything (10 SELF + 15 vested network) is claimable in one pull.
    claim_usdc!(ctx, protocol, vault_authority, vault_usdc, sponsor, sponsor_src);
    ctx.svm.assert_token_balance(&sponsor_src, 25 * UNIT);
    let after_claim = read_user(&ctx, sponsor_pda);
    assert_eq!(after_claim.lifetime_claimed_usdc, (25 * UNIT) as u128);
    // Vault retains exactly the buyer's unclaimed SELF liability.
    ctx.svm.assert_token_balance(&vault_usdc, 50 * UNIT);
}

/// Audit scenario 4: during a live INACTIVE partial window, the buyer's SELF
/// is preserved while the very same purchase's Pioneer share for the buyer's
/// own position expires to Treasury (exact atom accounting, including the
/// scaled fractional remainder path).
#[test]
fn inactive_partial_window_preserves_self_but_expires_own_pioneer_share() {
    audit_setup!(ctx, initializer, usdc_mint, protocol, vault_authority, root,
        vault_usdt, vault_usdc, treasury_usdt, treasury_usdc);
    let pioneer = ctx
        .svm
        .create_funded_account(10_000_000_000)
        .expect("pioneer");
    let pioneer_pda = register_under!(ctx, protocol, root, pioneer, Pubkey::default());
    let src = ctx
        .svm
        .create_associated_token_account(&usdc_mint.pubkey(), &pioneer)
        .expect("pioneer ATA");
    ctx.svm
        .mint_to(&usdc_mint.pubkey(), &src, &initializer, 1_001 * UNIT)
        .expect("fund pioneer");

    // One 1,000-unit purchase: ACTIVE week 1 + exactly one Pioneer position.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, pioneer, src, root, [root; 8], 1_000);
    let created = read_user(&ctx, pioneer_pda);
    assert_eq!(created.pioneer_positions, 1);

    // Claim the 500 SELF so later deltas are isolated.
    claim_usdc!(ctx, protocol, vault_authority, vault_usdc, pioneer, src);

    // Fall INACTIVE past GRACE.
    let created = read_user(&ctx, pioneer_pda);
    warp_to!(ctx, created.grace_until + 1);

    // A 1-unit INACTIVE purchase opens a live window. SELF (0.5) must be
    // preserved; the purchase's own Pioneer accrual for the buyer's position
    // (2% of 1 USDC / 100 = 200 atoms) must expire to Treasury immediately.
    buy!(ctx, protocol, vault_authority, vault_usdt, vault_usdc, treasury_usdt,
        treasury_usdc, pioneer, src, root, [root; 8], 1);
    let after = read_user(&ctx, pioneer_pda);
    let protocol_state = read_protocol(&ctx, protocol);
    assert_eq!(after.qualification_progress_units, 1);
    assert_eq!(after.self_accrued_usdc, UNIT / 2, "live-window SELF preserved");
    assert_eq!(
        after.lifetime_expired_usdc, 200u128,
        "own-purchase Pioneer share must expire, no INACTIVE exception"
    );
    // Checkpoint caught up to the full entitlement: nothing left due.
    let due = (protocol_state.pioneer_index_usdc * (after.pioneer_positions as u128)
        - after.pioneer_checkpoint_usdc)
        / PIONEER_SCALE;
    assert_eq!(due, 0);
    // Vault holds exactly the preserved SELF liability; everything else of the
    // 1 USDC purchase (0.43 unallocated + 0.05 service + 0.0198 pioneer
    // unassigned + 0.0002 expired pioneer) reached Treasury.
    ctx.svm.assert_token_balance(&vault_usdc, UNIT / 2);
    ctx.svm
        .assert_token_balance(&treasury_usdc, 500 * UNIT + UNIT / 2);
}
