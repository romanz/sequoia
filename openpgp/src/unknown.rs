use Tag;
use Unknown;
use Packet;

impl Unknown {
    /// Returns a new `Unknown` packet.
    pub fn new(tag: Tag) -> Self {
        Unknown {
            common: Default::default(),
            tag: tag,
        }
    }

    /// Gets the unknown packet's tag.
    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Sets the unknown packet's tag.
    pub fn set_tag(&mut self, tag: Tag) {
        self.tag = tag;
    }

    /// Sets the packet's contents.
    ///
    /// This is the raw packet content not include the CTB and length
    /// information, and not encoded using something like OpenPGP's
    /// partial body encoding.
    pub fn body(&self) -> Option<&[u8]> {
        self.common.body.as_ref().map(|b| b.as_slice())
    }

    /// Sets the packet's contents.
    ///
    /// This is the raw packet content not include the CTB and length
    /// information, and not encoded using something like OpenPGP's
    /// partial body encoding.
    pub fn set_body(&mut self, data: Vec<u8>) {
        self.common.body = Some(data);
    }

    /// Convert the `Unknown` struct to a `Packet`.
    pub fn to_packet(self) -> Packet {
        Packet::Unknown(self)
    }
}
