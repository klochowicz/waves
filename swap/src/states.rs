use crate::make_txout;
use crate::unblind_asset_from_txout;
use bitcoin::Amount;
use bitcoin::Script;
use elements_fun::confidential::Asset;
use elements_fun::confidential::Nonce;
use elements_fun::confidential::Value;
use elements_fun::wally::asset_final_vbf;
use elements_fun::Address;
use elements_fun::AssetId;
use elements_fun::OutPoint;
use elements_fun::Transaction;
use elements_fun::TxIn;
use elements_fun::TxOut;
use elements_fun::TxOutWitness;
use rand::CryptoRng;
use rand::RngCore;
use secp256k1::SecretKey;
use secp256k1::Signature;

/// Sent from Alice to Bob, assuming Alice has bitcoin.
pub struct Message0 {
    pub input: TxIn,
    pub input_as_txout: TxOut,
    pub input_blinding_sk: SecretKey,
    pub address_redeem: Address,
    pub abf_redeem: SecretKey,
    pub vbf_redeem: SecretKey,
    pub address_change: Address,
    pub abf_change: SecretKey,
    pub vbf_change: SecretKey,
    pub fee: Amount,
}

/// Sent from Bob to Alice.
pub struct Message1 {
    pub input: TxIn,
    pub asset_id_in: AssetId,
    pub asset_id_commitment_in: Asset,
    pub abf_in: SecretKey,
    pub address_redeem: Address,
    pub abf_redeem: SecretKey,
    pub vbf_redeem: SecretKey,
    pub signature: Signature,
}

pub struct Alice0 {
    pub amount_alice: Amount,
    pub amount_bob: Amount,
    pub input: TxIn,
    pub input_as_txout: TxOut,
    pub input_sk: SecretKey,
    pub input_blinding_sk: SecretKey,
    pub asset_id_bob: AssetId,
    pub address_redeem: Address,
    pub abf_redeem: SecretKey,
    pub vbf_redeem: SecretKey,
    pub address_change: Address,
    pub abf_change: SecretKey,
    pub vbf_change: SecretKey,
    pub fee: Amount,
}

impl Alice0 {
    pub fn new<R>(
        rng: &mut R,
        amount_alice: Amount,
        amount_bob: Amount,
        // TODO: Define struct
        input: (OutPoint, TxOut),
        input_sk: SecretKey,
        input_blinding_sk: SecretKey,
        asset_id_bob: AssetId,
        address_redeem: Address,
        address_change: Address,
        fee: Amount,
    ) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let input_as_txout = input.1;
        let input = TxIn {
            previous_output: input.0,
            is_pegin: false,
            has_issuance: false,
            script_sig: Default::default(),
            sequence: 0xFFFF_FFFF,
            asset_issuance: Default::default(),
            witness: Default::default(),
        };

        let abf_redeem = SecretKey::new(rng);
        let vbf_redeem = SecretKey::new(rng);

        let abf_change = SecretKey::new(rng);
        let vbf_change = SecretKey::new(rng);

        Self {
            amount_alice,
            amount_bob,
            input,
            input_as_txout,
            input_sk,
            input_blinding_sk,
            asset_id_bob,
            address_redeem,
            abf_redeem,
            vbf_redeem,
            address_change,
            abf_change,
            vbf_change,
            fee,
        }
    }

    pub fn compose(&self) -> Message0 {
        Message0 {
            input: self.input.clone(),
            input_as_txout: self.input_as_txout.clone(),
            input_blinding_sk: self.input_blinding_sk,
            address_redeem: self.address_redeem.clone(),
            abf_redeem: self.abf_redeem,
            vbf_redeem: self.vbf_redeem,
            address_change: self.address_change.clone(),
            abf_change: self.abf_change,
            vbf_change: self.vbf_change,
            fee: self.fee,
        }
    }
}

pub struct Bob0 {
    pub amount_alice: Amount,
    pub amount_bob: Amount,
    pub input: TxIn,
    pub input_as_txout: TxOut,
    pub input_sk: SecretKey,
    pub input_blinding_sk: SecretKey,
    pub asset_id_alice: AssetId,
    pub address_redeem: Address,
    pub abf_redeem: SecretKey,
    pub vbf_redeem: SecretKey,
    pub address_change: Address,
    pub abf_change: SecretKey,
}

impl Bob0 {
    pub fn new<R>(
        rng: &mut R,
        amount_alice: Amount,
        amount_bob: Amount,
        // TODO: Define struct
        input: (OutPoint, TxOut),
        input_sk: SecretKey,
        input_blinding_sk: SecretKey,
        asset_id_alice: AssetId,
        address_redeem: Address,
        address_change: Address,
    ) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let input_as_txout = input.1;

        let input = TxIn {
            previous_output: input.0,
            is_pegin: false,
            has_issuance: false,
            script_sig: Default::default(),
            sequence: 0xFFFF_FFFF,
            asset_issuance: Default::default(),
            witness: Default::default(),
        };

        let abf_redeem = SecretKey::new(rng);
        let vbf_redeem = SecretKey::new(rng);

        let abf_change = SecretKey::new(rng);

        Self {
            amount_alice,
            amount_bob,
            input,
            input_as_txout,
            input_sk,
            input_blinding_sk,
            asset_id_alice,
            address_redeem,
            abf_redeem,
            vbf_redeem,
            address_change,
            abf_change,
        }
    }

    pub fn interpret<R>(self, rng: &mut R, msg: Message0) -> Bob1
    where
        R: RngCore + CryptoRng,
    {
        // TODO: Verify amounts and assets

        let (
            asset_id_alice,
            asset_id_commitment_in_alice,
            abf_in_alice,
            vbf_in_alice,
            amount_in_alice,
        ) = unblind_asset_from_txout(msg.input_as_txout, msg.input_blinding_sk);
        let (asset_id_bob, asset_id_commitment_in_bob, abf_in_bob, vbf_in_bob, amount_in_bob) =
            unblind_asset_from_txout(self.input_as_txout, self.input_blinding_sk);

        let abfs = vec![
            abf_in_alice.as_ref().to_vec(),
            abf_in_bob.as_ref().to_vec(),
            msg.abf_redeem.as_ref().to_vec(),
            self.abf_redeem.as_ref().to_vec(),
            msg.abf_change.as_ref().to_vec(),
            self.abf_change.as_ref().to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let vbfs = vec![
            vbf_in_alice.as_ref().to_vec(),
            vbf_in_bob.as_ref().to_vec(),
            msg.vbf_redeem.as_ref().to_vec(),
            self.vbf_redeem.as_ref().to_vec(),
            msg.vbf_change.as_ref().to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        let change_amount_alice = amount_in_alice - self.amount_alice - msg.fee;
        let change_amount_bob = amount_in_bob - self.amount_bob;

        let input_alice = msg.input;
        let input_bob = self.input;

        let inputs = vec![
            (asset_id_alice, asset_id_commitment_in_alice, abf_in_alice),
            (asset_id_bob, asset_id_commitment_in_bob, abf_in_bob),
        ];

        let redeem_output_alice = make_txout(
            rng,
            self.amount_alice,
            msg.address_redeem,
            asset_id_bob,
            *msg.abf_redeem.as_ref(),
            *msg.vbf_redeem.as_ref(),
            &inputs,
        );
        let redeem_output_bob = make_txout(
            rng,
            self.amount_bob,
            self.address_redeem,
            self.asset_id_alice,
            *self.abf_redeem.as_ref(),
            *self.vbf_redeem.as_ref(),
            &inputs,
        );

        let change_output_alice = make_txout(
            rng,
            change_amount_alice,
            msg.address_change,
            self.asset_id_alice,
            *msg.abf_change.as_ref(),
            *msg.vbf_change.as_ref(),
            &inputs,
        );

        let vbf_change_bob = asset_final_vbf(
            vec![
                amount_in_alice.as_sat(),
                amount_in_bob.as_sat(),
                self.amount_alice.as_sat(),
                self.amount_bob.as_sat(),
                change_amount_alice.as_sat(),
                change_amount_bob.as_sat(),
            ],
            2,
            abfs,
            vbfs,
        );
        let change_output_bob = make_txout(
            rng,
            change_amount_bob,
            self.address_change,
            asset_id_bob,
            *self.abf_change.as_ref(),
            vbf_change_bob,
            &inputs,
        );

        let fee = TxOut {
            asset: Asset::Explicit(self.asset_id_alice),
            value: Value::Explicit(msg.fee.as_sat()),
            nonce: Nonce::Null,
            script_pubkey: Script::default(),
            witness: TxOutWitness::default(),
        };

        let transaction = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![input_alice, input_bob],
            output: vec![
                redeem_output_alice.clone(),
                redeem_output_bob,
                change_output_alice,
                change_output_bob,
                fee,
            ],
        };

        Bob1 { transaction }
    }
}

pub struct Bob1 {
    // TODO: Add missing info for Message1
    pub transaction: Transaction,
}
