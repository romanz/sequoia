/// This program demonstrates how to wrap a stream into a literal data
/// packet.
///
/// It is also used to generate test vectors for the armor subsystem.

use std::env;
use std::io;

extern crate openpgp;
use openpgp::armor;
use openpgp::constants::DataFormat;
use openpgp::serialize::stream::{wrap, LiteralWriter};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 1 {
        panic!("A simple filter wrapping data into a literal data packet.\n\n\
                Usage: {} <input >output\n", args[0]);
    }

    // Compose a writer stack corresponding to the output format and
    // packet structure we want.  First, we want the output to be
    // ASCII armored.
    let sink = armor::Writer::new(io::stdout(), armor::Kind::Message, &[][..])
        .expect("Failed to create armored writer.");

    // Then, create a literal writer to wrap the data in a literal
    // message packet.
    let mut literal = LiteralWriter::new(wrap(sink), DataFormat::Binary,
                                         None, None)
        .expect("Failed to create literal writer");

    // Finally, just copy all the data.
    io::copy(&mut io::stdin(), &mut literal)
        .expect("Failed to sign data");

    // Teardown the stack to ensure all the data is written.
    literal.finalize()
        .expect("Failed to write data");
}