use crate::{esplora, esplora::Utxo, storage::Storage, SECP};
use anyhow::Context;
use elements_fun::{
    bitcoin,
    bitcoin::secp256k1::SecretKey,
    secp256k1::{rand, PublicKey},
    Address, AddressParams, AssetId, TxOut,
};
use futures::{
    lock::{MappedMutexGuard, Mutex, MutexGuard},
    stream::FuturesUnordered,
    StreamExt, TryStreamExt,
};
use itertools::Itertools;
use std::{fmt, str};
use wasm_bindgen::{JsValue, UnwrapThrowExt};

pub async fn create_new(
    name: String,
    password: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<(), JsValue> {
    let storage = Storage::local_storage()?;

    let mut wallets = storage
        .get_item::<ListOfWallets>("wallets")?
        .unwrap_or_default();

    if wallets.has(&name) {
        return Err(JsValue::from_str(&format!(
            "wallet with name '{}' already exists",
            name
        )));
    }

    wallets.add(name.clone());

    let wallet_sk = SecretKey::new(&mut rand::thread_rng());

    #[cfg(not(test))]
    let params = scrypt::ScryptParams::recommended();
    #[cfg(test)] // use weak parameters for testing
    let params = scrypt::ScryptParams::new(1, 1, 1).unwrap();

    let hashed_password = map_err_from_anyhow!(
        scrypt::scrypt_simple(&password, &params).context("failed to hash password")
    )?;

    storage.set_item(&format!("wallets.{}.password", name), hashed_password)?;
    storage.set_item(&format!("wallets.{}.secret_key", name), wallet_sk)?; // TODO: encrypt secret key!
    storage.set_item("wallets", wallets)?;

    let new_wallet = Wallet {
        name,
        encryption_key: [0u8; 32], // TODO: Derive from password,
        secret_key: wallet_sk,
    };

    current_wallet.lock().await.replace(new_wallet);

    log::info!("New wallet successfully initialized");

    Ok(())
}

pub async fn load_existing(
    name: String,
    password: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<(), JsValue> {
    let mut guard = current_wallet.lock().await;

    if let Some(Wallet { name: loaded, .. }) = &*guard {
        return Err(JsValue::from_str(&format!(
            "cannot load wallet '{}' because wallet '{}' is currently loaded",
            name, loaded
        )));
    }

    let storage = Storage::local_storage()?;
    let wallets = storage
        .get_item::<ListOfWallets>("wallets")?
        .unwrap_or_default();

    if !wallets.has(&name) {
        return Err(JsValue::from_str(&format!(
            "wallet '{}' does not exist",
            name
        )));
    }

    let stored_password = storage
        .get_item::<String>(&format!("wallets.{}.password", name))?
        .ok_or_else(|| JsValue::from_str("no password stored for wallet"))?;

    scrypt::scrypt_check(&password, &stored_password)
        .map_err(|_| JsValue::from_str(&format!("bad password for wallet '{}'", name)))?;

    let secret_key = storage
        .get_item(&format!("wallets.{}.secret_key", name))?
        .ok_or_else(|| JsValue::from_str("no secret key for wallet"))?;

    let wallet = Wallet {
        name,
        encryption_key: [0u8; 32], // derive from password + store data
        secret_key,
    };

    guard.replace(wallet);

    log::info!("Wallet successfully loaded");

    Ok(())
}

pub async fn unload_current(current_wallet: &Mutex<Option<Wallet>>) {
    let mut guard = current_wallet.lock().await;

    if guard.is_none() {
        log::debug!("Wallet is already unloaded");
        return;
    }

    *guard = None;
}

pub async fn get_status(
    name: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<WalletStatus, JsValue> {
    let storage = Storage::local_storage()?;

    let wallets = storage
        .get_item::<ListOfWallets>("wallets")?
        .unwrap_or_default();
    let exists = wallets.has(&name);

    let guard = current_wallet.lock().await;
    let loaded = guard.as_ref().map_or(false, |w| w.name == name);

    Ok(WalletStatus { loaded, exists })
}

pub async fn get_address(
    name: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<Address, JsValue> {
    let wallet = current(name, current_wallet).await?;

    let address = wallet.get_address()?;

    Ok(address)
}

pub async fn get_balances(
    name: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<Vec<BalanceEntry>, JsValue> {
    let wallet = current(name, current_wallet).await?;

    let address = wallet.get_address()?;

    let utxos = map_err_from_anyhow!(esplora::fetch_utxos(&address).await)?;

    let utxos = map_err_from_anyhow!(
        utxos
            .into_iter()
            .map(|Utxo { txid, vout, .. }| async move {
                let tx = esplora::fetch_transaction(txid).await;

                tx.map(|mut tx| tx.output.remove(vout as usize))
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await
    )?;

    let grouped_utxos = utxos
        .into_iter()
        .filter_map(|utxo| match utxo {
            TxOut::Explicit(explicit) => Some((explicit.asset.0, explicit.value.0)),
            TxOut::Confidential(_) => {
                unimplemented!("unblind once we no longer depend on wally-sys")
                // match confidential.unblind(wallet.blinding_key) {
                //     Ok(unblinded_txout) => {
                //         Some((unblinded_txout.asset, unblinded_txout.value))
                //     },
                //     Err(e) => {
                //         log::warn!("failed to unblind txout: {}", e);
                //         None
                //     }
                // }
            }
            TxOut::Null(_) => None,
        })
        .group_by(|(asset, _)| *asset);

    let balances = (&grouped_utxos)
        .into_iter()
        .map(|(asset, utxos)| async move {
            BalanceEntry {
                value: utxos.map(|(_, value)| value).sum(),
                asset,
                ticker: match esplora::fetch_asset_description(&asset).await {
                    Ok(ad) => ad.ticker,
                    Err(e) => {
                        log::debug!("failed to fetched asset description: {}", e);
                        None
                    }
                },
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;

    Ok(balances)
}

pub async fn current(
    name: String,
    current_wallet: &Mutex<Option<Wallet>>,
) -> Result<MappedMutexGuard<'_, Option<Wallet>, Wallet>, JsValue> {
    let mut guard = current_wallet.lock().await;

    match &mut *guard {
        Some(wallet) if wallet.name == name => {}
        _ => {
            return Err(JsValue::from_str(&format!(
                "wallet with name '{}' is currently not loaded",
                name
            )))
        }
    };

    Ok(MutexGuard::map(guard, |w| w.as_mut().unwrap_throw()))
}

/// A single balance entry as returned by [`get_balances`].
#[derive(Debug, serde::Serialize)]
pub struct BalanceEntry {
    value: u64,
    asset: AssetId,
    /// The ticker symbol of the asset.
    ///
    /// Not all assets are part of the registry and as such, not all of them have a ticker symbol.
    ticker: Option<String>,
}

#[derive(Debug)]
pub struct Wallet {
    pub name: String,
    pub encryption_key: [u8; 32],
    pub secret_key: SecretKey,
}

impl Wallet {
    pub fn get_address(&self) -> Result<Address, JsValue> {
        let public_key = PublicKey::from_secret_key(&*SECP, &self.secret_key);
        let blinding_key = PublicKey::from_secret_key(&*SECP, &self.blinding_key());

        let address = Address::p2wpkh(
            &bitcoin::PublicKey {
                compressed: true,
                key: public_key,
            },
            Some(blinding_key),
            &AddressParams::LIQUID,
        );

        Ok(address)
    }

    /// Derive the blinding key.
    ///
    /// # Choice of salt
    ///
    /// We choose to not add a salt because the ikm is already a randomly-generated, secret value with decent entropy.
    ///
    /// # Choice of ikm
    ///
    /// We derive the blinding key from the secret key to avoid having to store two secret values on disk.
    ///
    /// # Choice of info
    ///
    /// We choose to tag the derived key with `b"BLINDING_KEY"` in case we ever want to derive something else from the secret key.
    fn blinding_key(&self) -> SecretKey {
        let h = hkdf::Hkdf::<sha2::Sha256>::new(None, self.secret_key.as_ref());

        let mut bk = [0u8; 32];
        h.expand(b"BLINDING_KEY", &mut bk)
            .expect("output length aligns with sha256");

        SecretKey::from_slice(bk.as_ref()).expect("always a valid secret key")
    }
}

#[derive(Debug, serde::Serialize)]
pub struct WalletStatus {
    loaded: bool,
    exists: bool,
}

#[derive(Default)]
pub struct ListOfWallets(Vec<String>);

impl ListOfWallets {
    #[allow(clippy::ptr_arg)] // not sure how to fix this
    fn has(&self, wallet: &String) -> bool {
        self.0.contains(wallet)
    }

    fn add(&mut self, wallet: String) {
        self.0.push(wallet);
    }
}

impl str::FromStr for ListOfWallets {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split('\t');

        Ok(ListOfWallets(split.map(|s| s.to_owned()).collect()))
    }
}

impl fmt::Display for ListOfWallets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("\t"))
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    pub async fn given_no_wallet_when_getting_address_then_fails() {
        let current_wallet = Mutex::default();

        let result = get_address("no-existent-wallet".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str(
                "wallet with name 'no-existent-wallet' is currently not loaded"
            ))
        );
    }

    #[wasm_bindgen_test]
    pub async fn given_a_wallet_can_get_an_address() {
        let current_wallet = Mutex::default();

        create_new("wallet-1".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();

        let result = get_address("wallet-1".to_owned(), &current_wallet).await;

        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    pub async fn given_a_wallet_when_unloaded_cannot_get_address() {
        let current_wallet = Mutex::default();

        create_new("wallet-2".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();

        unload_current(&current_wallet).await;
        let result = get_address("wallet-2".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str(
                "wallet with name 'wallet-2' is currently not loaded"
            ))
        );
    }

    #[wasm_bindgen_test]
    pub async fn cannot_create_two_wallets_with_same_name() {
        let current_wallet = Mutex::default();

        create_new("wallet-3".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();
        let result = create_new("wallet-3".to_owned(), "foo".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str(
                "wallet with name 'wallet-3' already exists"
            ))
        );
    }

    #[wasm_bindgen_test]
    pub async fn cannot_load_multiple_wallets_at_the_same_time() {
        let current_wallet = Mutex::default();

        create_new("wallet-4".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();
        create_new("wallet-5".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();

        let result = load_existing("wallet-4".to_owned(), "foo".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str(
                "cannot load wallet 'wallet-4' because wallet 'wallet-5' is currently loaded"
            ))
        );
    }

    #[wasm_bindgen_test]
    pub async fn cannot_load_wallet_with_wrong_password() {
        let current_wallet = Mutex::default();

        create_new("wallet-6".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();
        unload_current(&current_wallet).await;

        let result = load_existing("wallet-6".to_owned(), "bar".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str("bad password for wallet 'wallet-6'"))
        );
    }

    #[wasm_bindgen_test]
    pub async fn cannot_load_wallet_that_doesnt_exist() {
        let current_wallet = Mutex::default();

        let result = load_existing("foobar".to_owned(), "bar".to_owned(), &current_wallet).await;

        assert_eq!(
            result,
            Err(JsValue::from_str("wallet 'foobar' does not exist"))
        );
    }

    #[wasm_bindgen_test]
    pub async fn new_wallet_is_automatically_loaded() {
        let current_wallet = Mutex::default();

        create_new("wallet-7".to_owned(), "foo".to_owned(), &current_wallet)
            .await
            .unwrap();
        let status = get_status("wallet-7".to_owned(), &current_wallet)
            .await
            .unwrap();

        assert_eq!(status.loaded, true);
    }

    #[wasm_bindgen_test]
    pub async fn given_unknown_wallet_status_returns_that_it_doesnt_exist() {
        let current_wallet = Mutex::default();

        let status = get_status("wallet-8".to_owned(), &current_wallet)
            .await
            .unwrap();

        assert_eq!(status.exists, false);
    }
}
