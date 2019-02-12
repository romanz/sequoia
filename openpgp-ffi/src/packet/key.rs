//! Public key, public subkey, private key and private subkey packets.
//!
//! See [Section 5.5 of RFC 4880] for details.
//!
//!   [Section 5.5 of RFC 4880]: https://tools.ietf.org/html/rfc4880#section-5.5

use libc::c_int;

extern crate sequoia_openpgp as openpgp;
use self::openpgp::{
    packet,
};
use super::super::fingerprint::Fingerprint;
use super::super::keyid::KeyID;

use MoveIntoRaw;

/// Clones the key.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_clone(key: *const packet::Key)
                                      -> *mut packet::Key {
    let key = ffi_param_ref!(key);
    box_raw!(key.clone())
}

/// Computes and returns the key's fingerprint as per Section 12.2
/// of RFC 4880.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_fingerprint(key: *const packet::Key)
                                            -> *mut Fingerprint {
    let key = ffi_param_ref!(key);
    key.fingerprint().move_into_raw()
}

/// Computes and returns the key's key ID as per Section 12.2 of RFC
/// 4880.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_keyid(key: *const packet::Key)
                                      -> *mut KeyID {
    let key = ffi_param_ref!(key);
    key.keyid().move_into_raw()
}

/// Returns the key's creation time.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_creation_time(key: *const packet::Key)
    -> u32
{
    let key = ffi_param_ref!(key);
    let ct = key.creation_time();

    ct.to_timespec().sec as u32
}

/// Returns the key's public key algorithm.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_public_key_algo(key: *const packet::Key)
    -> c_int
{
    let key = ffi_param_ref!(key);
    let pk_algo : u8 = key.pk_algo().into();
    pk_algo as c_int
}

/// Returns the public key's size in bits.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_public_key_bits(key: *const packet::Key)
    -> c_int
{
    use self::openpgp::crypto::mpis::PublicKey::*;

    let key = ffi_param_ref!(key);
    match key.mpis() {
        RSA { e: _, n } => n.bits as c_int,
        DSA { p: _, q: _, g: _, y } => y.bits as c_int,
        Elgamal { p: _, g: _, y } => y.bits as c_int,
        EdDSA { curve: _, q } => q.bits as c_int,
        ECDSA { curve: _, q } =>  q.bits as c_int,
        ECDH { curve: _, q, hash: _, sym: _ } =>  q.bits as c_int,
        Unknown { mpis: _, rest: _ } => 0,
    }
}

/// Creates a new key pair from a Key packet with an unencrypted
/// secret key.
///
/// # Errors
///
/// Fails if the secret key is missing, or encrypted.
#[::sequoia_ffi_macros::extern_fn] #[no_mangle]
pub extern "system" fn pgp_key_into_key_pair(errp: Option<&mut *mut ::error::Error>,
                                              key: *mut packet::Key)
                                              -> *mut self::openpgp::crypto::KeyPair
{
    ffi_make_fry_from_errp!(errp);
    let key = ffi_param_move!(key);
    ffi_try_box!(key.into_keypair())
}
