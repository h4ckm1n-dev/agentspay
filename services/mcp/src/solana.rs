//! Solana devnet primitives for AgentsPay.
//!
//! Lower-level Solana helpers used by [`crate::x402`] when signing real
//! payments:
//!
//! * Devnet USDC mint, RPC URL helpers.
//! * Balance lookups (introspection — exposed for v0.3 features).
//! * Construction of a signed USDC `transfer_checked` transaction with an
//!   idempotent recipient-ATA creation instruction.
//! * Bincode + base64 encoding of the resulting transaction.
//!
//! Keypair load/create lives in [`crate::wallet`].
//!
//! Wire format note: we follow the x402 Solana payload convention — the
//! `payload.transaction` field is the **base64 of a `bincode`-serialized
//! signed `Transaction`**, and `PaymentRequirements.asset` is the mint
//! pubkey (base58), with `extra.decimals` giving the token decimals.
//! `maxAmountRequired` is in token base units (1 USDC = 1_000_000 micro-units).

use std::{env, str::FromStr};

use anyhow::{Context, Result};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_token::instruction as token_ix;

/// Circle's USDC mint on Solana **devnet** (base58).
pub const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
/// USDC has 6 decimals.
pub const USDC_DECIMALS: u8 = 6;
/// Default RPC URL (Solana Labs public devnet).
pub const DEFAULT_DEVNET_RPC: &str = "https://api.devnet.solana.com";
/// Environment variable that overrides the RPC URL.
pub const RPC_URL_ENV: &str = "AGENTSPAY_SOLANA_RPC_URL";

/// Returns the configured Solana RPC URL.
pub fn default_rpc_url() -> String {
    env::var(RPC_URL_ENV).unwrap_or_else(|_| DEFAULT_DEVNET_RPC.to_string())
}

/// Returns the devnet USDC mint as a `Pubkey`.
pub fn usdc_mint_devnet() -> Pubkey {
    // This pubkey is a hardcoded compile-time constant; `from_str` cannot
    // fail in practice — the test below guards the invariant.
    Pubkey::from_str(USDC_MINT_DEVNET).expect("USDC_MINT_DEVNET must be a valid base58 pubkey")
}

/// Returns the SOL balance for `owner` in lamports.
#[allow(dead_code)] // exposed for future v0.3 balance-introspection helpers.
pub async fn sol_balance_lamports(rpc: &RpcClient, owner: &Pubkey) -> Result<u64> {
    let lamports = rpc
        .get_balance(owner)
        .await
        .with_context(|| format!("get_balance({owner})"))?;
    Ok(lamports)
}

/// Returns the USDC balance for `owner` in base units (6-decimal micro-USDC).
/// Returns `0` when the owner has no USDC associated-token-account yet.
#[allow(dead_code)] // exposed for future v0.3 balance-introspection helpers.
pub async fn usdc_balance_base_units(rpc: &RpcClient, owner: &Pubkey) -> Result<u64> {
    let mint = usdc_mint_devnet();
    let ata = get_associated_token_address(owner, &mint);

    // Fast path: the ATA usually exists once funded. We use `getTokenAccountsByOwner`
    // filtered by the USDC mint so a non-existent ATA returns an empty array
    // rather than an RPC error.
    let accounts = rpc
        .get_token_accounts_by_owner(owner, TokenAccountsFilter::Mint(mint))
        .await
        .with_context(|| format!("get_token_accounts_by_owner({owner})"))?;

    if accounts.is_empty() {
        return Ok(0);
    }

    // Prefer the canonical ATA pubkey; fall back to any returned account.
    let target = accounts
        .iter()
        .find(|a| a.pubkey == ata.to_string())
        .or_else(|| accounts.first())
        .expect("non-empty checked");

    let balance = rpc
        .get_token_account_balance(&Pubkey::from_str(&target.pubkey)?)
        .await
        .with_context(|| format!("get_token_account_balance({})", target.pubkey))?;
    let amount: u64 = balance
        .amount
        .parse()
        .with_context(|| format!("parsing token amount {}", balance.amount))?;
    Ok(amount)
}

/// Build (and sign) a Solana transaction that transfers `amount_base_units`
/// of USDC from `payer` to `recipient`. The recipient's ATA is created via
/// `create_associated_token_account_idempotent` so the caller does not need
/// to know whether it already exists.
///
/// The transaction is signed with `payer`, ready to be submitted with
/// `send_and_confirm_transaction`.
pub async fn build_usdc_transfer_tx(
    rpc: &RpcClient,
    payer: &Keypair,
    recipient: &Pubkey,
    amount_base_units: u64,
) -> Result<Transaction> {
    let mint = usdc_mint_devnet();
    let payer_ata = get_associated_token_address(&payer.pubkey(), &mint);
    let recipient_ata = get_associated_token_address(recipient, &mint);

    let create_recipient_ata_ix = create_associated_token_account_idempotent(
        &payer.pubkey(),
        recipient,
        &mint,
        &spl_token::id(),
    );

    let transfer_ix = token_ix::transfer_checked(
        &spl_token::id(),
        &payer_ata,
        &mint,
        &recipient_ata,
        &payer.pubkey(),
        &[&payer.pubkey()],
        amount_base_units,
        USDC_DECIMALS,
    )
    .context("building transfer_checked instruction")?;

    let blockhash = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .context("get_latest_blockhash")?
        .0;

    let tx = Transaction::new_signed_with_payer(
        &[create_recipient_ata_ix, transfer_ix],
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );
    Ok(tx)
}

/// SPL Memo program v2. The x402 SVM "exact" scheme requires a memo
/// instruction carrying `extra.memo` or a random nonce.
pub const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

/// Compute-unit limit requested for the sponsored transfer. An SPL
/// `transfer_checked` plus a memo costs well under this; the facilitator pays
/// the fee, so we ask for a tight bound rather than the 200k default.
const SPONSORED_COMPUTE_UNIT_LIMIT: u32 = 30_000;

/// Compute-unit price (micro-lamports per CU). The *facilitator* pays this, so
/// we do not bid their money up — the scheme only requires the instruction to
/// be present. Raise only if facilitators start requiring a floor.
const SPONSORED_COMPUTE_UNIT_PRICE: u64 = 0;

/// Build a **partially-signed v0 versioned transaction** for the x402 SVM
/// "exact" scheme, where a facilitator sponsors the fee.
///
/// Differences from [`build_usdc_transfer_tx`] (the legacy self-relay path),
/// all mandated by the scheme:
///
/// * The **fee payer is `fee_payer`** (the facilitator, from
///   `requirement.extra.feePayer`) — not the agent. The agent therefore needs
///   no SOL.
/// * Instructions MUST be exactly, in order: compute-unit limit, compute-unit
///   price, `transfer_checked`, memo. Notably an idempotent ATA-create
///   instruction is **not** permitted, so the recipient's ATA must already
///   exist — a real x402 seller's does.
/// * Only the agent signs; the fee payer's signature slot is left empty for
///   the facilitator to fill.
///
/// Fee-payer isolation (which the facilitator enforces and will reject on) is
/// satisfied by construction: `fee_payer` appears only as the message fee
/// payer, never inside an instruction's accounts, never as the transfer
/// authority, never as the source.
pub async fn build_sponsored_usdc_transfer_tx(
    rpc: &RpcClient,
    authority: &Keypair,
    fee_payer: &Pubkey,
    recipient: &Pubkey,
    amount_base_units: u64,
    memo: &str,
) -> Result<VersionedTransaction> {
    let blockhash = rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .context("get_latest_blockhash")?
        .0;
    let message = build_sponsored_transfer_message(
        &authority.pubkey(),
        fee_payer,
        recipient,
        amount_base_units,
        memo,
        blockhash,
    )?;
    partially_sign_v0(message, authority)
}

/// Pure half of [`build_sponsored_usdc_transfer_tx`]: compile the v0 message
/// for a given blockhash. Split out so the scheme-mandated instruction order
/// and fee-payer isolation are unit-testable without an RPC round-trip.
pub fn build_sponsored_transfer_message(
    authority: &Pubkey,
    fee_payer: &Pubkey,
    recipient: &Pubkey,
    amount_base_units: u64,
    memo: &str,
    blockhash: Hash,
) -> Result<VersionedMessage> {
    if fee_payer == authority {
        anyhow::bail!("facilitator feePayer must not be the agent's own pubkey");
    }
    let mint = usdc_mint_devnet();
    let source_ata = get_associated_token_address(authority, &mint);
    let recipient_ata = get_associated_token_address(recipient, &mint);

    let transfer_ix = token_ix::transfer_checked(
        &spl_token::id(),
        &source_ata,
        &mint,
        &recipient_ata,
        authority,
        &[authority],
        amount_base_units,
        USDC_DECIMALS,
    )
    .context("building transfer_checked instruction")?;

    let memo_program: Pubkey = MEMO_PROGRAM_ID
        .parse()
        .context("parsing SPL memo program id")?;
    // Memo takes its content as instruction data; no accounts, which also
    // keeps the fee payer out of every instruction's account list.
    let memo_ix = Instruction::new_with_bytes(memo_program, memo.as_bytes(), vec![]);

    // Order is mandated by the scheme: limit, price, transfer, memo.
    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(SPONSORED_COMPUTE_UNIT_LIMIT),
        ComputeBudgetInstruction::set_compute_unit_price(SPONSORED_COMPUTE_UNIT_PRICE),
        transfer_ix,
        memo_ix,
    ];

    let message = v0::Message::try_compile(fee_payer, &instructions, &[], blockhash)
        .context("compiling v0 message")?;
    Ok(VersionedMessage::V0(message))
}

/// Sign `message` with `authority` only, leaving every other required
/// signature slot (i.e. the facilitator fee payer at index 0) zeroed.
fn partially_sign_v0(
    message: VersionedMessage,
    authority: &Keypair,
) -> Result<VersionedTransaction> {
    let required = message.header().num_required_signatures as usize;
    let signer_index = message
        .static_account_keys()
        .iter()
        .position(|k| k == &authority.pubkey())
        .context("agent pubkey is not a required signer of the compiled message")?;
    anyhow::ensure!(
        signer_index < required,
        "agent pubkey is not in the signer section of the message"
    );

    let mut signatures = vec![Signature::default(); required];
    signatures[signer_index] = authority.sign_message(&message.serialize());
    Ok(VersionedTransaction {
        signatures,
        message,
    })
}

/// Serialize a partially-signed `VersionedTransaction` to the x402 wire
/// format: `bincode`, then base64 (standard alphabet).
pub fn encode_versioned_transaction_b64(tx: &VersionedTransaction) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let bytes = bincode::serialize(tx).context("bincode serialize versioned transaction")?;
    Ok(BASE64.encode(bytes))
}

/// Serialize a signed Solana `Transaction` to the wire format expected by an
/// x402 Solana facilitator: `bincode` of the `Transaction`, then base64
/// (standard alphabet). Round-trippable via `bincode::deserialize`.
pub fn encode_transaction_b64(tx: &Transaction) -> Result<String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let bytes = bincode::serialize(tx).context("bincode serialize transaction")?;
    Ok(BASE64.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usdc_mint_parses() {
        let _ = usdc_mint_devnet();
    }

    #[test]
    fn default_rpc_falls_back_to_devnet() {
        // Force unset for this test; another test may set it, so we don't
        // assert in the presence of env-var leakage.
        if env::var(RPC_URL_ENV).is_err() {
            assert_eq!(default_rpc_url(), DEFAULT_DEVNET_RPC);
        }
    }

    // ------------------------------------------------------------------
    // x402 SVM "exact" scheme: facilitator-sponsored transaction shape.
    // The facilitator REJECTS a transaction that breaks any of these, and we
    // cannot reach a real facilitator from a unit test, so the structure is
    // pinned here instead.
    // ------------------------------------------------------------------

    fn sponsored_fixture() -> (Keypair, Pubkey, Pubkey, VersionedMessage) {
        let agent = Keypair::new();
        let fee_payer = Keypair::new().pubkey();
        let recipient = Keypair::new().pubkey();
        let message = build_sponsored_transfer_message(
            &agent.pubkey(),
            &fee_payer,
            &recipient,
            100_000,
            "a1b2c3d4e5f60718293a4b5c6d7e8f90",
            Hash::default(),
        )
        .expect("message compiles");
        (agent, fee_payer, recipient, message)
    }

    #[test]
    fn sponsored_message_has_scheme_mandated_instruction_order() {
        let (_agent, _fee_payer, _recipient, message) = sponsored_fixture();
        let keys = message.static_account_keys().to_vec();
        let ixs = message.instructions();
        // The scheme allows 3-6 instructions; we emit exactly 4.
        assert_eq!(ixs.len(), 4, "limit, price, transfer_checked, memo");

        let compute_budget: Pubkey = "ComputeBudget111111111111111111111111111111"
            .parse()
            .unwrap();
        let memo_program: Pubkey = MEMO_PROGRAM_ID.parse().unwrap();

        assert_eq!(keys[ixs[0].program_id_index as usize], compute_budget);
        assert_eq!(ixs[0].data[0], 2, "SetComputeUnitLimit discriminator");
        assert_eq!(keys[ixs[1].program_id_index as usize], compute_budget);
        assert_eq!(ixs[1].data[0], 3, "SetComputeUnitPrice discriminator");
        assert_eq!(keys[ixs[2].program_id_index as usize], spl_token::id());
        assert_eq!(ixs[2].data[0], 12, "transfer_checked discriminator");
        assert_eq!(keys[ixs[3].program_id_index as usize], memo_program);
    }

    /// Fee-payer isolation: the facilitator refuses to sponsor a transaction
    /// where its own account appears inside any instruction, is the transfer
    /// authority, or is the source of funds.
    #[test]
    fn sponsored_message_keeps_fee_payer_out_of_every_instruction() {
        let (agent, fee_payer, _recipient, message) = sponsored_fixture();
        let keys = message.static_account_keys().to_vec();

        // Fee payer is the message fee payer => account index 0, and signs.
        assert_eq!(keys[0], fee_payer, "fee payer must be account index 0");
        assert!(message.header().num_required_signatures >= 2);

        for ix in message.instructions() {
            for account_index in &ix.accounts {
                assert_ne!(
                    keys[*account_index as usize], fee_payer,
                    "fee payer must not appear in any instruction's accounts"
                );
            }
        }

        // The agent — never the fee payer — is the transfer authority and the
        // source owner.
        let transfer = &message.instructions()[2];
        let authority = keys[*transfer.accounts.last().unwrap() as usize];
        assert_eq!(authority, agent.pubkey());
        assert_ne!(authority, fee_payer);
    }

    #[test]
    fn sponsored_tx_is_partially_signed_leaving_fee_payer_slot_empty() {
        let (agent, _fee_payer, _recipient, message) = sponsored_fixture();
        let tx = partially_sign_v0(message, &agent).expect("partial sign");
        let keys = tx.message.static_account_keys().to_vec();
        let required = tx.message.header().num_required_signatures as usize;
        assert_eq!(tx.signatures.len(), required);

        // Fee payer (index 0) is left for the facilitator to fill.
        assert_eq!(
            tx.signatures[0],
            Signature::default(),
            "facilitator's fee-payer slot must be left empty"
        );
        // The agent's own slot carries a real signature that verifies.
        let agent_index = keys.iter().position(|k| *k == agent.pubkey()).unwrap();
        assert_ne!(tx.signatures[agent_index], Signature::default());
        assert!(tx.signatures[agent_index].verify(agent.pubkey().as_ref(), &tx.message.serialize()));

        // Round-trips through the wire encoding.
        let b64 = encode_versioned_transaction_b64(&tx).expect("encode");
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        let raw = BASE64.decode(&b64).expect("base64");
        let back: VersionedTransaction = bincode::deserialize(&raw).expect("bincode");
        assert_eq!(back.signatures, tx.signatures);
    }

    #[test]
    fn sponsored_message_rejects_fee_payer_equal_to_agent() {
        let agent = Keypair::new();
        let err = build_sponsored_transfer_message(
            &agent.pubkey(),
            &agent.pubkey(),
            &Keypair::new().pubkey(),
            100_000,
            "nonce",
            Hash::default(),
        )
        .expect_err("agent must not be its own facilitator fee payer");
        assert!(err.to_string().contains("feePayer"));
    }

    #[test]
    fn encode_then_decode_roundtrip_via_bincode() {
        // We can't reach the network here, so build an unsigned dummy tx
        // with a known blockhash and confirm bincode + base64 round-trip.
        // Build a no-op instruction manually so we don't depend on the
        // deprecated `solana_sdk::system_instruction` path; round-trip
        // exercises the serializer, not the instruction semantics.
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        use solana_sdk::{
            hash::Hash,
            instruction::{AccountMeta, Instruction},
            message::Message,
        };

        let payer = Keypair::new();
        let recipient = Keypair::new().pubkey();
        let ix = Instruction::new_with_bytes(
            solana_sdk::system_program::id(),
            &[2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(recipient, false),
            ],
        );
        let blockhash = Hash::default();
        let msg = Message::new(&[ix], Some(&payer.pubkey()));
        let mut tx = Transaction::new_unsigned(msg);
        tx.sign(&[&payer], blockhash);

        let b64 = encode_transaction_b64(&tx).expect("encode");
        let raw = BASE64.decode(&b64).expect("base64 decode");
        let back: Transaction = bincode::deserialize(&raw).expect("bincode decode");
        assert_eq!(tx.signatures, back.signatures);
        assert_eq!(tx.message_data(), back.message_data());
    }
}
