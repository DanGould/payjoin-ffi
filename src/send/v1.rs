use std::io::Cursor;
use std::str::FromStr;
use std::sync::Arc;

pub use payjoin::send as pdk;

use crate::error::PayjoinError;
use crate::send::Context;
use crate::types::Request;
use crate::uri::{PjUri, Url};

///Builder for sender-side payjoin parameters
///
///These parameters define how client wants to handle Payjoin.
#[derive(Clone)]
pub struct SenderBuilder(pdk::SenderBuilder<'static>);

impl From<pdk::SenderBuilder<'static>> for SenderBuilder {
    fn from(value: pdk::SenderBuilder<'static>) -> Self {
        Self(value)
    }
}

impl SenderBuilder {
    //TODO: Replicate all functions like this & remove duplicate code
    /// Prepare an HTTP request and request context to process the response
    ///
    /// An HTTP client will own the Request data while Context sticks around so
    /// a `(Request, Context)` tuple is returned from `SenderBuilder::build()`
    /// to keep them separated.
    pub fn from_psbt_and_uri(
        psbt: String,
        #[cfg(not(feature = "uniffi"))] uri: PjUri,
        #[cfg(feature = "uniffi")] uri: Arc<PjUri>,
    ) -> Result<Self, PayjoinError> {
        let psbt = payjoin::bitcoin::psbt::Psbt::from_str(psbt.as_str())?;
        #[cfg(feature = "uniffi")]
        let uri: PjUri = (*uri).clone();
        pdk::SenderBuilder::from_psbt_and_uri(psbt, uri.into())
            .map(|e| e.into())
            .map_err(|e| e.into())
    }

    /// Disable output substitution even if the receiver didn't.
    ///
    /// This forbids receiver switching output or decreasing amount.
    /// It is generally **not** recommended to set this as it may prevent the receiver from
    /// doing advanced operations such as opening LN channels and it also guarantees the
    /// receiver will **not** reward the sender with a discount.
    pub fn always_disable_output_substitution(&self, disable: bool) -> Arc<Self> {
        Arc::new(self.0.clone().always_disable_output_substitution(disable).into())
    }
    // Calculate the recommended fee contribution for an Original PSBT.
    //
    // BIP 78 recommends contributing `originalPSBTFeeRate * vsize(sender_input_type)`.
    // The minfeerate parameter is set if the contribution is available in change.
    //
    // This method fails if no recommendation can be made or if the PSBT is malformed.
    pub fn build_recommended(&self, min_fee_rate: u64) -> Result<Arc<Sender>, PayjoinError> {
        self.0
            .clone()
            .build_recommended(payjoin::bitcoin::FeeRate::from_sat_per_kwu(min_fee_rate))
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    /// Offer the receiver contribution to pay for his input.
    ///
    /// These parameters will allow the receiver to take `max_fee_contribution` from given change
    /// output to pay for additional inputs. The recommended fee is `size_of_one_input * fee_rate`.
    ///
    /// `change_index` specifies which output can be used to pay fee. If `None` is provided, then
    /// the output is auto-detected unless the supplied transaction has more than two outputs.
    ///
    /// `clamp_fee_contribution` decreases fee contribution instead of erroring.
    ///
    /// If this option is true and a transaction with change amount lower than fee
    /// contribution is provided then instead of returning error the fee contribution will
    /// be just lowered in the request to match the change amount.
    pub fn build_with_additional_fee(
        &self,
        max_fee_contribution: u64,
        change_index: Option<u8>,
        min_fee_rate: u64,
        clamp_fee_contribution: bool,
    ) -> Result<Arc<Sender>, PayjoinError> {
        self.0
            .clone()
            .build_with_additional_fee(
                payjoin::bitcoin::Amount::from_sat(max_fee_contribution),
                change_index.map(|x| x as usize),
                payjoin::bitcoin::FeeRate::from_sat_per_kwu(min_fee_rate),
                clamp_fee_contribution,
            )
            .map(|e| Arc::new(e.into()))
            .map_err(|e| e.into())
    }
    /// Perform Payjoin without incentivizing the payee to cooperate.
    ///
    /// While it's generally better to offer some contribution some users may wish not to.
    /// This function disables contribution.
    pub fn build_non_incentivizing(&self, min_fee_rate: u64) -> Result<Arc<Sender>, PayjoinError> {
        match self
            .0
            .clone()
            .build_non_incentivizing(payjoin::bitcoin::FeeRate::from_sat_per_kwu(min_fee_rate))
        {
            Ok(e) => Ok(Arc::new(e.into())),
            Err(e) => Err(e.into()),
        }
    }
}
#[derive(Clone)]
pub struct Sender(payjoin::send::Sender);

impl From<payjoin::send::Sender> for Sender {
    fn from(value: payjoin::send::Sender) -> Self {
        Self(value)
    }
}

#[derive(Clone)]
pub struct RequestContext {
    pub request: Request,
    pub context: Arc<Context>,
}

impl Sender {
    /// Extract serialized Request and Context from a Payjoin Proposal.
    ///
    /// In order to support polling, this may need to be called many times to be encrypted with
    /// new unique nonces to make independent OHTTP requests.
    ///
    /// The `ohttp_proxy` merely passes the encrypted payload to the ohttp gateway of the receiver
    pub fn extract_highest_version(
        &self,
        ohttp_proxy_url: Arc<Url>,
    ) -> Result<RequestContext, PayjoinError> {
        match self.0.clone().extract_highest_version((*ohttp_proxy_url).clone().into()) {
            Ok(e) => Ok(RequestContext { request: e.0.into(), context: Arc::new(e.1.into()) }),
            Err(e) => Err(e.into()),
        }
    }
}
///Data required for validation of response.
/// This type is used to process the response. Get it from SenderBuilder's build methods. Then you only need to call .process_response() on it to continue BIP78 flow.
#[derive(Clone)]
pub struct V1Context(Arc<payjoin::send::V1Context>);
impl From<payjoin::send::V1Context> for V1Context {
    fn from(value: payjoin::send::V1Context) -> Self {
        Self(Arc::new(value))
    }
}

impl V1Context {
    ///Decodes and validates the response.
    /// Call this method with response from receiver to continue BIP78 flow. If the response is valid you will get appropriate PSBT that you should sign and broadcast.
    pub fn process_response(&self, response: Vec<u8>) -> Result<String, PayjoinError> {
        let mut decoder = Cursor::new(response);
        self.0.clone().process_response(&mut decoder).map(|e| e.to_string()).map_err(|e| e.into())
    }
}
