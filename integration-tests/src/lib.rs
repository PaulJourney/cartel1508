#[cfg(test)]
mod tests {
    use anchor_lang::{prelude::*, AccountDeserialize};
    use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
    use service_referral_protocol::{
        state::{ProtocolState, UnitBatch, UserState},
        ID,
    };
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const PROGRAM_BYTES: &[u8] =
        include_bytes!("../../target/deploy/service_referral_protocol.so");
    const UNIT: u64 = 1_000_000;

    #[test]
    fn initialize_creates_canonical_protocol_and_root_pdas() {
        let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
        let initializer = ctx
            .svm
            .create_funded_account(10_000_000_000)
            .expect("fund initializer");

        let service_treasury = Pubkey::new_unique();
        let usdt_mint = Pubkey::new_unique();
        let usdc_mint = Pubkey::new_unique();
        let qualified_revenue_source = Pubkey::new_unique();

        let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
        let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
        let (technical_root, _) =
            Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);

        let clock: Clock = ctx.svm.get_sysvar();
        let registration_open_at = clock.unix_timestamp + 60;

        let ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Initialize {
                initializer: initializer.pubkey(),
                service_treasury,
                usdt_mint,
                usdc_mint,
                protocol: protocol_pda,
                vault_authority,
                technical_root,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Initialize {
                registration_open_at,
                qualified_revenue_source,
            })
            .instruction()
            .expect("build initialize instruction");

        ctx.execute_instruction(ix, &[&initializer])
            .expect("execute initialize")
            .assert_success();

        let protocol_account = ctx
            .svm
            .get_account(&protocol_pda)
            .expect("protocol account exists");
        let mut protocol_data = protocol_account.data.as_slice();
        let protocol = ProtocolState::try_deserialize(&mut protocol_data)
            .expect("deserialize protocol state");

        assert_eq!(protocol.service_treasury, service_treasury);
        assert_eq!(protocol.qualified_revenue_source, qualified_revenue_source);
        assert_eq!(protocol.usdt_mint, usdt_mint);
        assert_eq!(protocol.usdc_mint, usdc_mint);
        assert_eq!(protocol.registration_open_at, registration_open_at);
        assert_eq!(protocol.real_user_count, 0);
        assert_eq!(protocol.pioneer_count, 0);

        let root_account = ctx
            .svm
            .get_account(&technical_root)
            .expect("technical root exists");
        let mut root_data = root_account.data.as_slice();
        let root = UserState::try_deserialize(&mut root_data)
            .expect("deserialize technical root");

        assert!(root.is_technical_root());
        assert_eq!(root.wallet, Pubkey::default());
        assert_eq!(root.referrer, Pubkey::default());
        assert_eq!(root.pioneer_id, 0);
    }

    #[test]
    fn registration_is_time_gated_and_first_real_user_is_pioneer_one() {
        let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
        let initializer = ctx
            .svm
            .create_funded_account(10_000_000_000)
            .expect("fund initializer");
        let user = ctx
            .svm
            .create_funded_account(10_000_000_000)
            .expect("fund user");

        let service_treasury = Pubkey::new_unique();
        let usdt_mint = Pubkey::new_unique();
        let usdc_mint = Pubkey::new_unique();
        let qualified_revenue_source = Pubkey::new_unique();

        let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
        let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
        let (technical_root, _) =
            Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
        let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &ID);

        let mut clock: Clock = ctx.svm.get_sysvar();
        let registration_open_at = clock.unix_timestamp + 60;

        let initialize_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Initialize {
                initializer: initializer.pubkey(),
                service_treasury,
                usdt_mint,
                usdc_mint,
                protocol: protocol_pda,
                vault_authority,
                technical_root,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Initialize {
                registration_open_at,
                qualified_revenue_source,
            })
            .instruction()
            .expect("build initialize instruction");
        ctx.execute_instruction(initialize_ix, &[&initializer])
            .expect("execute initialize")
            .assert_success();

        let pre_open_register_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: user.pubkey(),
                protocol: protocol_pda,
                referrer_wallet: Pubkey::default(),
                referrer: technical_root,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction()
            .expect("build pre-open register instruction");

        let before_open = ctx
            .execute_instruction(pre_open_register_ix, &[&user])
            .expect("execute pre-open register transaction");
        assert!(!before_open.is_success(), "registration must fail before open time");
        assert!(ctx.svm.get_account(&user_pda).is_none());

        clock.unix_timestamp = registration_open_at;
        ctx.svm.set_sysvar(&clock);

        let open_register_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: user.pubkey(),
                protocol: protocol_pda,
                referrer_wallet: Pubkey::default(),
                referrer: technical_root,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction()
            .expect("build open register instruction");

        ctx.execute_instruction(open_register_ix, &[&user])
            .expect("execute open register transaction")
            .assert_success();

        let user_account = ctx.svm.get_account(&user_pda).expect("user PDA exists");
        let mut user_data = user_account.data.as_slice();
        let user_state = UserState::try_deserialize(&mut user_data).expect("deserialize user");
        assert_eq!(user_state.wallet, user.pubkey());
        assert_eq!(user_state.referrer, Pubkey::default());
        assert_eq!(user_state.pioneer_id, 1);

        let protocol_account = ctx.svm.get_account(&protocol_pda).expect("protocol exists");
        let mut protocol_data = protocol_account.data.as_slice();
        let protocol = ProtocolState::try_deserialize(&mut protocol_data).expect("deserialize protocol");
        assert_eq!(protocol.real_user_count, 1);
        assert_eq!(protocol.pioneer_count, 1);
    }

    #[test]
    fn purchase_ten_units_transfers_stablecoin_and_activates_without_creating_rewards() {
        let mut ctx = AnchorLiteSVM::build_with_program(ID, PROGRAM_BYTES);
        let initializer = ctx.svm.create_funded_account(20_000_000_000).expect("initializer");
        let treasury = ctx.svm.create_funded_account(10_000_000_000).expect("treasury");
        let user = ctx.svm.create_funded_account(10_000_000_000).expect("user");
        let revenue_source = ctx.svm.create_funded_account(10_000_000_000).expect("revenue source");

        let usdt_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDT mint");
        let usdc_mint = ctx.svm.create_token_mint(&initializer, 6).expect("USDC mint");

        let (protocol_pda, _) = Pubkey::find_program_address(&[b"protocol"], &ID);
        let (vault_authority, _) = Pubkey::find_program_address(&[b"vault-authority"], &ID);
        let (technical_root, _) = Pubkey::find_program_address(&[b"user", Pubkey::default().as_ref()], &ID);
        let (user_pda, _) = Pubkey::find_program_address(&[b"user", user.pubkey().as_ref()], &ID);
        let (batch_pda, _) = Pubkey::find_program_address(
            &[b"batch", user.pubkey().as_ref(), &0u64.to_le_bytes()],
            &ID,
        );

        let registration_open_at = ctx.svm.get_unix_timestamp();
        let initialize_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Initialize {
                initializer: initializer.pubkey(),
                service_treasury: treasury.pubkey(),
                usdt_mint: usdt_mint.pubkey(),
                usdc_mint: usdc_mint.pubkey(),
                protocol: protocol_pda,
                vault_authority,
                technical_root,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Initialize {
                registration_open_at,
                qualified_revenue_source: revenue_source.pubkey(),
            })
            .instruction()
            .expect("initialize ix");
        ctx.execute_instruction(initialize_ix, &[&initializer])
            .expect("initialize tx")
            .assert_success();

        let register_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::Register {
                wallet: user.pubkey(),
                protocol: protocol_pda,
                referrer_wallet: Pubkey::default(),
                referrer: technical_root,
                user: user_pda,
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::Register {})
            .instruction()
            .expect("register ix");
        ctx.execute_instruction(register_ix, &[&user])
            .expect("register tx")
            .assert_success();

        let treasury_usdt = ctx
            .svm
            .create_associated_token_account(&usdt_mint.pubkey(), &treasury)
            .expect("treasury USDT ATA");
        let treasury_usdc = ctx
            .svm
            .create_associated_token_account(&usdc_mint.pubkey(), &treasury)
            .expect("treasury USDC ATA");
        let user_usdt = ctx
            .svm
            .create_associated_token_account(&usdt_mint.pubkey(), &user)
            .expect("user USDT ATA");

        let vault_usdt = spl_associated_token_account::get_associated_token_address_with_program_id(
            &vault_authority,
            &usdt_mint.pubkey(),
            &spl_token::id(),
        );
        let vault_usdc = spl_associated_token_account::get_associated_token_address_with_program_id(
            &vault_authority,
            &usdc_mint.pubkey(),
            &spl_token::id(),
        );
        let create_vault_usdt = spl_associated_token_account::instruction::create_associated_token_account(
            &initializer.pubkey(),
            &vault_authority,
            &usdt_mint.pubkey(),
            &spl_token::id(),
        );
        let create_vault_usdc = spl_associated_token_account::instruction::create_associated_token_account(
            &initializer.pubkey(),
            &vault_authority,
            &usdc_mint.pubkey(),
            &spl_token::id(),
        );
        let create_vaults_tx = Transaction::new_signed_with_payer(
            &[create_vault_usdt, create_vault_usdc],
            Some(&initializer.pubkey()),
            &[&initializer],
            ctx.svm.latest_blockhash(),
        );
        ctx.svm.send_transaction(create_vaults_tx).expect("create vault ATAs");

        ctx.svm
            .mint_to(&usdt_mint.pubkey(), &user_usdt, &initializer, 20 * UNIT)
            .expect("mint user USDT");

        let purchase_ix = ctx
            .program()
            .accounts(service_referral_protocol::accounts::PurchaseServiceUnits {
                wallet: user.pubkey(),
                protocol: protocol_pda,
                user: user_pda,
                user_source: user_usdt,
                vault_authority,
                usdt_vault: vault_usdt,
                usdc_vault: vault_usdc,
                service_treasury_usdt: treasury_usdt,
                service_treasury_usdc: treasury_usdc,
                batch: batch_pda,
                token_program: spl_token::id(),
                system_program: anchor_lang::system_program::ID,
            })
            .args(service_referral_protocol::instruction::PurchaseServiceUnits { units: 10 })
            .instruction()
            .expect("purchase ix");
        ctx.execute_instruction(purchase_ix, &[&user])
            .expect("purchase tx")
            .assert_success();

        ctx.svm.assert_token_balance(&user_usdt, 10 * UNIT);
        ctx.svm.assert_token_balance(&treasury_usdt, 10 * UNIT);
        ctx.svm.assert_token_balance(&treasury_usdc, 0);
        ctx.svm.assert_token_balance(&vault_usdt, 0);
        ctx.svm.assert_token_balance(&vault_usdc, 0);

        let user_account = ctx.svm.get_account(&user_pda).expect("user state");
        let mut user_data = user_account.data.as_slice();
        let user_state = UserState::try_deserialize(&mut user_data).expect("deserialize user state");
        assert_eq!(user_state.lifetime_service_units, 10);
        assert_eq!(user_state.next_batch_index, 1);
        assert_eq!(user_state.qualification_progress_units, 0);
        assert_eq!(user_state.qualification_window_started_at, 0);
        assert!(user_state.active_until >= registration_open_at + 7 * 24 * 60 * 60);
        assert!(user_state.grace_until > user_state.active_until);
        assert_eq!(user_state.direct_accrued_usdt, 0);
        assert_eq!(user_state.network_claimable_usdt, 0);
        assert_eq!(user_state.network_pending_usdt, 0);

        let batch_account = ctx.svm.get_account(&batch_pda).expect("batch state");
        let mut batch_data = batch_account.data.as_slice();
        let batch = UnitBatch::try_deserialize(&mut batch_data).expect("deserialize batch");
        assert_eq!(batch.owner, user.pubkey());
        assert_eq!(batch.batch_index, 0);
        assert_eq!(batch.mint, usdt_mint.pubkey());
        assert_eq!(batch.units, 10);
        assert_eq!(batch.first_local_unit_index, 1);
        assert_eq!(batch.last_local_unit_index, 10);
    }
}
