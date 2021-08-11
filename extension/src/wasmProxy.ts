import Debug from "debug";
import {
    Address,
    BalanceUpdate,
    CreateSwapPayload,
    LoanDetails,
    LoanRequestPayload,
    Status,
    Trade,
    Txid,
    WalletStatus,
} from "./models";

Debug.enable("*");
const debug = Debug("wasmProxy");

export async function walletStatus(name: string): Promise<WalletStatus> {
    const { wallet_status } = await import("./wallet");

    debug("walletStatus");
    const status = await wallet_status(name);

    if (status.loaded && status.exists) {
        let address = await getAddress(name);
        return { status: Status.Loaded, address };
    } else if (status.exists) {
        return { status: Status.NotLoaded };
    } else {
        return { status: Status.None };
    }
}

export async function getAddress(name: string): Promise<Address> {
    const { get_address } = await import("./wallet");

    debug("getAddress");
    return get_address(name);
}

export async function unlockWallet(name: string, password: string): Promise<void> {
    const { load_existing_wallet } = await import("./wallet");

    debug("unlockWallet");
    return load_existing_wallet(name, password);
}

export async function getBalances(name: string): Promise<BalanceUpdate> {
    const { get_balances } = await import("./wallet");

    debug("getBalances");
    return get_balances(name);
}

export async function makeSellCreateSwapPayload(name: string, btc: string): Promise<CreateSwapPayload> {
    const { make_sell_create_swap_payload } = await import("./wallet");

    debug("makeSellCreateSwapPayload");
    return make_sell_create_swap_payload(name, btc);
}

export async function makeBuyCreateSwapPayload(name: string, usdt: string): Promise<CreateSwapPayload> {
    const { make_buy_create_swap_payload } = await import("./wallet");

    debug("makeBuyCreateSwapPayload");
    return make_buy_create_swap_payload(name, usdt);
}

export async function makeLoanRequestPayload(
    name: string,
    collateral: string,
    fee_rate: string,
): Promise<LoanRequestPayload> {
    const { make_loan_request } = await import("./wallet");

    debug("makeLoanRequestPayload");
    return make_loan_request(name, collateral, fee_rate);
}

export async function signAndSendSwap(name: string, hex: string): Promise<Txid> {
    const { sign_and_send_swap_transaction } = await import("./wallet");

    debug("signAndSendSwap");

    const tx = { inner: hex };
    return sign_and_send_swap_transaction(name, tx);
}

export async function extractTrade(name: string, hex: string): Promise<Trade> {
    const { extract_trade } = await import("./wallet");

    debug("extractTrade");
    const tx = { inner: hex };
    return extract_trade(name, tx);
}

// TODO: Replace any with actual LoanResponse interface
export async function extractLoan(name: string, loanResponse: any): Promise<LoanDetails> {
    const { extract_loan } = await import("./wallet");

    debug("extractLoan");
    return extract_loan(name, loanResponse);
}

export async function signLoan(name: string): Promise<string> {
    const { sign_loan } = await import("./wallet");

    debug("signLoan");
    return (await sign_loan(name)).inner;
}

export async function withdrawAll(name: string, address: string): Promise<Txid> {
    const { withdraw_everything_to } = await import("./wallet");

    debug("withdrawAll");
    return withdraw_everything_to(name, address);
}

export async function getOpenLoans(): Promise<LoanDetails[]> {
    const { get_open_loans } = await import("./wallet");

    debug("getOpenLoans");
    return get_open_loans();
}

export async function repayLoan(name: string, txid: string): Promise<void> {
    const { repay_loan } = await import("./wallet");

    debug("repayLoan");
    return repay_loan(name, txid);
}

export async function getPastTransactions(name: string): Promise<Txid[]> {
    const { get_past_transactions } = await import("./wallet");

    debug("getPastTransactions");
    return get_past_transactions(name);
}

export async function bip39SeedWords(): Promise<string> {
    const { bip39_seed_words } = await import("./wallet");

    debug("bip39_seed_words");
    return bip39_seed_words();
}

export async function createNewBip39Wallet(name: string, seedWords: string, password: string): Promise<string> {
    const { create_new_bip39_wallet } = await import("./wallet");

    debug("create_new_bip39_wallet");
    return create_new_bip39_wallet(name, seedWords, password);
}
