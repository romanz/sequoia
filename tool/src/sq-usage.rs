//! A command-line frontend for Sequoia.
//!
//! # Usage
//!
//! ```text
//! Sequoia is an implementation of OpenPGP.  This is a command-line frontend.
//!
//! USAGE:
//!     sq [OPTIONS] [SUBCOMMAND]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -d, --domain <DOMAIN>            Sets the domain to use
//!     -p, --policy <NETWORK-POLICY>    Sets the network policy to use
//!     -s, --store <STORE>              Sets the store to use (default: 'default')
//!
//! SUBCOMMANDS:
//!     dearmor      Removes ASCII Armor from a file
//!     decrypt      Decrypts an OpenPGP message
//!     dump         Lists OpenPGP packets
//!     enarmor      Applies ASCII Armor to a file
//!     encrypt      Encrypts a message
//!     help         Prints this message or the help of the given subcommand(s)
//!     keyserver    Interacts with keyservers
//!     list         Lists key stores and known keys
//!     store        Interacts with key stores
//! ```
//!
//! ## Subcommand dearmor
//!
//! ```text
//! Removes ASCII Armor from a file
//!
//! USAGE:
//!     sq dearmor [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>     Sets the input file to use
//!     -o, --output <FILE>    Sets the output file to use
//! ```
//!
//! ## Subcommand decrypt
//!
//! ```text
//! Decrypts an OpenPGP message
//!
//! USAGE:
//!     sq decrypt [FLAGS] [OPTIONS]
//!
//! FLAGS:
//!         --dump       Print a packet dump to stderr
//!     -h, --help       Prints help information
//!     -x, --hex        Print a hexdump (implies --dump)
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>     Sets the input file to use
//!     -o, --output <FILE>    Sets the output file to use
//! ```
//!
//! ## Subcommand dump
//!
//! ```text
//! Lists OpenPGP packets
//!
//! USAGE:
//!     sq dump [FLAGS] [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -x, --hex        Print a hexdump
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>     Sets the input file to use
//!     -o, --output <FILE>    Sets the output file to use
//! ```
//!
//! ## Subcommand enarmor
//!
//! ```text
//! Applies ASCII Armor to a file
//!
//! USAGE:
//!     sq enarmor [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>     Sets the input file to use
//!     -o, --output <FILE>    Sets the output file to use
//! ```
//!
//! ## Subcommand encrypt
//!
//! ```text
//! Encrypts a message
//!
//! USAGE:
//!     sq encrypt [FLAGS] [OPTIONS]
//!
//! FLAGS:
//!     -A, --armor        Write armored data to file
//!     -h, --help         Prints help information
//!     -s, --symmetric    Encrypt with a password (can be given multiple times)
//!     -V, --version      Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>            Sets the input file to use
//!     -o, --output <FILE>           Sets the output file to use
//!     -r, --recipient <LABEL>...    Recipient to encrypt for (can be given multiple times)
//! ```
//!
//! ## Subcommand keyserver
//!
//! ```text
//! Interacts with keyservers
//!
//! USAGE:
//!     sq keyserver [OPTIONS] [SUBCOMMAND]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -s, --server <URI>    Sets the keyserver to use
//!
//! SUBCOMMANDS:
//!     get     Retrieves a key
//!     help    Prints this message or the help of the given subcommand(s)
//!     send    Sends a key
//! ```
//!
//! ### Subcommand keyserver get
//!
//! ```text
//! Retrieves a key
//!
//! USAGE:
//!     sq keyserver get [FLAGS] [OPTIONS] <KEYID>
//!
//! FLAGS:
//!     -A, --armor      Write armored data to file
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -o, --output <FILE>    Sets the output file to use
//!
//! ARGS:
//!     <KEYID>    ID of the key to retrieve
//! ```
//!
//! ### Subcommand keyserver send
//!
//! ```text
//! Sends a key
//!
//! USAGE:
//!     sq keyserver send [OPTIONS]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>    Sets the input file to use
//! ```
//!
//! ## Subcommand list
//!
//! ```text
//! Lists key stores and known keys
//!
//! USAGE:
//!     sq list [SUBCOMMAND]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! SUBCOMMANDS:
//!     bindings    Lists all bindings in all key stores
//!     help        Prints this message or the help of the given subcommand(s)
//!     keys        Lists all keys in the common key pool
//!     log         Lists the server log
//!     stores      Lists key stores
//! ```
//!
//! ### Subcommand list bindings
//!
//! ```text
//! Lists all bindings in all key stores
//!
//! USAGE:
//!     sq list bindings [PREFIX]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <PREFIX>    List only bindings from stores with the given domain prefix
//! ```
//!
//! ### Subcommand list keys
//!
//! ```text
//! Lists all keys in the common key pool
//!
//! USAGE:
//!     sq list keys
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//! ```
//!
//! ### Subcommand list log
//!
//! ```text
//! Lists the server log
//!
//! USAGE:
//!     sq list log
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//! ```
//!
//! ### Subcommand list stores
//!
//! ```text
//! Lists key stores
//!
//! USAGE:
//!     sq list stores [PREFIX]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <PREFIX>    List only stores with the given domain prefix
//! ```
//!
//! ## Subcommand store
//!
//! ```text
//! Interacts with key stores
//!
//! USAGE:
//!     sq store [SUBCOMMAND]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! SUBCOMMANDS:
//!     add       Add a key identified by fingerprint
//!     delete    Deletes bindings or stores
//!     export    Exports a key
//!     help      Prints this message or the help of the given subcommand(s)
//!     import    Imports a key
//!     list      Lists keys in the store
//!     log       Lists the keystore log
//!     stats     Get stats for the given label
//! ```
//!
//! ### Subcommand store add
//!
//! ```text
//! Add a key identified by fingerprint
//!
//! USAGE:
//!     sq store add <LABEL> <FINGERPRINT>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <LABEL>          Label to use
//!     <FINGERPRINT>    Key to add
//! ```
//!
//! ### Subcommand store delete
//!
//! ```text
//! Deletes bindings or stores
//!
//! USAGE:
//!     sq store delete [FLAGS] [LABEL]
//!
//! FLAGS:
//!     -h, --help         Prints help information
//!         --the-store    Delete the whole store
//!     -V, --version      Prints version information
//!
//! ARGS:
//!     <LABEL>    Delete binding with this label
//! ```
//!
//! ### Subcommand store export
//!
//! ```text
//! Exports a key
//!
//! USAGE:
//!     sq store export [FLAGS] [OPTIONS] <LABEL>
//!
//! FLAGS:
//!     -A, --armor      Write armored data to file
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -o, --output <FILE>    Sets the output file to use
//!
//! ARGS:
//!     <LABEL>    Label to use
//! ```
//!
//! ### Subcommand store import
//!
//! ```text
//! Imports a key
//!
//! USAGE:
//!     sq store import [OPTIONS] <LABEL>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! OPTIONS:
//!     -i, --input <FILE>    Sets the input file to use
//!
//! ARGS:
//!     <LABEL>    Label to use
//! ```
//!
//! ### Subcommand store list
//!
//! ```text
//! Lists keys in the store
//!
//! USAGE:
//!     sq store list
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//! ```
//!
//! ### Subcommand store log
//!
//! ```text
//! Lists the keystore log
//!
//! USAGE:
//!     sq store log [LABEL]
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <LABEL>    List messages related to this label
//! ```
//!
//! ### Subcommand store stats
//!
//! ```text
//! Get stats for the given label
//!
//! USAGE:
//!     sq store stats <LABEL>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <LABEL>    Label to use
//! ```

include!("sq.rs");