use baru::loan::Borrower1;
use elements::Transaction;
use futures::lock::Mutex;

use crate::{
    esplora::EsploraClient,
    storage::Storage,
    wallet::{current, LoanDetails},
    Wallet,
};

pub(crate) async fn sign_loan(
    name: String,
    current_wallet: &Mutex<Option<Wallet>>,
    client: &EsploraClient,
) -> Result<Transaction, Error> {
    let storage = Storage::local_storage().map_err(Error::Storage)?;
    let borrower = storage
        .get_item::<String>("borrower_state")
        .map_err(Error::Load)?
        .ok_or(Error::EmptyState)?;
    let (borrower, loan_details) =
        serde_json::from_str::<(Borrower1, LoanDetails)>(&borrower).map_err(Error::Deserialize)?;

    let loan_transaction = borrower
        .sign(|transaction| async {
            let mut wallet = current(&name, current_wallet).await?;
            wallet.sync(&*client).await?;
            let transaction = wallet.sign(transaction);

            Ok(transaction)
        })
        .await
        .map_err(Error::Sign)?;

    // We don't broadcast this transaction ourselves, but we expect
    // the lender to do so very soon. We therefore save the borrower
    // state so that we can later on build, sign and broadcast the
    // repayment transaction

    let mut open_loans = match storage
        .get_item::<String>("open_loans")
        .map_err(Error::Load)?
    {
        Some(open_loans) => serde_json::from_str(&open_loans).map_err(Error::Deserialize)?,
        None => Vec::<LoanDetails>::new(),
    };

    open_loans.push(loan_details);
    storage
        .set_item(
            "open_loans",
            serde_json::to_string(&open_loans).map_err(Error::Serialize)?,
        )
        .map_err(Error::Save)?;

    storage
        .set_item(
            &format!("loan_state:{}", loan_transaction.txid()),
            serde_json::to_string(&borrower).map_err(Error::Serialize)?,
        )
        .map_err(Error::Save)?;

    Ok(loan_transaction)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(anyhow::Error),
    #[error("Failed to load item from storage: {0}")]
    Load(anyhow::Error),
    #[error("Loaded empty borrower state")]
    EmptyState,
    #[error("Failed to save item to storage: {0}")]
    Save(anyhow::Error),
    #[error("Deserialization failed: {0}")]
    Deserialize(serde_json::Error),
    #[error("Serialization failed: {0}")]
    Serialize(serde_json::Error),
    #[error("Failed to sign transaction: {0}")]
    Sign(anyhow::Error),
}
