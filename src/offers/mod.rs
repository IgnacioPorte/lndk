use std::{error::Error, fmt::Display};

use lightning::{
    ln::channelmanager::PaymentId,
    offers::{merkle::SignError, parse::Bolt12SemanticError},
};
use tonic_lnd::tonic::Status;

use crate::lndkrpc::{LndkError, LndkErrorCode};

mod client_impls;
pub mod handler;
mod lnd_requests;
mod parse;

pub(crate) use lnd_requests::connect_to_peer;
pub use lnd_requests::create_reply_path;
pub use parse::{decode, get_destination, validate_amount};

#[derive(Debug)]
/// OfferError is an error that occurs during the process of paying an offer.
pub enum OfferError {
    /// AlreadyProcessing indicates that we're already trying to make a payment with the same id.
    AlreadyProcessing(PaymentId),
    /// BuildUIRFailure indicates a failure to build the unsigned invoice request.
    BuildUIRFailure(Bolt12SemanticError),
    /// SignError indicates a failure to sign the invoice request.
    SignError(SignError),
    /// DeriveKeyFailure indicates a failure to derive key for signing the invoice request.
    DeriveKeyFailure(Status),
    /// User provided an invalid amount.
    InvalidAmount(String),
    /// Invalid currency contained in the offer.
    InvalidCurrency,
    /// Unable to connect to peer.
    PeerConnectError(Status),
    /// No node address.
    NodeAddressNotFound,
    /// Cannot list peers.
    ListPeersFailure(Status),
    /// Failure to build a reply path.
    BuildBlindedPathFailure,
    /// Unable to find or send to payment route.
    RouteFailure(Status),
    /// Failed to track payment.
    TrackFailure(Status),
    /// Failed to send payment.
    PaymentFailure,
    /// Failed to receive an invoice back from offer creator before the timeout.
    InvoiceTimeout(u32),
    /// Failed to find introduction node for blinded path.
    IntroductionNodeNotFound,
    /// Cannot fetch channel info.
    GetChannelInfo(Status),
}

impl Display for OfferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OfferError::AlreadyProcessing(id) => {
                write!(
                    f,
                    "We're already trying to pay for a payment with this id {id}"
                )
            }
            OfferError::BuildUIRFailure(e) => write!(f, "Error building invoice request: {e:?}"),
            OfferError::SignError(e) => write!(f, "Error signing invoice request: {e:?}"),
            OfferError::DeriveKeyFailure(e) => write!(f, "Error signing invoice request: {e:?}"),
            OfferError::InvalidAmount(e) => write!(f, "User provided an invalid amount: {e:?}"),
            OfferError::InvalidCurrency => write!(
                f,
                "LNDK doesn't yet support offer currencies other than bitcoin"
            ),
            OfferError::PeerConnectError(e) => write!(f, "Error connecting to peer: {e:?}"),
            OfferError::NodeAddressNotFound => write!(f, "Couldn't get node address"),
            OfferError::ListPeersFailure(e) => write!(f, "Error listing peers: {e:?}"),
            OfferError::BuildBlindedPathFailure => write!(f, "Error building blinded path"),
            OfferError::RouteFailure(e) => write!(f, "Error routing payment: {e:?}"),
            OfferError::TrackFailure(e) => write!(f, "Error tracking payment: {e:?}"),
            OfferError::PaymentFailure => write!(f, "Failed to send payment"),
            OfferError::InvoiceTimeout(e) => write!(f, "Did not receive invoice in {e:?} seconds."),
            OfferError::IntroductionNodeNotFound => write!(f, "Could not find introduction node."),
            OfferError::GetChannelInfo(e) => write!(f, "Could not fetch channel info: {e:?}"),
        }
    }
}

impl Error for OfferError {}

impl OfferError {
    pub fn to_error_code(&self) -> LndkErrorCode {
        match self {
            OfferError::AlreadyProcessing(_) => LndkErrorCode::AlreadyProcessing,
            OfferError::BuildUIRFailure(_) => LndkErrorCode::BuildInvoiceRequestFailed,
            OfferError::SignError(_) => LndkErrorCode::SignFailed,
            OfferError::DeriveKeyFailure(_) => LndkErrorCode::DeriveKeyFailed,
            OfferError::InvalidAmount(_) => LndkErrorCode::InvalidAmount,
            OfferError::InvalidCurrency => LndkErrorCode::InvalidCurrency,
            OfferError::PeerConnectError(_) => LndkErrorCode::PeerConnectionFailed,
            OfferError::NodeAddressNotFound => LndkErrorCode::NodeAddressNotFound,
            OfferError::ListPeersFailure(_) => LndkErrorCode::ListPeersFailed,
            OfferError::BuildBlindedPathFailure => LndkErrorCode::BlindedPathBuildFailed,
            OfferError::RouteFailure(_) => LndkErrorCode::RouteFailed,
            OfferError::TrackFailure(_) => LndkErrorCode::TrackPaymentFailed,
            OfferError::PaymentFailure => LndkErrorCode::PaymentFailed,
            OfferError::InvoiceTimeout(_) => LndkErrorCode::InvoiceTimeout,
            OfferError::IntroductionNodeNotFound => LndkErrorCode::IntroductionNodeNotFound,
            OfferError::GetChannelInfo(_) => LndkErrorCode::ChannelInfoFailed,
        }
    }

    pub fn get_details(&self) -> Option<String> {
        match self {
            OfferError::AlreadyProcessing(id) => Some(format!("payment_id: {}", id)),
            OfferError::BuildUIRFailure(e) => Some(format!("semantic_error: {e:?}")),
            OfferError::SignError(e) => Some(format!("sign_error: {e:?}")),
            OfferError::DeriveKeyFailure(e) => Some(format!("status_code: {}", e.code())),
            OfferError::InvalidAmount(e) => Some(e.clone()),
            OfferError::PeerConnectError(e) => Some(format!("status_code: {}", e.code())),
            OfferError::ListPeersFailure(e) => Some(format!("status_code: {}", e.code())),
            OfferError::RouteFailure(e) => Some(format!("status_code: {}", e.code())),
            OfferError::TrackFailure(e) => Some(format!("status_code: {}", e.code())),
            OfferError::InvoiceTimeout(timeout) => Some(format!("timeout_seconds: {}", timeout)),
            OfferError::GetChannelInfo(e) => Some(format!("status_code: {}", e.code())),
            _ => None,
        }
    }

    pub fn to_lndk_error(&self) -> LndkError {
        LndkError {
            code: self.to_error_code() as i32,
            message: self.to_string(),
            details: self.get_details(),
        }
    }

    pub fn to_grpc_status_code(&self) -> tonic_lnd::tonic::Code {
        match self {
            OfferError::InvalidAmount(_)
            | OfferError::InvalidCurrency
            | OfferError::AlreadyProcessing(_) => tonic_lnd::tonic::Code::InvalidArgument,

            OfferError::PeerConnectError(_)
            | OfferError::NodeAddressNotFound
            | OfferError::ListPeersFailure(_) => tonic_lnd::tonic::Code::Unavailable,

            OfferError::RouteFailure(_)
            | OfferError::PaymentFailure
            | OfferError::IntroductionNodeNotFound => tonic_lnd::tonic::Code::FailedPrecondition,

            OfferError::InvoiceTimeout(_) => tonic_lnd::tonic::Code::DeadlineExceeded,

            _ => tonic_lnd::tonic::Code::Internal,
        }
    }

    pub fn to_status(&self) -> Status {
        Status::new(self.to_grpc_status_code(), self.to_string())
    }
}
