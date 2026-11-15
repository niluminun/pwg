use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::{Digest, Sha3_256, Shake256, Shake256Reader};

pub fn sha3_256(data: &[u8]) -> [u8; 32] {
    Sha3_256::digest(data).into()
}

/// Endless byte stream from SHAKE256, read in order.
pub struct ByteStream {
    reader: Shake256Reader,
}

pub fn shake256(seed: &[u8]) -> ByteStream {
    let mut xof = Shake256::default();
    xof.update(seed);
    ByteStream {
        reader: xof.finalize_xof(),
    }
}

impl ByteStream {
    pub fn read(&mut self, buf: &mut [u8]) {
        self.reader.read(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha3_256_known_answer() {
        assert_eq!(
            hex(&sha3_256(b"abc")),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
    }

    #[test]
    fn shake256_known_answer() {
        let mut s = shake256(b"abc");
        let mut out = [0u8; 8];
        s.read(&mut out);
        assert_eq!(hex(&out), "483366601360a877");
    }
}
