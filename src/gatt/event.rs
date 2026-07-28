use futures::channel::{mpsc, oneshot};

pub type EventSender = mpsc::Sender<Event>;
pub type ResponseSender = oneshot::Sender<Response>;

/// Which central a request came from.
///
/// On BlueZ this is the `device` entry of the options map BlueZ passes to
/// `ReadValue` / `WriteValue` — a D-Bus object path such as
/// `/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF`, unique per connected central for
/// the life of the connection.
///
/// It is a **connection scope, not an identity**: LE privacy means the address
/// the path encodes may be a resolvable private address that rotates, and a
/// reconnect may produce a different path for the same peer. Use it to key
/// per-connection state (reassembly buffers, per-central response routing);
/// never persist it and never authorize on it.
///
/// `None` where the backend cannot supply one — see [`Event`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Peer(pub String);

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// # Which events carry a [`Peer`], and why the notify pair does not
///
/// `ReadRequest` and `WriteRequest` do, because BlueZ's `ReadValue` /
/// `WriteValue` take an options dict that contains the central's device object
/// path. `NotifySubscribe` and `NotifyUnsubscribe` do **not**, and this is not
/// an omission that can be fixed here: `org.bluez.GattCharacteristic1`'s
/// `StartNotify` and `StopNotify` take *no arguments at all*, so bluetoothd
/// never tells the application which central subscribed. The per-device notify
/// surface BlueZ does offer is the separate `AcquireNotify(dict) -> (fd, mtu)`
/// method, which this crate does not implement.
///
/// Consequence for a caller: a subscriber map keyed on anything is a map with
/// one bucket. Design a per-connection control channel around
/// write-with-response rather than around notify, or implement `AcquireNotify`.
#[derive(Debug)]
pub enum Event {
    ReadRequest(ReadRequest),
    WriteRequest(WriteRequest),
    NotifySubscribe(NotifySubscribe),
    NotifyUnsubscribe,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ReadRequest {
    pub offset: u16,
    pub response: ResponseSender,
    pub mtu: u16,
    /// The central that issued the read. `None` when the backend could not
    /// supply one — see [`Peer`].
    pub peer: Option<Peer>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct WriteRequest {
    pub data: Vec<u8>,
    pub offset: u16,
    pub without_response: bool,
    pub response: ResponseSender,
    /// The central that issued the write. `None` when the backend could not
    /// supply one — see [`Peer`].
    pub peer: Option<Peer>,
}

#[derive(Debug, Clone)]
pub struct NotifySubscribe {
    pub notification: mpsc::Sender<Vec<u8>>,
}

/// The answer an application gives to a read or write request.
///
/// The variants are ATT error codes (Bluetooth Core, Vol 3 Part F §3.4.1.1)
/// restricted to the ones BlueZ has a D-Bus error name for. A reason BlueZ
/// cannot name has nowhere to go, so it must be reported as
/// [`Response::UnlikelyError`] rather than invented.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Response {
    Success(Vec<u8>),
    /// ATT 0x07 — `org.bluez.Error.InvalidOffset`.
    InvalidOffset,
    /// ATT 0x0D — no distinct BlueZ name; goes out as `org.bluez.Error.Failed`.
    InvalidAttributeLength,
    /// ATT 0x02 / 0x03 — `org.bluez.Error.NotPermitted`.
    NotPermitted,
    /// ATT 0x05 / 0x08 — `org.bluez.Error.NotAuthorized`.
    NotAuthorized,
    /// ATT 0x06 — `org.bluez.Error.NotSupported`.
    NotSupported,
    /// ATT 0x0E — `org.bluez.Error.Failed`. The honest catch-all.
    UnlikelyError,
}
