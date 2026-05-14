//! Agent wallet: load/create the local Solana keypair used to sign x402
//! exact-scheme payloads on devnet.
//!
//! Wraps a [`solana_sdk::signature::Keypair`] in an [`AgentWallet`] so the
//! rest of the codebase (notably the MCP server and the x402 client) can
//! refer to the wallet by an opaque, [`Clone`]-able handle without leaking
//! the secret bytes through everyday types.
//!
//! Storage format matches the Solana CLI: a JSON array of 64 little-endian
//! bytes at `~/.agentspay/keypair.json`, mode `0o600`. The path can be
//! overridden via the `AGENTSPAY_KEYPAIR_PATH` environment variable.
//!
//! This module is the canonical entry point for "give me the agent's
//! keypair". Lower-level Solana helpers (USDC mint, transfer building) live
//! in [`crate::solana`].

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use thiserror::Error;

/// Environment variable that overrides the on-disk keypair location.
pub const KEYPAIR_PATH_ENV: &str = "AGENTSPAY_KEYPAIR_PATH";
/// Default location, relative to `$HOME`.
const DEFAULT_SUBPATH: &str = ".agentspay/keypair.json";

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("$HOME is not set; cannot pick default keypair path")]
    MissingHome,
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("keypair file at {path} is not valid JSON: {source}")]
    BadJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("keypair file at {path} does not contain a valid Solana keypair: {message}")]
    BadKeypair { path: PathBuf, message: String },
}

/// A loaded (or freshly generated) Solana keypair tied to this MCP server.
///
/// `AgentWallet` does **not** implement `Clone` — the secret key bytes are
/// kept behind a single owner. Share it via `Arc<AgentWallet>` instead.
pub struct AgentWallet {
    keypair: Keypair,
    pubkey: Pubkey,
}

impl AgentWallet {
    /// Load the keypair from `path`, generating a new one if the file does
    /// not exist. The generated keypair is written with mode `0o600` on
    /// Unix; on other platforms the file inherits the default ACL.
    pub fn load_or_create(path: &Path) -> Result<Self, WalletError> {
        if path.exists() {
            return Self::load(path);
        }

        // First-run: ensure parent directory exists, then write a fresh keypair.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| WalletError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let keypair = Keypair::new();
        let bytes = keypair.to_bytes().to_vec();
        let serialized = serde_json::to_string(&bytes).map_err(|e| WalletError::BadJson {
            path: path.to_path_buf(),
            source: e,
        })?;
        fs::write(path, &serialized).map_err(|e| WalletError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        set_owner_only_permissions(path)?;

        let pubkey = keypair.pubkey();
        Ok(Self { keypair, pubkey })
    }

    fn load(path: &Path) -> Result<Self, WalletError> {
        let raw = fs::read_to_string(path).map_err(|e| WalletError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let bytes: Vec<u8> = serde_json::from_str(&raw).map_err(|e| WalletError::BadJson {
            path: path.to_path_buf(),
            source: e,
        })?;
        let keypair = Keypair::try_from(bytes.as_slice()).map_err(|e| WalletError::BadKeypair {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        let pubkey = keypair.pubkey();
        Ok(Self { keypair, pubkey })
    }

    /// Returns the agent's public key.
    #[allow(dead_code)]
    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    /// Returns the agent's public key as a base58 string (Solana convention).
    pub fn pubkey_base58(&self) -> String {
        self.pubkey.to_string()
    }

    /// Borrow the underlying keypair for signing.
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }
}

/// Resolve the keypair path from `AGENTSPAY_KEYPAIR_PATH` or fall back to
/// `$HOME/.agentspay/keypair.json`.
pub fn resolved_path() -> Result<PathBuf, WalletError> {
    if let Some(value) = env::var_os(KEYPAIR_PATH_ENV) {
        let s = value.to_string_lossy();
        if !s.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(WalletError::MissingHome)?;
    Ok(home.join(DEFAULT_SUBPATH))
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<(), WalletError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| WalletError::Io {
            path: path.to_path_buf(),
            source: e,
        })?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms).map_err(|e| WalletError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<(), WalletError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_or_create_generates_then_reloads_same_pubkey() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keypair.json");

        let w1 = AgentWallet::load_or_create(&path).expect("first create");
        assert!(path.exists());
        let pk1 = w1.pubkey_base58();

        let w2 = AgentWallet::load_or_create(&path).expect("reload");
        let pk2 = w2.pubkey_base58();
        assert_eq!(pk1, pk2, "reloaded keypair must match");
    }

    #[cfg(unix)]
    #[test]
    fn created_keypair_is_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keypair.json");
        let _ = AgentWallet::load_or_create(&path).expect("create");
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "keypair file must be 0600, got {mode:o}");
    }

    #[test]
    fn corrupted_keypair_reports_useful_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("keypair.json");
        fs::write(&path, "not-json").unwrap();
        // AgentWallet intentionally does not derive Debug (the keypair holds
        // secret bytes), so we can't `unwrap_err()` — match on the variant.
        match AgentWallet::load_or_create(&path) {
            Err(WalletError::BadJson { .. }) => {}
            Err(other) => panic!("expected BadJson, got {other}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
