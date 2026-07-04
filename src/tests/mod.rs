#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{
        CreateAssociatedTokenAccount, CreateMint, MintTo,
        spl_token::{self},
    };
    use solana_clock::Clock;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const DECIMALS: u8 = 6;
    const SECONDS_PER_DAY: i64 = 86_400;

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn ata_program() -> Pubkey {
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
            .parse()
            .unwrap()
    }

    fn system_program() -> Pubkey {
        solana_sdk_ids::system_program::ID
    }

    fn so_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for subdir in &["sbpf-solana-solana", "sbf-solana-solana"] {
            let p = manifest_dir
                .join("target")
                .join(subdir)
                .join("release/pinocchio_fundraiser.so");
            if p.exists() {
                return p;
            }
        }
        manifest_dir.join("target/deploy/pinocchio_fundraiser.so")
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let maker = Keypair::new();
        svm.airdrop(&maker.pubkey(), 100 * LAMPORTS_PER_SOL)
            .expect("airdrop failed");

        let program_data = std::fs::read(so_path())
            .expect("failed to read pinocchio_fundraiser.so — run `cargo build-sbf` first");
        svm.add_program(program_id(), &program_data)
            .expect("failed to add program");

        (svm, maker)
    }

    fn fundraiser_pda(maker: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"fundraiser", maker.as_ref()], &program_id())
    }

    fn contributor_pda(fundraiser: &Pubkey, contributor: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"contributor", fundraiser.as_ref(), contributor.as_ref()],
            &program_id(),
        )
    }

    fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[owner.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
            &ata_program(),
        )
        .0
    }

    fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
        let acc = svm.get_account(account).expect("token account not found");
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    }

    fn u64_at(data: &[u8], off: usize) -> u64 {
        u64::from_le_bytes(data[off..off + 8].try_into().unwrap())
    }

    fn is_closed(svm: &LiteSVM, account: &Pubkey) -> bool {
        match svm.get_account(account) {
            None => true,
            Some(a) => a.lamports == 0,
        }
    }

    fn advance_days(svm: &mut LiteSVM, days: i64) {
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp += days * SECONDS_PER_DAY;
        svm.set_sysvar::<Clock>(&clock);
    }

    struct Ctx {
        svm: LiteSVM,
        maker: Keypair,
        mint: Pubkey,
        fundraiser: Pubkey,
        vault: Pubkey,
    }

    fn initialize(amount_to_raise: u64, duration: u8) -> Result<Ctx, ()> {
        let (mut svm, maker) = setup();

        let mint = CreateMint::new(&mut svm, &maker)
            .decimals(DECIMALS)
            .authority(&maker.pubkey())
            .send()
            .unwrap();

        let (fundraiser, _) = fundraiser_pda(&maker.pubkey());
        let vault = ata(&fundraiser, &mint);

        let mut data = vec![0u8];
        data.extend_from_slice(&amount_to_raise.to_le_bytes());
        data.push(duration);

        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new(fundraiser, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(ata_program(), false),
            ],
            data,
        };

        let msg = Message::new(&[ix], Some(&maker.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&maker], msg, blockhash);
        svm.send_transaction(tx).map_err(|_| ())?;

        Ok(Ctx { svm, maker, mint, fundraiser, vault })
    }

    fn new_contributor(ctx: &mut Ctx, funded: u64) -> (Keypair, Pubkey) {
        let contributor = Keypair::new();
        ctx.svm
            .airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        let contributor_ata =
            CreateAssociatedTokenAccount::new(&mut ctx.svm, &contributor, &ctx.mint)
                .owner(&contributor.pubkey())
                .send()
                .unwrap();
        MintTo::new(&mut ctx.svm, &ctx.maker, &ctx.mint, &contributor_ata, funded)
            .send()
            .unwrap();
        (contributor, contributor_ata)
    }

    fn contribute_ix(
        ctx: &Ctx,
        contributor: &Pubkey,
        contributor_account: &Pubkey,
        contributor_ata: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let mut data = vec![1u8];
        data.extend_from_slice(&amount.to_le_bytes());
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*contributor, true),
                AccountMeta::new_readonly(ctx.mint, false),
                AccountMeta::new(ctx.fundraiser, false),
                AccountMeta::new(*contributor_account, false),
                AccountMeta::new(*contributor_ata, false),
                AccountMeta::new(ctx.vault, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(system_program(), false),
            ],
            data,
        }
    }

    fn check_ix(ctx: &Ctx, maker_ata: &Pubkey) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(ctx.maker.pubkey(), true),
                AccountMeta::new_readonly(ctx.mint, false),
                AccountMeta::new(ctx.fundraiser, false),
                AccountMeta::new(ctx.vault, false),
                AccountMeta::new(*maker_ata, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(system_program(), false),
                AccountMeta::new_readonly(ata_program(), false),
            ],
            data: vec![2u8],
        }
    }

    fn refund_ix(
        ctx: &Ctx,
        contributor: &Pubkey,
        contributor_account: &Pubkey,
        contributor_ata: &Pubkey,
    ) -> Instruction {
        Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(*contributor, true),
                AccountMeta::new_readonly(ctx.maker.pubkey(), false),
                AccountMeta::new_readonly(ctx.mint, false),
                AccountMeta::new(ctx.fundraiser, false),
                AccountMeta::new(*contributor_account, false),
                AccountMeta::new(*contributor_ata, false),
                AccountMeta::new(ctx.vault, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(system_program(), false),
            ],
            data: vec![3u8],
        }
    }

    fn send(svm: &mut LiteSVM, ix: Instruction, payer: &Keypair, signers: &[&Keypair]) -> bool {
        let msg = Message::new(&[ix], Some(&payer.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(signers, msg, blockhash);
        svm.send_transaction(tx).is_ok()
    }

    #[test]
    fn initialize_creates_fundraiser_and_vault() {
        let amount = 1_000_000_000u64;
        let ctx = initialize(amount, 5).expect("initialize failed");

        let acc = ctx.svm.get_account(&ctx.fundraiser).expect("fundraiser missing");
        assert_eq!(acc.owner, program_id());
        assert_eq!(acc.data.len(), 90);
        assert_eq!(&acc.data[0..32], ctx.maker.pubkey().as_ref());
        assert_eq!(&acc.data[32..64], ctx.mint.as_ref());
        assert_eq!(u64_at(&acc.data, 64), amount);
        assert_eq!(u64_at(&acc.data, 72), 0);
        assert_eq!(acc.data[88], 5);
        assert_eq!(acc.data[89], fundraiser_pda(&ctx.maker.pubkey()).1);

        let vault = ctx.svm.get_account(&ctx.vault).expect("vault missing");
        assert_eq!(vault.owner, TOKEN_PROGRAM_ID);
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 0);
    }

    #[test]
    fn initialize_rejects_amount_below_minimum() {
        // 3^DECIMALS = 729, so anything <= 729 must be rejected
        assert!(initialize(700, 5).is_err());
    }

    #[test]
    fn contribute_moves_tokens_and_updates_state() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        let amount = 100_000_000u64; // exactly the 10% cap
        let ix = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, amount);
        assert!(send(&mut ctx.svm, ix, &contributor, &[&contributor]));

        assert_eq!(token_balance(&ctx.svm, &ctx.vault), amount);
        assert_eq!(token_balance(&ctx.svm, &contributor_ata), 500_000_000 - amount);

        let ca = ctx.svm.get_account(&contributor_account).unwrap();
        assert_eq!(u64_at(&ca.data, 0), amount);
        let f = ctx.svm.get_account(&ctx.fundraiser).unwrap();
        assert_eq!(u64_at(&f.data, 72), amount);
    }

    #[test]
    fn contribute_rejects_too_small() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        // amount must be > 1
        let ix = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 1);
        assert!(!send(&mut ctx.svm, ix, &contributor, &[&contributor]));
    }

    #[test]
    fn contribute_rejects_too_big() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        // 200_000_000 > 10% of 1_000_000_000
        let ix = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 200_000_000);
        assert!(!send(&mut ctx.svm, ix, &contributor, &[&contributor]));
    }

    #[test]
    fn contribute_rejects_exceeding_per_contributor_cap() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        let first = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 100_000_000);
        assert!(send(&mut ctx.svm, first, &contributor, &[&contributor]));

        // already at the 10% cap; any further contribution must fail
        let second = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 1_000);
        assert!(!send(&mut ctx.svm, second, &contributor, &[&contributor]));
    }

    #[test]
    fn contribute_rejects_after_deadline() {
        let mut ctx = initialize(1_000_000_000, 1).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        advance_days(&mut ctx.svm, 2);

        let ix = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 100_000_000);
        assert!(!send(&mut ctx.svm, ix, &contributor, &[&contributor]));
    }

    #[test]
    fn check_contributions_pays_maker_and_closes_when_target_met() {
        let target = 1_000_000_000u64;
        let mut ctx = initialize(target, 5).unwrap();

        // reach the goal by minting straight into the vault (check keys off the live balance)
        MintTo::new(&mut ctx.svm, &ctx.maker, &ctx.mint, &ctx.vault, target)
            .send()
            .unwrap();

        let maker_ata = ata(&ctx.maker.pubkey(), &ctx.mint);
        let ix = check_ix(&ctx, &maker_ata);
        let maker = ctx.maker.insecure_clone();
        assert!(send(&mut ctx.svm, ix, &maker, &[&maker]));

        assert_eq!(token_balance(&ctx.svm, &maker_ata), target);
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 0);
        assert!(is_closed(&ctx.svm, &ctx.fundraiser));
    }

    #[test]
    fn check_contributions_rejected_when_target_not_met() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        let c = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 100_000_000);
        assert!(send(&mut ctx.svm, c, &contributor, &[&contributor]));

        let maker_ata = ata(&ctx.maker.pubkey(), &ctx.mint);
        let ix = check_ix(&ctx, &maker_ata);
        let maker = ctx.maker.insecure_clone();
        assert!(!send(&mut ctx.svm, ix, &maker, &[&maker]));

        // fundraiser untouched, vault still holds the contribution
        assert!(!is_closed(&ctx.svm, &ctx.fundraiser));
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 100_000_000);
    }

    #[test]
    fn refund_returns_tokens_and_closes_contributor_after_deadline() {
        let funded = 500_000_000u64;
        let contribution = 100_000_000u64;
        let mut ctx = initialize(1_000_000_000, 1).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, funded);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        let c = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, contribution);
        assert!(send(&mut ctx.svm, c, &contributor, &[&contributor]));

        advance_days(&mut ctx.svm, 2);

        let ix = refund_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata);
        assert!(send(&mut ctx.svm, ix, &contributor, &[&contributor]));

        assert_eq!(token_balance(&ctx.svm, &contributor_ata), funded);
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 0);
        assert!(is_closed(&ctx.svm, &contributor_account));
        let f = ctx.svm.get_account(&ctx.fundraiser).unwrap();
        assert_eq!(u64_at(&f.data, 72), 0);
    }

    #[test]
    fn refund_rejected_before_deadline() {
        let mut ctx = initialize(1_000_000_000, 5).unwrap();
        let (contributor, contributor_ata) = new_contributor(&mut ctx, 500_000_000);
        let (contributor_account, _) = contributor_pda(&ctx.fundraiser, &contributor.pubkey());

        let c = contribute_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata, 100_000_000);
        assert!(send(&mut ctx.svm, c, &contributor, &[&contributor]));

        // still within the fundraising window
        let ix = refund_ix(&ctx, &contributor.pubkey(), &contributor_account, &contributor_ata);
        assert!(!send(&mut ctx.svm, ix, &contributor, &[&contributor]));
        assert!(!is_closed(&ctx.svm, &contributor_account));
    }

    #[test]
    fn refund_rejects_another_contributors_account() {
        let mut ctx = initialize(1_000_000_000, 1).unwrap();

        // victim contributes
        let (victim, victim_ata) = new_contributor(&mut ctx, 500_000_000);
        let (victim_account, _) = contributor_pda(&ctx.fundraiser, &victim.pubkey());
        let c = contribute_ix(&ctx, &victim.pubkey(), &victim_account, &victim_ata, 100_000_000);
        assert!(send(&mut ctx.svm, c, &victim, &[&victim]));

        // attacker just needs their own ATA to receive stolen funds
        let (attacker, attacker_ata) = new_contributor(&mut ctx, 1_000_000);

        advance_days(&mut ctx.svm, 2);

        // attacker signs as themselves but passes the victim's contributor_account
        let ix = refund_ix(&ctx, &attacker.pubkey(), &victim_account, &attacker_ata);
        assert!(!send(&mut ctx.svm, ix, &attacker, &[&attacker]));

        // victim's account + the vault are untouched
        assert!(!is_closed(&ctx.svm, &victim_account));
        assert_eq!(token_balance(&ctx.svm, &ctx.vault), 100_000_000);
        assert_eq!(token_balance(&ctx.svm, &attacker_ata), 1_000_000);
    }
}
