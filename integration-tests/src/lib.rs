#[cfg(test)]
mod tests {
    use anchor_lang::{prelude::*, AccountDeserialize};
    use anchor_litesvm::{AnchorLiteSVM, AssertionHelpers, TestHelpers};
    use service_referral_protocol::{
        state::{ProtocolState, UserState},
        ID,
    };
    use solana_signer::Signer;

    const PROGRAM_BYTES: &[u8] =
        include_bytes!("../../target/deploy/service_referral_protocol.so");

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

        let register_ix = || {
            ctx.program()
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
                .expect("build register instruction")
        };

        let before_open = ctx
            .execute_instruction(register_ix(), &[&user])
            .expect("execute pre-open register transaction");
        assert!(!before_open.is_success(), "registration must fail before open time");
        assert!(ctx.svm.get_account(&user_pda).is_none());

        clock.unix_timestamp = registration_open_at;
        ctx.svm.set_sysvar(&clock);

        ctx.execute_instruction(register_ix(), &[&user])
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
}
