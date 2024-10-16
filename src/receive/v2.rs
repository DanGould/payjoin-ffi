use std::collections::HashMap;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use payjoin::bitcoin::psbt::Psbt;
use payjoin::bitcoin::FeeRate;
use payjoin::receive as pdk;

use crate::ohttp::ClientResponse;
#[cfg(feature = "uniffi")]
use crate::receive::v1::{
    CanBroadcast, GenerateScript, IsOutputKnown, IsScriptOwned, ProcessPartiallySignedTransaction,
};
use crate::types::{Network, Script};
use crate::uri::PjUriBuilder;
use crate::{OhttpKeys, OutPoint, PayjoinError, Request, TxOut, Url};

pub struct RequestResponse {
    pub request: Request,
    pub client_response: Arc<ClientResponse>,
}

#[derive(Clone, Debug)]
pub struct Receiver(pub payjoin::receive::v2::Receiver);
impl From<Receiver> for payjoin::receive::v2::Receiver {
    fn from(value: Receiver) -> Self {
        value.0
    }
}

impl From<payjoin::receive::v2::Receiver> for Receiver {
    fn from(value: payjoin::receive::v2::Receiver) -> Self {
        Self(value)
    }
}

impl Receiver {
    /// Creates a new `SessionInitializer` with the provided parameters.
    ///
    /// # Parameters
    /// - `address`: The Bitcoin address for the payjoin session.
    /// - `network`: The network to use for address verification.
    /// - `directory`: The URL of the store-and-forward payjoin directory.
    /// - `ohttp_keys`: The OHTTP keys used for encrypting and decrypting HTTP requests and responses.
    /// - `ohttp_relay`: The URL of the OHTTP relay, used to keep client IP address confidential.
    /// - `expire_after`: The duration in seconds after which the session expires.
    ///
    /// # Returns
    /// A new instance of `SessionInitializer`.
    ///
    /// # References
    /// - [BIP 77: Payjoin Version 2: Serverless Payjoin](https://github.com/bitcoin/bips/pull/1483)
    #[cfg(feature = "uniffi")]
    pub fn new(
        address: String,
        network: Network,
        directory: Arc<Url>,
        ohttp_keys: Arc<OhttpKeys>,
        ohttp_relay: Arc<Url>,
        expire_after: Option<u64>,
    ) -> Result<Self, PayjoinError> {
        let address = payjoin::bitcoin::Address::from_str(address.as_str())?
            .require_network(network.into())?;
        Ok(payjoin::receive::v2::Receiver::new(
            address,
            (*directory).clone().into(),
            (*ohttp_keys).clone().into(),
            (*ohttp_relay).clone().into(),
            expire_after.map(|e| Duration::from_secs(e)),
        )
        .into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn new(
        address: String,
        network: Network,
        directory: Url,
        ohttp_keys: OhttpKeys,
        ohttp_relay: Url,
        expire_after: Option<u64>,
    ) -> Result<Self, PayjoinError> {
        let address = payjoin::bitcoin::Address::from_str(address.as_str())?
            .require_network(network.into())?;
        Ok(payjoin::receive::v2::Receiver::new(
            address,
            directory.into(),
            ohttp_keys.into(),
            ohttp_relay.into(),
            expire_after.map(|e| Duration::from_secs(e)),
        )
        .into())
    }

    #[cfg(feature = "uniffi")]
    pub fn extract_req(&self) -> Result<RequestResponse, PayjoinError> {
        match self.0.clone().extract_req() {
            Ok(e) => {
                Ok(RequestResponse { request: e.0.into(), client_response: Arc::new(e.1.into()) })
            }
            Err(e) => Err(PayjoinError::V2Error { message: e.to_string() }),
        }
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn extract_req(&self) -> Result<(Request, ohttp::ClientResponse), PayjoinError> {
        match self.0.clone().extract_req() {
            Ok(e) => Ok((e.0.into(), e.1)),
            Err(e) => Err(PayjoinError::V2Error { message: e.to_string() }),
        }
    }

    ///The response can either be an UncheckedProposal or an ACCEPTED message indicating no UncheckedProposal is available yet.
    #[cfg(feature = "uniffi")]
    pub fn process_res(
        &self,
        body: Vec<u8>,
        context: Arc<ClientResponse>,
    ) -> Result<Option<Arc<V2UncheckedProposal>>, PayjoinError> {
        <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone())
            .process_res(Cursor::new(body), context.as_ref().into())
            .map(|e| e.map(|x| Arc::new(x.into())))
            .map_err(|e| e.into())
    }
    ///The response can either be an UncheckedProposal or an ACCEPTED message indicating no UncheckedProposal is available yet.
    #[cfg(not(feature = "uniffi"))]
    pub fn process_res(
        &self,
        body: Vec<u8>,
        ctx: ohttp::ClientResponse,
    ) -> Result<Option<V2UncheckedProposal>, PayjoinError> {
        <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone())
            .process_res(Cursor::new(body), ctx)
            .map(|e| e.map(|o| o.into()))
            .map_err(|e| e.into())
    }

    #[cfg(not(feature = "uniffi"))]
    pub fn pj_uri_builder(&self) -> PjUriBuilder {
        <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone()).pj_uri_builder().into()
    }
    #[cfg(feature = "uniffi")]
    pub fn pj_uri_builder(&self) -> Arc<PjUriBuilder> {
        Arc::new(
            <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone())
                .pj_uri_builder()
                .into(),
        )
    }
    /// The contents of the `&pj=` query parameter including the base64url-encoded public key receiver subdirectory.
    /// This identifies a session at the payjoin directory server.
    #[cfg(feature = "uniffi")]
    pub fn pj_url(&self) -> Arc<Url> {
        Arc::new(<Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone()).pj_url().into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn pj_url(&self) -> Url {
        <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone()).pj_url().into()
    }
    ///The per-session public key to use as an identifier
    pub fn id(&self) -> String {
        <Self as Into<payjoin::receive::v2::Receiver>>::into(self.clone()).id().to_string()
    }
}

#[derive(Clone)]
pub struct V2UncheckedProposal(payjoin::receive::v2::UncheckedProposal);

impl From<payjoin::receive::v2::UncheckedProposal> for V2UncheckedProposal {
    fn from(value: payjoin::receive::v2::UncheckedProposal) -> Self {
        Self(value)
    }
}
impl From<V2UncheckedProposal> for payjoin::receive::v2::UncheckedProposal {
    fn from(value: V2UncheckedProposal) -> Self {
        value.0
    }
}
impl V2UncheckedProposal {
    ///The Sender’s Original PSBT
    pub fn extract_tx_to_schedule_broadcast(&self) -> Vec<u8> {
        payjoin::bitcoin::consensus::encode::serialize(
            &self.0.clone().extract_tx_to_schedule_broadcast(),
        )
    }

    #[cfg(feature = "uniffi")]
    /// Call after checking that the Original PSBT can be broadcast.
    ///
    /// Receiver MUST check that the Original PSBT from the sender
    /// can be broadcast, i.e. `testmempoolaccept` bitcoind rpc returns { "allowed": true,.. }
    /// for `extract_tx_to_sheculed_broadcast()` before calling this method.
    ///
    /// Do this check if you generate bitcoin uri to receive Payjoin on sender request without manual human approval, like a payment processor.
    /// Such so called "non-interactive" receivers are otherwise vulnerable to probing attacks.
    /// If a sender can make requests at will, they can learn which bitcoin the receiver owns at no cost.
    /// Broadcasting the Original PSBT after some time in the failure case makes incurs sender cost and prevents probing.
    ///
    /// Call this after checking downstream.
    pub fn check_broadcast_suitability(
        &self,
        min_fee_rate: Option<u64>,
        can_broadcast: Box<dyn CanBroadcast>,
    ) -> Result<Arc<V2MaybeInputsOwned>, PayjoinError> {
        self.0
            .clone()
            .check_broadcast_suitability(
                min_fee_rate.map(|x| payjoin::bitcoin::FeeRate::from_sat_per_kwu(x)),
                |tx| {
                    can_broadcast
                        .callback(payjoin::bitcoin::consensus::encode::serialize(tx))
                        .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
                },
            )
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }

    #[cfg(not(feature = "uniffi"))]
    pub fn check_broadcast_suitability(
        &self,
        min_fee_rate: Option<u64>,
        can_broadcast: impl Fn(&Vec<u8>) -> Result<bool, PayjoinError>,
    ) -> Result<Arc<V2MaybeInputsOwned>, PayjoinError> {
        self.0
            .clone()
            .check_broadcast_suitability(
                min_fee_rate.map(|x| FeeRate::from_sat_per_kwu(x)),
                |transaction| {
                    can_broadcast(&payjoin::bitcoin::consensus::encode::serialize(transaction))
                        .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
                },
            )
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }

    /// Call this method if the only way to initiate a Payjoin with this receiver
    /// requires manual intervention, as in most consumer wallets.
    ///
    /// So-called "non-interactive" receivers, like payment processors, that allow arbitrary requests are otherwise vulnerable to probing attacks.
    /// Those receivers call `extract_tx_to_check_broadcast()` and `attest_tested_and_scheduled_broadcast()` after making those checks downstream.
    pub fn assume_interactive_receiver(&self) -> Arc<V2MaybeInputsOwned> {
        Arc::new(self.0.clone().assume_interactive_receiver().into())
    }
}
#[derive(Clone)]
pub struct V2MaybeInputsOwned(payjoin::receive::v2::MaybeInputsOwned);
impl From<payjoin::receive::v2::MaybeInputsOwned> for V2MaybeInputsOwned {
    fn from(value: payjoin::receive::v2::MaybeInputsOwned) -> Self {
        Self(value)
    }
}
impl V2MaybeInputsOwned {
    #[cfg(feature = "uniffi")]
    ///Check that the Original PSBT has no receiver-owned inputs. Return original-psbt-rejected error or otherwise refuse to sign undesirable inputs.
    /// An attacker could try to spend receiver's own inputs. This check prevents that.
    pub fn check_inputs_not_owned(
        &self,
        is_owned: Box<dyn IsScriptOwned>,
    ) -> Result<Arc<V2MaybeMixedInputScripts>, PayjoinError> {
        self.0
            .clone()
            .check_inputs_not_owned(|input| {
                is_owned
                    .callback(input.to_bytes())
                    .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
            })
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn check_inputs_not_owned(
        &self,
        is_owned: impl Fn(&Vec<u8>) -> Result<bool, PayjoinError>,
    ) -> Result<Arc<V2MaybeMixedInputScripts>, PayjoinError> {
        self.0
            .clone()
            .check_inputs_not_owned(|input| {
                is_owned(&input.to_bytes())
                    .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
            })
            .map_err(|e| e.into())
            .map(|e| Arc::new(e.into()))
    }
}
#[derive(Clone)]
pub struct V2MaybeMixedInputScripts(payjoin::receive::v2::MaybeMixedInputScripts);

impl From<payjoin::receive::v2::MaybeMixedInputScripts> for V2MaybeMixedInputScripts {
    fn from(value: payjoin::receive::v2::MaybeMixedInputScripts) -> Self {
        Self(value)
    }
}

impl V2MaybeMixedInputScripts {
    /// Verify the original transaction did not have mixed input types
    /// Call this after checking downstream.
    ///
    /// Note: mixed spends do not necessarily indicate distinct wallet fingerprints.
    /// This check is intended to prevent some types of wallet fingerprinting.
    pub fn check_no_mixed_input_scripts(&self) -> Result<Arc<V2MaybeInputsSeen>, PayjoinError> {
        self.0
            .clone()
            .check_no_mixed_input_scripts()
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
}
#[derive(Clone)]
pub struct V2MaybeInputsSeen(payjoin::receive::v2::MaybeInputsSeen);
impl From<payjoin::receive::v2::MaybeInputsSeen> for V2MaybeInputsSeen {
    fn from(value: payjoin::receive::v2::MaybeInputsSeen) -> Self {
        Self(value)
    }
}

impl V2MaybeInputsSeen {
    #[cfg(feature = "uniffi")]
    /// Make sure that the original transaction inputs have never been seen before.
    /// This prevents probing attacks. This prevents reentrant Payjoin, where a sender
    /// proposes a Payjoin PSBT as a new Original PSBT for a new Payjoin.
    pub fn check_no_inputs_seen_before(
        &self,
        is_known: Box<dyn IsOutputKnown>,
    ) -> Result<Arc<V2OutputsUnknown>, PayjoinError> {
        self.0
            .clone()
            .check_no_inputs_seen_before(|outpoint| {
                is_known
                    .callback(outpoint.clone().into())
                    .map_err(|e| pdk::Error::Server(Box::new(e)))
            })
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn check_no_inputs_seen_before(
        &self,
        is_known: impl Fn(&OutPoint) -> Result<bool, PayjoinError>,
    ) -> Result<Arc<V2OutputsUnknown>, PayjoinError> {
        self.0
            .clone()
            .check_no_inputs_seen_before(|outpoint| {
                is_known(&outpoint.clone().into()).map_err(|e| pdk::Error::Server(Box::new(e)))
            })
            .map_err(|e| e.into())
            .map(|e| Arc::new(e.into()))
    }
}

/// The receiver has not yet identified which outputs belong to the receiver.
///
/// Only accept PSBTs that send us money.
/// Identify those outputs with `identify_receiver_outputs()` to proceed
#[derive(Clone)]
pub struct V2OutputsUnknown(payjoin::receive::v2::OutputsUnknown);

impl From<payjoin::receive::v2::OutputsUnknown> for V2OutputsUnknown {
    fn from(value: payjoin::receive::v2::OutputsUnknown) -> Self {
        Self(value)
    }
}

impl V2OutputsUnknown {
    #[cfg(feature = "uniffi")]
    /// Find which outputs belong to the receiver
    pub fn identify_receiver_outputs(
        &self,
        is_receiver_output: Box<dyn IsScriptOwned>,
    ) -> Result<Arc<V2ProvisionalProposal>, PayjoinError> {
        self.0
            .clone()
            .identify_receiver_outputs(|output_script| {
                is_receiver_output
                    .callback(output_script.to_bytes())
                    .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
            })
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn identify_receiver_outputs(
        &self,
        is_receiver_output: impl Fn(&Vec<u8>) -> Result<bool, PayjoinError>,
    ) -> Result<V2ProvisionalProposal, PayjoinError> {
        self.0
            .clone()
            .identify_receiver_outputs(|input| {
                is_receiver_output(&input.to_bytes())
                    .map_err(|e| payjoin::receive::Error::Server(Box::new(e)))
            })
            .map_err(|e| e.into())
            .map(|e| e.into())
    }
}

pub struct V2WantsOutputs(payjoin::receive::v2::WantsOutputs);

impl From<payjoin::receive::v2::WantsOutputs> for V2WantsOutputs {
    fn from(value: payjoin::receive::v2::WantsOutputs) -> Self {
        Self(value)
    }
}
impl V2WantsOutputs {
    pub fn replace_receiver_outputs(
        &self,
        replacement_outputs: Vec<TxOut>,
        drain_script: &Script,
    ) -> Result<V2WantsOutputs, PayjoinError> {
        self.0
            .clone()
            .replace_receiver_outputs(replacement_outputs.into(), drain_script.clone().into())
    }

    pub fn commit_outputs(&self) -> Result<V2WantsInputs, PayjoinError> {
        self.0.clone().commit_outputs().map_err(|e| e.into())
    }
}

pub struct V2WantsInputs(payjoin::receive::v2::WantsInputs);

impl From<payjoin::receive::v2::WantsInputs> for V2WantsInputs {
    fn from(value: payjoin::receive::v2::WantsInputs) -> Self {
        Self(value)
    }
}

impl V2WantsInputs {
    pub fn contribute_witness_inputs(
        &self,
        replacement_inputs: Vec<(OutPoint, TxOut)>,
    ) -> Result<V2WantsInputs, PayjoinError> {
        self.0.clone().replace_receiver_inputs(replacement_inputs.into())
    }

    pub fn commit_inputs(&self) -> Result<V2ProvisionalProposal, PayjoinError> {
        self.0.clone().commit_inputs().map_err(|e| e.into())
    }
}

pub struct V2ProvisionalProposal(pub Mutex<payjoin::receive::v2::ProvisionalProposal>);

impl From<payjoin::receive::v2::ProvisionalProposal> for V2ProvisionalProposal {
    fn from(value: payjoin::receive::v2::ProvisionalProposal) -> Self {
        Self(Mutex::new(value))
    }
}

/// A mutable checked proposal that the receiver may contribute inputs to to make a payjoin.
impl V2ProvisionalProposal {
    fn mutex_guard(&self) -> MutexGuard<'_, payjoin::receive::v2::ProvisionalProposal> {
        self.0.lock().unwrap()
    }

    pub fn contribute_witness_input(
        &self,
        txo: TxOut,
        outpoint: OutPoint,
    ) -> Result<(), PayjoinError> {
        let txo: payjoin::bitcoin::blockdata::transaction::TxOut = txo.into();
        Ok(self.mutex_guard().contribute_witness_input(txo, outpoint.into()))
    }
    /// Select receiver input such that the payjoin avoids surveillance.
    /// Return the input chosen that has been applied to the Proposal.
    ///
    /// Proper coin selection allows payjoin to resemble ordinary transactions.
    /// To ensure the resemblance, a number of heuristics must be avoided.
    ///
    /// UIH "Unnecessary input heuristic" is one class of them to avoid. We define
    /// UIH1 and UIH2 according to the BlockSci practice
    /// BlockSci UIH1 and UIH2:
    // if min(out) < min(in) then UIH1 else UIH2
    // https://eprint.iacr.org/2022/589.pdf
    pub fn try_preserving_privacy(
        &self,
        candidate_inputs: HashMap<u64, OutPoint>,
    ) -> Result<OutPoint, PayjoinError> {
        let candidate_inputs: HashMap<payjoin::bitcoin::Amount, payjoin::bitcoin::OutPoint> =
            candidate_inputs
                .into_iter()
                .map(|(key, value)| (payjoin::bitcoin::Amount::from_sat(key), value.into()))
                .collect();

        match self.mutex_guard().try_preserving_privacy(candidate_inputs) {
            Ok(e) => Ok(OutPoint { txid: e.txid.to_string(), vout: e.vout }),
            Err(e) => Err(e.into()),
        }
    }
    pub fn is_output_substitution_disabled(&self) -> bool {
        self.mutex_guard().is_output_substitution_disabled()
    }

    #[cfg(not(feature = "uniffi"))]
    ///If output substitution is enabled, replace the receiver’s output script with a new one.
    pub fn try_substitute_receiver_output(
        &self,
        generate_script: impl Fn() -> Result<Vec<u8>, PayjoinError>,
    ) -> Result<(), PayjoinError> {
        self.mutex_guard()
            .try_substitute_receiver_output(|| {
                generate_script()
                    .map(|e| payjoin::bitcoin::ScriptBuf::from_bytes(e))
                    .map_err(|e| payjoin::Error::Server(Box::new(e)))
            })
            .map_err(|e| e.into())
    }
    #[cfg(feature = "uniffi")]
    pub fn try_substitute_receiver_output(
        &self,
        generate_script: Box<dyn GenerateScript>,
    ) -> Result<(), PayjoinError> {
        self.mutex_guard()
            .try_substitute_receiver_output(|| {
                generate_script
                    .callback()
                    .map(|e| payjoin::bitcoin::ScriptBuf::from_bytes(e))
                    .map_err(|e| payjoin::Error::Server(Box::new(e)))
            })
            .map_err(|e| e.into())
    }

    #[cfg(feature = "uniffi")]
    pub fn finalize_proposal(
        &self,
        process_psbt: Box<dyn ProcessPartiallySignedTransaction>,
        min_feerate_sat_per_vb: Option<u64>,
        max_fee_rate_sat_per_vb: u64,
    ) -> Result<Arc<V2PayjoinProposal>, PayjoinError> {
        self.mutex_guard()
            .clone()
            .finalize_proposal(
                |pre_processed| {
                    let processed = process_psbt
                        .callback(pre_processed.to_string())
                        .map(|e| Psbt::from_str(e.as_str()))
                        .map_err(|e| pdk::Error::Server(Box::new(e)))?;
                    match processed {
                        Ok(e) => Ok(e),
                        Err(e) => Err(pdk::Error::Server(Box::new(e))),
                    }
                },
                min_feerate_sat_per_vb.and_then(|x| FeeRate::from_sat_per_vb(x)),
                FeeRate::from_sat_per_vb(max_fee_rate_sat_per_vb),
            )
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn finalize_proposal(
        &self,
        process_psbt: impl Fn(String) -> Result<String, PayjoinError>,
        min_feerate_sat_per_vb: Option<u64>,
        max_feerate_sat_per_vb: u64,
    ) -> Result<Arc<V2PayjoinProposal>, PayjoinError> {
        self.mutex_guard()
            .clone()
            .finalize_proposal(
                |pre_processed| {
                    let processed = process_psbt(pre_processed.to_string())
                        .map(|e| Psbt::from_str(e.as_str()))
                        .map_err(|e| pdk::Error::Server(Box::new(e)))?;
                    match processed {
                        Ok(e) => Ok(e),
                        Err(e) => Err(pdk::Error::Server(Box::new(e))),
                    }
                },
                min_feerate_sat_per_vb.and_then(|x| FeeRate::from_sat_per_vb(x)),
                FeeRate::from_sat_per_vb(max_feerate_sat_per_vb),
            )
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
}

/// A mutable checked proposal that the receiver may contribute inputs to to make a payjoin.

#[derive(Clone)]
pub struct V2PayjoinProposal(pub payjoin::receive::v2::PayjoinProposal);
impl From<V2PayjoinProposal> for payjoin::receive::v2::PayjoinProposal {
    fn from(value: V2PayjoinProposal) -> Self {
        value.0
    }
}
impl From<payjoin::receive::v2::PayjoinProposal> for V2PayjoinProposal {
    fn from(value: payjoin::receive::v2::PayjoinProposal) -> Self {
        Self(value)
    }
}

impl V2PayjoinProposal {
    pub fn utxos_to_be_locked(&self) -> Vec<OutPoint> {
        let mut outpoints: Vec<OutPoint> = Vec::new();
        for e in
            <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
                .utxos_to_be_locked()
        {
            outpoints.push(e.to_owned().into());
        }
        outpoints
    }
    pub fn is_output_substitution_disabled(&self) -> bool {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .is_output_substitution_disabled()
    }
    pub fn owned_vouts(&self) -> Vec<u64> {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .owned_vouts()
            .iter()
            .map(|x| *x as u64)
            .collect()
    }
    pub fn psbt(&self) -> String {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .psbt()
            .clone()
            .to_string()
    }

    pub fn extract_v1_req(&self) -> String {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .extract_v1_req()
    }
    #[cfg(feature = "uniffi")]
    pub fn extract_v2_req(&self) -> Result<RequestResponse, PayjoinError> {
        let (req, res) = self.0.clone().extract_v2_req()?;
        Ok(RequestResponse { request: req.into(), client_response: Arc::new(res.into()) })
    }
    #[cfg(not(feature = "uniffi"))]
    pub fn extract_v2_req(&self) -> Result<(Request, ohttp::ClientResponse), PayjoinError> {
        match self.0.clone().extract_v2_req() {
            Ok(e) => Ok((e.0.into(), e.1)),
            Err(e) => Err(PayjoinError::V2Error { message: e.to_string() }),
        }
    }
    #[cfg(not(feature = "uniffi"))]
    ///Processes the response for the final POST message from the receiver client in the v2 Payjoin protocol.
    ///
    /// This function decapsulates the response using the provided OHTTP context. If the response status is successful, it indicates that the Payjoin proposal has been accepted. Otherwise, it returns an error with the status code.
    ///
    /// After this function is called, the receiver can either wait for the Payjoin transaction to be broadcast or choose to broadcast the original PSBT.
    pub fn process_res(
        &self,
        body: Vec<u8>,
        ctx: ohttp::ClientResponse,
    ) -> Result<(), PayjoinError> {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .process_res(body, ctx)
            .map_err(|e| e.into())
    }
    #[cfg(feature = "uniffi")]
    ///Processes the response for the final POST message from the receiver client in the v2 Payjoin protocol.
    ///
    /// This function decapsulates the response using the provided OHTTP context. If the response status is successful, it indicates that the Payjoin proposal has been accepted. Otherwise, it returns an error with the status code.
    ///
    /// After this function is called, the receiver can either wait for the Payjoin transaction to be broadcast or choose to broadcast the original PSBT.
    pub fn process_res(
        &self,
        res: Vec<u8>,
        ohttp_context: Arc<ClientResponse>,
    ) -> Result<(), PayjoinError> {
        <V2PayjoinProposal as Into<payjoin::receive::v2::PayjoinProposal>>::into(self.clone())
            .process_res(res, ohttp_context.as_ref().into())
            .map_err(|e| e.into())
    }
}
