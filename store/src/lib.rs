//! For storing transferable public keys.
//!
//! The key store stores transferable public keys (TPKs) using an
//! arbitrary label.  Stored keys are automatically updated from
//! remote sources.  This ensures that updates like new subkeys and
//! revocations are discovered in a timely manner.
//!
//! # Security considerations
//!
//! Storing public keys potentially leaks communication partners.
//! Protecting against adversaries inspecting the local storage is out
//! of scope for Sequoia.  Please take the necessary precautions.
//!
//! Sequoia updates keys in compliance with the [network policy] used
//! to create the store.
//!
//! [network policy]: ../sequoia_core/enum.NetworkPolicy.html
//!
//! # Example
//!
//! ```
//! # extern crate openpgp;
//! # extern crate sequoia_core;
//! # extern crate sequoia_store;
//! # use openpgp::Fingerprint;
//! # use sequoia_core::{Context, NetworkPolicy};
//! # use sequoia_store::{Store, Result};
//! # fn main() { f().unwrap(); }
//! # fn f() -> Result<()> {
//! # let ctx = Context::configure("org.sequoia-pgp.demo.store")
//! #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
//! let store = Store::open(&ctx, "default")?;
//!
//! let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
//! let binding = store.add("Mister B.", &fp)?;
//!
//! println!("Binding {:?}", binding.stats()?);
//! // prints:
//! // Binding Stats {
//! //     created: Some(SystemTime { tv_sec: 1513704042, tv_nsec: 0 }),
//! //     updated: None,
//! //     encryption: Stamps { count: 0, first: None, latest: None },
//! //     verification: Stamps { count: 0, first: None, latest: None }
//! // }
//! # Ok(())
//! # }
//! ```

extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
extern crate futures;
extern crate rand;
extern crate rusqlite;
extern crate time;
extern crate tokio_core;
extern crate tokio_io;

use std::cell::RefCell;
use std::fmt;
use std::io;
use std::rc::Rc;
use std::time::{SystemTime, SystemTimeError, Duration, UNIX_EPOCH};

use capnp::capability::Promise;
use capnp_rpc::rpc_twoparty_capnp::Side;
use futures::{Future};
use tokio_core::reactor::Core;

extern crate openpgp;
#[allow(unused_imports)]
#[macro_use]
extern crate sequoia_core;
extern crate sequoia_net;

use openpgp::Fingerprint;
use openpgp::tpk::{self, TPK};
use sequoia_core as core;
use sequoia_core::Context;
use sequoia_net::ipc;

#[allow(dead_code)] mod store_protocol_capnp;
use store_protocol_capnp::node;

/// Macros managing requests and responses.
#[macro_use] mod macros;

/// Storage backend.
mod backend;

/// Returns the service descriptor.
#[doc(hidden)]
pub fn descriptor(c: &Context) -> ipc::Descriptor {
    ipc::Descriptor::new(
        c.home().to_path_buf(),
        c.home().join("S.keystore"),
        c.lib().join("keystore"),
        backend::factory,
    )
}

/// A public key store.
pub struct Store {
    name: String,
    core: Rc<RefCell<Core>>,
    store: node::store::Client,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Store {{ name: {} }}", self.name)
    }
}

impl Store {
    /// Opens a store.
    ///
    /// Opens a store with the given name.  If the store does not
    /// exist, it is created.  Stores are handles for objects
    /// maintained by a background service.  The background service
    /// associates state with this name.
    ///
    /// The store updates TPKs in compliance with the network policy
    /// of the context that created the store in the first place.
    /// Opening the store with a different network policy is
    /// forbidden.
    pub fn open(c: &Context, name: &str) -> Result<Self> {
        let descriptor = descriptor(c);
        let mut core = tokio_core::reactor::Core::new()?;
        let handle = core.handle();

        let mut rpc_system
            = match descriptor.connect(&handle) {
                Ok(r) => r,
                Err(e) => return Err(e.into()),
            };

        let store: node::Client = rpc_system.bootstrap(Side::Server);
        handle.spawn(rpc_system.map_err(|_e| ()));

        let mut request = store.open_request();
        request.get().set_domain(c.domain());
        request.get().set_network_policy(c.network_policy().into());
        request.get().set_ephemeral(c.ephemeral());
        request.get().set_name(name);

        let store = make_request!(&mut core, request)?;
        Ok(Self::new(Rc::new(RefCell::new(core)), name, store))
    }

    fn new(core: Rc<RefCell<Core>>, name: &str, store: node::store::Client) -> Self {
        Store{core: core, name: name.into(), store: store}
    }

    /// Lists all stores with the given prefix.
    pub fn list(c: &Context, domain_prefix: &str) -> Result<StoreIter> {
        let descriptor = descriptor(c);
        let mut core = Core::new()?;
        let handle = core.handle();

        let mut rpc_system
            = match descriptor.connect(&handle) {
                Ok(r) => r,
                Err(e) => return Err(e.into()),
            };

        let node: node::Client = rpc_system.bootstrap(Side::Server);
        handle.spawn(rpc_system.map_err(|_e| ()));

        let mut request = node.iter_request();
        request.get().set_domain_prefix(domain_prefix);
        let iter = make_request!(&mut core, request)?;
        Ok(StoreIter{core: Rc::new(RefCell::new(core)), iter: iter})
    }

    /// Lists all keys in the common key pool.
    pub fn list_keys(c: &Context) -> Result<KeyIter> {
        let descriptor = descriptor(c);
        let mut core = Core::new()?;
        let handle = core.handle();

        let mut rpc_system
            = match descriptor.connect(&handle) {
                Ok(r) => r,
                Err(e) => return Err(e.into()),
            };

        let node: node::Client = rpc_system.bootstrap(Side::Server);
        handle.spawn(rpc_system.map_err(|_e| ()));

        let request = node.iter_keys_request();
        let iter = make_request!(&mut core, request)?;
        Ok(KeyIter{core: Rc::new(RefCell::new(core)), iter: iter})
    }

    /// Adds a key identified by fingerprint to the store.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// let store = Store::open(&ctx, "default")?;
    /// let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
    /// store.add("Mister B.", &fp)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add(&self, label: &str, fingerprint: &Fingerprint) -> Result<Binding> {
        let mut request = self.store.add_request();
        request.get().set_label(label);
        request.get().set_fingerprint(fingerprint.to_hex().as_ref());
        let binding = make_request!(self.core.borrow_mut(), request)?;
        Ok(Binding::new(self.core.clone(), label, binding))
    }

    /// Imports a key into the store.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::tpk::TPK;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// # let tpk = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy.pgp")).unwrap();
    /// let store = Store::open(&ctx, "default")?;
    /// store.import("Testy McTestface", &tpk)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn import(&self, label: &str, tpk: &TPK) -> Result<TPK> {
        let fingerprint = tpk.fingerprint();
        let mut request = self.store.add_request();
        request.get().set_label(label);
        request.get().set_fingerprint(fingerprint.to_hex().as_ref());
        let binding = make_request!(self.core.borrow_mut(), request)?;
        let binding = Binding::new(self.core.clone(), label, binding);
        binding.import(tpk)
    }

    /// Returns the binding for the given label.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// let store = Store::open(&ctx, "default")?;
    /// let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
    /// store.add("Mister B.", &fp)?;
    /// drop(store);
    /// // ...
    /// let store = Store::open(&ctx, "default")?;
    /// let binding = store.lookup("Mister B.")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn lookup(&self, label: &str) -> Result<Binding> {
        let mut request = self.store.lookup_request();
        request.get().set_label(label);
        let binding = make_request!(self.core.borrow_mut(), request)?;
        Ok(Binding::new(self.core.clone(), label, binding))
    }

    /// Deletes this store.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # #[macro_use] extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result, Error};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// let store = Store::open(&ctx, "default")?;
    /// let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
    /// store.add("Mister B.", &fp)?;
    /// store.delete()?;
    /// // ...
    /// let store = Store::open(&ctx, "default")?;
    /// let binding = store.lookup("Mister B.");
    /// assert_match!(Err(Error::NotFound) = binding);
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(self) -> Result<()> {
        let request = self.store.delete_request();
        make_request_map!(self.core.borrow_mut(), request, |_| Ok(()))
    }

    /// Lists all bindings.
    pub fn iter(&self) -> Result<BindingIter> {
        let request = self.store.iter_request();
        let iter = make_request!(self.core.borrow_mut(), request)?;
        Ok(BindingIter{core: self.core.clone(), iter: iter})
    }
}

/// Represents an entry in a Store.
///
/// Stores map labels to TPKs.  A `Binding` represents a pair in this
/// relation.  We make this explicit because we associate metadata
/// with these pairs.
pub struct Binding {
    label: String,
    core: Rc<RefCell<Core>>,
    binding: node::binding::Client,
}

impl fmt::Debug for Binding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Binding {{ label: {} }}", self.label)
    }
}

impl Binding {
    fn new(core: Rc<RefCell<Core>>, label: &str, binding: node::binding::Client) -> Self {
        Binding{label: label.into(), core: core, binding: binding}
    }

    /// Returns stats for this binding.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// let store = Store::open(&ctx, "default")?;
    ///
    /// let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
    /// let binding = store.add("Mister B.", &fp)?;
    ///
    /// println!("Binding {:?}", binding.stats()?);
    /// // prints:
    /// // Binding Stats {
    /// //     created: Some(SystemTime { tv_sec: 1513704042, tv_nsec: 0 }),
    /// //     updated: None,
    /// //     encryption: Stamps { count: 0, first: None, latest: None },
    /// //     verification: Stamps { count: 0, first: None, latest: None }
    /// // }
    /// # Ok(())
    /// # }
    /// ```
    pub fn stats(&self) -> Result<Stats> {
        make_stats_request!(self.core.borrow_mut(),
                            self.binding.stats_request())
    }

    /// Returns the `Key` of this binding.
    pub fn key(&self) -> Result<Key> {
        make_request_map!(self.core.borrow_mut(),
                          self.binding.key_request(),
                          |tpk| Ok(Key::new(self.core.clone(), tpk)))
    }

    /// Returns the `Tpk` of this binding.
    ///
    /// A shortcut for `self.key()?.tpk()`.
    pub fn tpk(&self) -> Result<TPK> {
        self.key()?.tpk()
    }

    /// Updates this binding with the given TPK.
    ///
    /// If the new key `tpk` matches the current key, i.e. they have
    /// the same fingerprint, both keys are merged and normalized.
    /// The returned key contains all packets known to Sequoia, and
    /// should be used instead of `tpk`.
    ///
    /// If the new key does not match the current key, but carries a
    /// valid signature from the current key, it replaces the current
    /// key.  This provides a natural way for key rotations.
    ///
    /// If the new key does not match the current key, and it does not
    /// carry a valid signature from the current key, an
    /// `Error::Conflict` is returned, and you have to resolve the
    /// conflict, either by ignoring the new key, or by using
    /// `TPK::rotate` to force a rotation.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # #[macro_use] extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::tpk::TPK;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result, Error};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// # let old = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy.pgp")).unwrap();
    /// # let new = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy-new.pgp")).unwrap();
    /// # let new_sig = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy-new-with-sig.pgp")).unwrap();
    /// let store = Store::open(&ctx, "default")?;
    /// store.import("Testy McTestface", &old)?;
    /// // later...
    /// let binding = store.lookup("Testy McTestface")?;
    /// let r = binding.import(&new);
    /// assert_match!(Err(Error::Conflict) = r); // no signature from old on new
    /// let r = binding.import(&new_sig)?;
    /// assert_eq!(new.fingerprint(), r.fingerprint());
    /// # Ok(())
    /// # }
    /// ```
    pub fn import(&self, tpk: &TPK) -> Result<TPK> {
        let mut blob = vec![];
        tpk.serialize(&mut blob)?;
        let mut request = self.binding.import_request();
        request.get().set_force(false);
        request.get().set_key(&blob);
        make_request_map!(
            self.core.borrow_mut(),
            request,
            |data| TPK::from_bytes(data).map_err(|e| e.into()))
    }

    /// Forces a keyrotation to the given TPK.
    ///
    /// The current key is replaced with the new key `tpk`, even if
    /// they do not have the same fingerprint.  If a key with the same
    /// fingerprint as `tpk` is already in the store, is merged with
    /// `tpk` and normalized.  The returned key contains all packets
    /// known to Sequoia, and should be used instead of `tpk`.
    ///
    /// Use this function to resolve conflicts returned from
    /// `TPK::import`.  Make sure that you have authenticated `tpk`
    /// properly.  How to do that depends on your thread model.  You
    /// could simply ask Alice to call her communication partner Bob
    /// and confirm that he rotated his keys.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # #[macro_use] extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::tpk::TPK;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result, Error};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// # let old = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy.pgp")).unwrap();
    /// # let new = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy-new.pgp")).unwrap();
    /// let store = Store::open(&ctx, "default")?;
    /// store.import("Testy McTestface", &old)?;
    /// // later...
    /// let binding = store.lookup("Testy McTestface")?;
    /// let r = binding.import(&new);
    /// assert_match!(Err(Error::Conflict) = r); // no signature from old on new
    /// let r = binding.rotate(&new)?;
    /// assert_eq!(new.fingerprint(), r.fingerprint());
    /// # Ok(())
    /// # }
    /// ```
    pub fn rotate(&self, tpk: &TPK) -> Result<TPK> {
        let mut blob = vec![];
        tpk.serialize(&mut blob)?;
        let mut request = self.binding.import_request();
        request.get().set_force(true);
        request.get().set_key(&blob);
        make_request_map!(
            self.core.borrow_mut(),
            request,
            |data| TPK::from_bytes(data).map_err(|e| e.into()))
    }

    /// Deletes this binding.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # #[macro_use] extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result, Error};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// let store = Store::open(&ctx, "default")?;
    /// let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
    /// let binding = store.add("Mister B.", &fp)?;
    /// binding.delete()?;
    /// let binding = store.lookup("Mister B.");
    /// assert_match!(Err(Error::NotFound) = binding);
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(self) -> Result<()> {
        let request = self.binding.delete_request();
        make_request_map!(self.core.borrow_mut(), request, |_| Ok(()))
    }

    fn register_encryption(&self) -> Result<Stats> {
        #![allow(dead_code)]     // XXX use
        make_stats_request!(
            self.core.borrow_mut(),
            self.binding.register_encryption_request())
    }

    fn register_verification(&self) -> Result<Stats> {
        #![allow(dead_code)]     // XXX use
        make_stats_request!(
            self.core.borrow_mut(),
            self.binding.register_verification_request())
    }
}

/// Represents a key in a store.
///
/// A `Key` is a handle to a stored TPK.  We make this explicit
/// because we associate metadata with TPKs.
pub struct Key {
    core: Rc<RefCell<Core>>,
    key: node::key::Client,
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key {{ }}")
    }
}

impl Key {
    fn new(core: Rc<RefCell<Core>>, key: node::key::Client) -> Self {
        Key{core: core, key: key}
    }

    /// Returns the TPK.
    pub fn tpk(&self) -> Result<TPK> {
        make_request_map!(self.core.borrow_mut(),
                          self.key.tpk_request(),
                          |tpk| TPK::from_bytes(tpk).map_err(|e| e.into()))
    }

    /// Returns stats for this key.
    pub fn stats(&self) -> Result<Stats> {
        make_stats_request!(self.core.borrow_mut(),
                            self.key.stats_request())
    }

    /// Updates this stored key with the given TPK.
    ///
    /// If the new key `tpk` matches the current key, i.e. they have
    /// the same fingerprint, both keys are merged and normalized.
    /// The returned key contains all packets known to Sequoia, and
    /// should be used instead of `tpk`.
    ///
    /// If the new key does not match the current key,
    /// `Error::Conflict` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate openpgp;
    /// # #[macro_use] extern crate sequoia_core;
    /// # extern crate sequoia_store;
    /// # use openpgp::Fingerprint;
    /// # use openpgp::tpk::TPK;
    /// # use sequoia_core::{Context, NetworkPolicy};
    /// # use sequoia_store::{Store, Result, Error};
    /// # fn main() { f().unwrap(); }
    /// # fn f() -> Result<()> {
    /// # let ctx = Context::configure("org.sequoia-pgp.demo.store")
    /// #     .network_policy(NetworkPolicy::Offline).ephemeral().build()?;
    /// # let old = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy.pgp")).unwrap();
    /// # let new = TPK::from_bytes(
    /// #     include_bytes!("../../openpgp/tests/data/keys/testy-new.pgp")).unwrap();
    /// let store = Store::open(&ctx, "default")?;
    /// let fp = Fingerprint::from_hex("3E8877C877274692975189F5D03F6F865226FE8B").unwrap();
    /// let binding = store.add("Testy McTestface", &fp)?;
    /// let key = binding.key()?;
    /// let r = key.import(&old)?;
    /// assert_eq!(r.fingerprint(), old.fingerprint());
    /// let r = key.import(&new);
    /// assert_match!(Err(Error::Conflict) = r);
    /// # Ok(())
    /// # }
    /// ```
    pub fn import(&self, tpk: &TPK) -> Result<TPK> {
        let mut blob = vec![];
        tpk.serialize(&mut blob)?;
        let mut request = self.key.import_request();
        request.get().set_key(&blob);
        make_request_map!(
            self.core.borrow_mut(),
            request,
            |data| TPK::from_bytes(data).map_err(|e| e.into()))
    }
}


/// Returns `t` as SystemTime.
fn from_unix(t: i64) -> Option<SystemTime> {
    if t <= 0 {
        None
    } else {
        Some(UNIX_EPOCH + Duration::new(t as u64, 0))
    }
}

/// Statistics about bindings and stored keys.
///
/// We collect some data about binginds and stored keys.  This
/// information can be used to make informed decisions about key
/// transitions.
#[derive(Debug)]
pub struct Stats {
    /// Records the time this item was created.
    pub created: Option<SystemTime>,

    /// Records the time this item was last updated.
    pub updated: Option<SystemTime>,

    /// Result of the latest update.
    pub message: Option<Log>,

    /// Records counters and timestamps of encryptions.
    pub encryption: Stamps,

    /// Records counters and timestamps of verifications.
    pub verification: Stamps,
}

#[derive(Debug)]
pub struct Log {
    pub timestamp: SystemTime,
    pub item: String,
    pub status: ::std::result::Result<String, (String, String)>,
}

impl Log {
    fn new(timestamp: i64, item: &str, message: &str, error: &str) -> Option<Self> {
        let timestamp = from_unix(timestamp)?;
        if message == "" {
            None
        } else {
            if error == "" {
                Some(Log{
                    timestamp: timestamp,
                    item: item.into(),
                    status: Err((message.into(), error.into())),
                })
            } else {
                Some(Log{
                    timestamp: timestamp,
                    item: item.into(),
                    status: Ok(message.into()),
                })
            }
        }
    }

    /// Returns the message without context.
    pub fn short(&self) -> String {
        match self.status {
            Ok(ref m) => m.clone(),
            Err((ref m, ref e)) => format!("{}: {}", m, e),
        }
    }

    /// Returns the message without context.
    pub fn string(&self) -> Result<String> {
        Ok(match self.status {
            Ok(ref m) => format!("{}: {}", format_system_time(&self.timestamp)?, m),
            Err((ref m, ref e)) => format!("{}: {}: {}", format_system_time(&self.timestamp)?, m, e),
        })
    }
}

/// Counter and timestamps.
#[derive(Debug)]
pub struct Stamps {
    /// Counts how many times this has been used.
    pub count: usize,

    /// Records the time when this has been used first.
    pub first:  Option<SystemTime>,

    /// Records the time when this has been used last.
    pub latest: Option<SystemTime>,
}

impl Stamps {
    fn new(count: i64, first: i64, latest: i64) -> Self {
        Stamps {
            count: count as usize,
            first: from_unix(first),
            latest: from_unix(latest),
        }
    }
}

/* Iterators.  */

/// Iterates over stores.
pub struct StoreIter {
    core: Rc<RefCell<Core>>,
    iter: node::store_iter::Client,
}

/// Items returned by `StoreIter`.
#[derive(Debug)]
pub struct StoreIterItem {
    pub domain: String,
    pub name: String,
    pub network_policy: core::NetworkPolicy,
    pub entries: usize,
    pub store: Store,
}

impl Iterator for StoreIter {
    type Item = StoreIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        let request = self.iter.next_request();
        let doit = || {
            make_request_map!(
                self.core.borrow_mut(), request,
                |r: node::store_iter::item::Reader|
                Ok(StoreIterItem{
                    domain: r.get_domain()?.into(),
                    name: r.get_name()?.into(),
                    network_policy: r.get_network_policy()?.into(),
                    entries: r.get_entries() as usize,
                    store: Store::new(self.core.clone(), r.get_name()?, r.get_store()?),
                }))
        };
        doit().ok()
    }
}

/// Iterates over bindings in a store.
pub struct BindingIter {
    core: Rc<RefCell<Core>>,
    iter: node::binding_iter::Client,
}

/// Items returned by `BindingIter`.
#[derive(Debug)]
pub struct BindingIterItem {
    pub label: String,
    pub fingerprint: openpgp::Fingerprint,
    pub binding: Binding,
}

impl Iterator for BindingIter {
    type Item = BindingIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        let request = self.iter.next_request();
        let doit = || {
            make_request_map!(
                self.core.borrow_mut(), request,
                |r: node::binding_iter::item::Reader| {
                    let label = String::from(r.get_label()?);
                    let binding = Binding::new(self.core.clone(), &label, r.get_binding()?);
                    Ok(BindingIterItem{
                        label: label,
                        fingerprint: openpgp::Fingerprint::from_hex(r.get_fingerprint()?).unwrap(),
                        binding: binding,
                    })
                })
        };
        doit().ok()
    }
}

/// Iterates over keys in the common key pool.
pub struct KeyIter {
    core: Rc<RefCell<Core>>,
    iter: node::key_iter::Client,
}

/// Items returned by `KeyIter`.
#[derive(Debug)]
pub struct KeyIterItem {
    pub fingerprint: openpgp::Fingerprint,
    pub bindings: usize,
    pub key: Key,
}

impl Iterator for KeyIter {
    type Item = KeyIterItem;

    fn next(&mut self) -> Option<Self::Item> {
        let request = self.iter.next_request();
        let doit = || {
            make_request_map!(
                self.core.borrow_mut(), request,
                |r: node::key_iter::item::Reader| {
                    Ok(KeyIterItem{
                        fingerprint: openpgp::Fingerprint::from_hex(r.get_fingerprint()?).unwrap(),
                        bindings: r.get_bindings() as usize,
                        key: Key::new(self.core.clone(), r.get_key()?),
                    })
                })
        };
        doit().ok()
    }
}

/// XXX Use the correct time type.
///
/// We should use time::Timespec and get rid of this function.
pub fn format_system_time(t: &SystemTime) -> Result<String> {
    let tm = time::at(time::Timespec::new(t.duration_since(UNIX_EPOCH)?.as_secs() as i64, 0));
    Ok(time::strftime("%F %H:%M", &tm)
       // Only parse errors can happen.
       .unwrap())
}

/* Error handling.  */

/// Results for sequoia-store.
pub type Result<T> = ::std::result::Result<T, Error>;


// Converts from backend errors.
impl From<node::Error> for Error {
    fn from(error: node::Error) -> Self {
        match error {
            node::Error::Unspecified => Error::StoreError,
            node::Error::NotFound => Error::NotFound,
            node::Error::Conflict => Error::Conflict,
            node::Error::SystemError => Error::StoreError,
            node::Error::MalformedKey => Error::MalformedKey,
            node::Error::NetworkPolicyViolationOffline =>
                core::Error::NetworkPolicyViolation(core::NetworkPolicy::Offline).into(),
            node::Error::NetworkPolicyViolationAnonymized =>
                core::Error::NetworkPolicyViolation(core::NetworkPolicy::Anonymized).into(),
            node::Error::NetworkPolicyViolationEncrypted =>
                core::Error::NetworkPolicyViolation(core::NetworkPolicy::Encrypted).into(),
            node::Error::NetworkPolicyViolationInsecure =>
                core::Error::NetworkPolicyViolation(core::NetworkPolicy::Insecure).into(),
        }
    }
}


/// Errors returned from the store.
#[derive(Debug)]
pub enum Error {
    /// A requested key was not found.
    NotFound,
    /// The new key is in conflict with the current key.
    Conflict,
    /// A `sequoia_core::Error` occurred.
    CoreError(sequoia_core::Error),
    /// An `io::Error` occurred.
    IoError(io::Error),
    /// This is a catch-all for unspecified backend errors, and should
    /// go away soon.
    StoreError,
    /// A protocol error occurred.
    ProtocolError,
    /// A TPK is malformed.
    MalformedKey,
    /// A `openpgp::tpk::Error` occurred.
    TpkError(tpk::Error),
    /// A `capnp::Error` occurred.
    RpcError(capnp::Error),
}

impl From<sequoia_core::Error> for Error {
    fn from(error: sequoia_core::Error) -> Self {
        Error::CoreError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<tpk::Error> for Error {
    fn from(error: tpk::Error) -> Self {
        Error::TpkError(error)
    }
}
impl From<capnp::Error> for Error {
    fn from(error: capnp::Error) -> Self {
        Error::RpcError(error)
    }
}

impl From<capnp::NotInSchema> for Error {
    fn from(_: capnp::NotInSchema) -> Self {
        Error::ProtocolError
    }
}

impl From<SystemTimeError> for Error {
    fn from(_: SystemTimeError) -> Self {
        Error::ProtocolError
    }
}

#[cfg(test)]
mod store_test {
    use super::{core, Store, Error, TPK, Fingerprint};

    macro_rules! bytes {
        ( $x:expr ) => { include_bytes!(concat!("../../openpgp/tests/data/keys/", $x)) };
    }

    #[test]
    fn store_network_policy_mismatch() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        // Create store.
        Store::open(&ctx, "default").unwrap();

        let ctx2 = core::Context::configure("org.sequoia-pgp.tests")
            .home(ctx.home())
            .network_policy(core::NetworkPolicy::Encrypted)
            .build().unwrap();
        let store = Store::open(&ctx2, "default");
        assert_match!(Err(Error::CoreError(core::Error::NetworkPolicyViolation(_))) = store);
    }

    #[test]
    fn key_not_found() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx, "default").unwrap();
        let r = store.lookup("I do not exist");
        assert_match!(Err(Error::NotFound) = r);
    }

    #[test]
    fn add_then_import_wrong_key() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx, "default").unwrap();
        let tpk = TPK::from_bytes(bytes!("testy.pgp")).unwrap();
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        let binding = store.add("Mister B.", &fp).unwrap();
        let r = binding.import(&tpk);
        assert_match!(Err(Error::Conflict) = r);
    }


    #[test]
    fn delete_store_twice() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let s0 = Store::open(&ctx, "default").unwrap();
        let s1 = Store::open(&ctx, "default").unwrap();
        s0.delete().unwrap();
        s1.delete().unwrap();
    }

    #[test]
    fn delete_store_then_use() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let s0 = Store::open(&ctx, "default").unwrap();
        let s1 = Store::open(&ctx, "default").unwrap();
        s0.delete().unwrap();
        let binding = s1.lookup("Foobarbaz");
        assert_match!(Err(Error::NotFound) = binding);
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        let binding = s1.add("Mister B.", &fp);
        assert_match!(Err(Error::NotFound) = binding);
    }

    #[test]
    fn delete_binding_twice() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx, "default").unwrap();
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        let b0 = store.add("Mister B.", &fp).unwrap();
        let b1 = store.lookup("Mister B.").unwrap();
        b0.delete().unwrap();
        b1.delete().unwrap();
    }

    #[test]
    fn delete_binding_then_use() {
        let ctx = core::Context::configure("org.sequoia-pgp.tests")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx, "default").unwrap();
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        let b0 = store.add("Mister B.", &fp).unwrap();
        let b1 = store.lookup("Mister B.").unwrap();
        b0.delete().unwrap();
        assert_match!(Err(Error::NotFound) = b1.stats());
        assert_match!(Err(Error::NotFound) = b1.key());
    }

    fn make_some_stores() -> core::Context {
        let ctx0 = core::Context::configure("org.sequoia-pgp.tests.foo")
            .ephemeral()
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx0, "default").unwrap();
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        store.add("Mister B.", &fp).unwrap();
        store.add("B4", &fp).unwrap();

        Store::open(&ctx0, "another store").unwrap();

        let ctx1 = core::Context::configure("org.sequoia-pgp.tests.bar")
            .home(ctx0.home())
            .network_policy(core::NetworkPolicy::Offline)
            .build().unwrap();
        let store = Store::open(&ctx1, "default").unwrap();
        let fp = Fingerprint::from_bytes(b"cccccccccccccccccccc");
        store.add("Mister C.", &fp).unwrap();

        ctx0
    }

    #[test]
    fn store_iterator() {
        let ctx = make_some_stores();
        let mut iter = Store::list(&ctx, "org.sequoia-pgp.tests.f").unwrap();
        let item = iter.next().unwrap();
        assert_eq!(item.domain, "org.sequoia-pgp.tests.foo");
        assert_eq!(item.name, "default");
        assert_eq!(item.network_policy, core::NetworkPolicy::Offline);
        assert_eq!(item.entries, 2);
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        item.store.add("Mister B.", &fp).unwrap();
        let item = iter.next().unwrap();
        assert_eq!(item.domain, "org.sequoia-pgp.tests.foo");
        assert_eq!(item.name, "another store");
        assert_eq!(item.network_policy, core::NetworkPolicy::Offline);
        assert_eq!(item.entries, 0);
        item.store.add("Mister B.", &fp).unwrap();
        assert!(iter.next().is_none());
    }

    #[test]
    fn binding_iterator() {
        let ctx = make_some_stores();
        let store = Store::open(&ctx, "default").unwrap();
        let mut iter = store.iter().unwrap();
        let item = iter.next().unwrap();
        let fp = Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb");
        assert_eq!(item.label, "Mister B.");
        assert_eq!(item.fingerprint, fp);
        item.binding.stats().unwrap();
        let item = iter.next().unwrap();
        assert_eq!(item.label, "B4");
        assert_eq!(item.fingerprint, fp);
        item.binding.stats().unwrap();
        assert!(iter.next().is_none());
    }

    #[test]
    fn key_iterator() {
        let ctx = make_some_stores();
        let mut iter = Store::list_keys(&ctx).unwrap();
        let item = iter.next().unwrap();
        assert_eq!(item.fingerprint, Fingerprint::from_bytes(b"bbbbbbbbbbbbbbbbbbbb"));
        assert_eq!(item.bindings, 2);
        item.key.stats().unwrap();
        let item = iter.next().unwrap();
        assert_eq!(item.fingerprint, Fingerprint::from_bytes(b"cccccccccccccccccccc"));
        assert_eq!(item.bindings, 1);
        item.key.stats().unwrap();
        assert!(iter.next().is_none());
    }
}

