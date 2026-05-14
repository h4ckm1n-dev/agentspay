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
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
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
