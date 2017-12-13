use std::io;
use std::cmp;

use super::*;

/// A `BufferedReaderLimitor` limits the amount of data that can be
/// read from a `BufferedReader`.
#[derive(Debug)]
pub struct BufferedReaderLimitor<T: BufferedReader> {
    reader: T,
    limit: u64,
}

impl<T: BufferedReader> BufferedReaderLimitor<T> {
    pub fn new(reader: T, limit: u64) -> BufferedReaderLimitor<T> {
        BufferedReaderLimitor {
            reader: reader,
            limit: limit,
        }
    }
}

impl<T: BufferedReader> io::Read for BufferedReaderLimitor<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let len = cmp::min(self.limit, buf.len() as u64) as usize;
        return self.reader.read(&mut buf[0..len]);
    }
}

impl<T: BufferedReader> BufferedReader for BufferedReaderLimitor<T> {
    /// Return the buffer.  Ensure that it contains at least `amount`
    /// bytes.
    fn data(&mut self, amount: usize) -> Result<&[u8], io::Error> {
        let amount = cmp::min(amount as u64, self.limit) as usize;
        let result = self.reader.data(amount);
        match result {
            Ok(ref buffer) =>
                if buffer.len() as u64 > self.limit {
                    return Ok(&buffer[0..self.limit as usize]);
                } else {
                    return Ok(buffer);
                },
            Err(err) => return Err(err),
        }
    }

    fn consume(&mut self, amount: usize) -> &[u8] {
        assert!(amount as u64 <= self.limit);
        self.limit -= amount as u64;
        let data = self.reader.consume(amount);
        return &data[..cmp::min(self.limit + amount as u64, data.len() as u64) as usize];
    }

    fn data_consume(&mut self, amount: usize) -> Result<&[u8], io::Error> {
        let amount = cmp::min(amount as u64, self.limit) as usize;
        let result = self.reader.data_consume(amount);
        if let Ok(ref buffer) = result {
            self.limit -= amount as u64;
            return Ok(&buffer[
                ..cmp::min(buffer.len() as u64, self.limit + amount as u64) as usize]);
        }
        return result;
    }

    fn data_consume_hard(&mut self, amount: usize) -> Result<&[u8], io::Error> {
        if amount as u64 > self.limit {
            return Err(Error::new(ErrorKind::UnexpectedEof, "EOF"));
        }
        let result = self.reader.data_consume_hard(amount);
        if let Ok(ref buffer) = result {
            self.limit -= amount as u64;
            return Ok(&buffer[
                ..cmp::min(buffer.len() as u64, self.limit + amount as u64) as usize]);
        }
        return result;
    }

    fn into_inner<'b>(self: Box<Self>) -> Option<Box<BufferedReader + 'b>>
            where Self: 'b {
        Some(Box::new(self.reader))
    }
}

#[test]
fn buffered_reader_limitor_test() {
    let data : &[u8] = b"01234567890123456789";

    /* Add a single limitor.  */
    {
        let mut bio : Box<BufferedReader>
            = Box::new(BufferedReaderMemory::new(data));

        bio = {
            let mut bio2 = Box::new(BufferedReaderLimitor::new(bio, 5));
            {
                let result = bio2.data(5).unwrap();
                assert_eq!(result.len(), 5);
                assert_eq!(result, &b"01234"[..]);
            }
            bio2.consume(5);
            {
                let result = bio2.data(1).unwrap();
                assert_eq!(result.len(), 0);
                assert_eq!(result, &b""[..]);
            }

            bio2.into_inner().unwrap()
        };

        {
            {
                let result = bio.data(15).unwrap();
                assert_eq!(result.len(), 15);
                assert_eq!(result, &b"567890123456789"[..]);
            }
            bio.consume(15);
            {
                let result = bio.data(1).unwrap();
                assert_eq!(result.len(), 0);
                assert_eq!(result, &b""[..]);
            }
        }
    }

    /* Try with two limitors where the first one imposes the real
     * limit.  */
    {
        let mut bio : Box<BufferedReader>
            = Box::new(BufferedReaderMemory::new(data));

        bio = {
            let bio2 : Box<BufferedReader>
                = Box::new(BufferedReaderLimitor::new(bio, 5));
            // We limit to 15 bytes, but bio2 will still limit us to 5
            // bytes.
            let mut bio3 : Box<BufferedReader>
                = Box::new(BufferedReaderLimitor::new(bio2, 15));
            {
                let result = bio3.data(100).unwrap();
                assert_eq!(result.len(), 5);
                assert_eq!(result, &b"01234"[..]);
            }
            bio3.consume(5);
            {
                let result = bio3.data(1).unwrap();
                assert_eq!(result.len(), 0);
                assert_eq!(result, &b""[..]);
            }

            bio3.into_inner().unwrap().into_inner().unwrap()
        };

        {
            {
                let result = bio.data(15).unwrap();
                assert_eq!(result.len(), 15);
                assert_eq!(result, &b"567890123456789"[..]);
            }
            bio.consume(15);
            {
                let result = bio.data(1).unwrap();
                assert_eq!(result.len(), 0);
                assert_eq!(result, &b""[..]);
            }
        }
    }
}