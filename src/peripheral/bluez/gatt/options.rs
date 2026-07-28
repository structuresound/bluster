//! The two things every `ReadValue` / `WriteValue` handler does with what
//! BlueZ hands it, written once instead of four times.
//!
//! Both `characteristic.rs` and `descriptor.rs` declared their own `OptionsMap`
//! and their own `Response` match; the second copy is how the `device` entry
//! came to be read by neither.

// `dbus_tree::MethodErr` and `dbus_crossroads::MethodErr` are both re-exports
// of this one type; characteristic.rs reaches it by the first name and
// descriptor.rs by the second, which is why this module names the source.
use dbus::MethodErr;
use dbus::arg::{RefArg, Variant};
use std::collections::HashMap;

use super::super::constants::{
    BLUEZ_ERROR_FAILED,
    BLUEZ_ERROR_INVALIDOFFSET,
    BLUEZ_ERROR_NOTAUTHORIZED,
    BLUEZ_ERROR_NOTPERMITTED,
    BLUEZ_ERROR_NOTSUPPORTED,
};
use crate::gatt::event::{Peer, Response};

pub type OptionsMap = HashMap<String, Variant<Box<dyn RefArg>>>;

/// The central this request came from, out of BlueZ's options dict.
///
/// BlueZ documents `device` as an object path (`org.bluez.GattCharacteristic1`,
/// `ReadValue` / `WriteValue`). `RefArg::as_str` yields it for both the
/// `ObjectPath` and `String` variants, so a bus that sends either is read the
/// same. Absent on a peer that predates the option, and on some
/// bluetoothd-internal reads — hence `Option`, not a fabricated placeholder.
pub fn peer(options: &OptionsMap) -> Option<Peer> {
    options.get("device").and_then(|v| v.as_str()).map(|s| Peer(s.to_owned()))
}

/// `u16` out of the options dict, or `default`.
pub fn u16_opt(options: &OptionsMap, key: &str, default: u16) -> u16 {
    options.get(key).and_then(RefArg::as_u64).map(|v| v as u16).unwrap_or(default)
}

/// Turn an application's answer into the D-Bus reply BlueZ turns into an ATT
/// response.
///
/// The mapping is why [`Response`] has the variants it has: each names an error
/// BlueZ has a wire name for. The handlers used to collapse every non-`Success`
/// into `org.bluez.Error.Failed`, so a read-only characteristic could not tell a
/// writing central that the write was not permitted — the refusal reached the
/// air as a generic failure.
pub fn into_reply(response: Response) -> Result<Vec<u8>, MethodErr> {
    match response {
        Response::Success(value) => Ok(value),
        Response::InvalidOffset => Err(MethodErr::from((BLUEZ_ERROR_INVALIDOFFSET, ""))),
        Response::NotPermitted => Err(MethodErr::from((BLUEZ_ERROR_NOTPERMITTED, ""))),
        Response::NotAuthorized => Err(MethodErr::from((BLUEZ_ERROR_NOTAUTHORIZED, ""))),
        Response::NotSupported => Err(MethodErr::from((BLUEZ_ERROR_NOTSUPPORTED, ""))),
        // BlueZ has no name for an invalid attribute length, and an unlikely
        // error is what `Failed` means. Both are honest here.
        Response::InvalidAttributeLength | Response::UnlikelyError => {
            Err(MethodErr::from((BLUEZ_ERROR_FAILED, "")))
        }
    }
}
