use crate::{cs_bs::BalanceEntry, Component};
use elements::Txid;
use serde::{Deserialize, Serialize};
use wallet::{CreateSwapPayload, WalletStatus};

/// Message to be send between in-page script and content script
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub rpc_data: RpcData,
    pub target: Component,
    pub source: Component,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RpcData {
    GetWalletStatus,
    GetBalance,
    GetSellCreateSwapPayload(String),
    GetBuyCreateSwapPayload(String),
    WalletStatus(WalletStatus),
    SignAndSend(String),
    Balance(Vec<BalanceEntry>),
    SellCreateSwapPayload(CreateSwapPayload),
    BuyCreateSwapPayload(CreateSwapPayload),
    SwapTxid(Txid),
    Hello(String),
}
