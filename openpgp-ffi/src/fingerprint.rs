//! Fingerprints.
//!
//! A fingerprint uniquely identifies a public key.  For more details
//! about how a fingerprint is generated, see [Section 12.2 of RFC
//! 4880].
//!
//!   [Section 12.2 of RFC 4880]: https://tools.ietf.org/html/rfc4880#section-12.2
//!
//! Wraps [`sequoia-openpgp::Fingerprint`].
//!
//! [`sequoia-openpgp::Fingerprint`]: ../../sequoia_openpgp/enum.Fingerprint.html

use std::slice;
use libc::{uint8_t, c_char, size_t};

extern crate sequoia_openpgp as openpgp;
use self::openpgp::KeyID;
use Maybe;

/// Holds a fingerprint.
///
/// A fingerprint uniquely identifies a public key.  For more details
/// about how a fingerprint is generated, see [Section 12.2 of RFC
/// 4880].
///
///   [Section 12.2 of RFC 4880]: https://tools.ietf.org/html/rfc4880#section-12.2
///
/// Wraps [`sequoia-openpgp::Fingerprint`].
///
/// [`sequoia-openpgp::Fingerprint`]: ../../sequoia_openpgp/enum.Fingerprint.html
#[::ffi_wrapper_type(prefix = "pgp_",
                     derive = "Clone, Debug, Display, Hash, PartialEq")]
pub struct Fingerprint(openpgp::Fingerprint);

/// Reads a binary fingerprint.
#[::ffi_catch_abort] #[no_mangle] pub extern "system"
fn pgp_fingerprint_from_bytes(buf: *const uint8_t,
                              len: size_t)
                              -> *mut openpgp::Fingerprint {
    assert!(!buf.is_null());
    let buf = unsafe {
        slice::from_raw_parts(buf, len as usize)
    };
    openpgp::Fingerprint::from_bytes(buf).move_into_raw()
}

/// Reads a hexadecimal fingerprint.
///
/// # Example
///
/// ```c
/// #include <assert.h>
/// #include <stdlib.h>
/// #include <string.h>
/// #include <sequoia/openpgp.h>
///
/// pgp_fingerprint_t fp =
///     pgp_fingerprint_from_hex ("D2F2C5D45BE9FDE6A4EE0AAF31855247603831FD");
///
/// char *pretty = pgp_fingerprint_to_string (fp);
/// assert (strcmp (pretty,
///                 "D2F2 C5D4 5BE9 FDE6 A4EE  0AAF 3185 5247 6038 31FD") == 0);
///
/// free (pretty);
/// pgp_fingerprint_free (fp);
/// ```
#[::ffi_catch_abort] #[no_mangle] pub extern "system"
fn pgp_fingerprint_from_hex(hex: *const c_char)
                            -> Maybe<openpgp::Fingerprint> {
    let hex = ffi_param_cstr!(hex).to_string_lossy();
    openpgp::Fingerprint::from_hex(&hex).ok().move_into_raw()
}

/// Returns a reference to the raw Fingerprint.
///
/// This returns a reference to the internal buffer that is valid as
/// long as the fingerprint is.
#[::ffi_catch_abort] #[no_mangle] pub extern "system"
fn pgp_fingerprint_as_bytes(fp: *const openpgp::Fingerprint,
                            fp_len: Option<&mut size_t>)
                            -> *const uint8_t {
    let fp = fp.ref_raw();
    if let Some(p) = fp_len {
        *p = fp.as_slice().len();
    }
    fp.as_slice().as_ptr()
}

/// Converts the fingerprint to a hexadecimal number.
#[::ffi_catch_abort] #[no_mangle] pub extern "system"
fn pgp_fingerprint_to_hex(fp: *const openpgp::Fingerprint)
                          -> *mut c_char {
    ffi_return_string!(fp.ref_raw().to_hex())
}

/// Converts the fingerprint to a key ID.
#[::ffi_catch_abort] #[no_mangle] pub extern "system"
fn pgp_fingerprint_to_keyid(fp: *const openpgp::Fingerprint)
                            -> *mut KeyID {
    fp.ref_raw().to_keyid().move_into_raw()
}
