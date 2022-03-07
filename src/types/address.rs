use rand::Rng;
use ring::digest;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};

// 20-byte address
#[derive(Eq, PartialEq, Serialize, Deserialize, Clone, Hash, Default, Copy)]
pub struct Address([u8; 20]);

impl std::convert::From<&[u8; 20]> for Address {
    fn from(input: &[u8; 20]) -> Address {
        let mut buffer: [u8; 20] = [0; 20];
        buffer[..].copy_from_slice(input);
        Address(buffer)
    }
}

impl std::convert::From<[u8; 20]> for Address {
    fn from(input: [u8; 20]) -> Address {
        Address(input)
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let start = if let Some(precision) = f.precision() {
            if precision >= 40 {
                0
            } else {
                20 - precision / 2
            }
        } else {
            0
        };
        for byte_idx in start..20 {
            write!(f, "{:>02x}", &self.0[byte_idx])?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:>02x}{:>02x}..{:>02x}{:>02x}",
            &self.0[0], &self.0[1], &self.0[18], &self.0[19]
        )
    }
}

impl Address {
    pub fn from_public_key_bytes(bytes: &[u8]) -> Address {
        let hash: Vec<u8> = digest::digest(&digest::SHA256, bytes).as_ref().to_vec();
        let n = hash.len();
        let tmp = &hash[(n - 20)..n];
        let mut addr: [u8; 20] = [0; 20];
        let mut i = 0;
        for num in tmp.iter() {
            addr[i] = *num;
            i += 1;
        }

        Address(addr)
    }
    pub fn address_from_public_key(public_key: <Ed25519KeyPair as KeyPair>::PublicKey) -> Address {
        let public_key_hash = digest::digest(&digest::SHA256, public_key.as_ref());

        let mut raw_address: [u8; 20] = [0; 20];
        raw_address.copy_from_slice(&(public_key_hash.as_ref()[12..32]));
        Address(raw_address)
    }
}

pub fn generate_random_address() -> Address {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    let mut raw_bytes = [0; 20];
    raw_bytes.copy_from_slice(&random_bytes);
    (&raw_bytes).into()
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod test {
    use super::Address;

    #[test]
    fn from_a_test_key() {
        let test_key = hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d");
        let addr = Address::from_public_key_bytes(&test_key);
        let correct_addr: Address = hex!("1851a0eae0060a132cf0f64a0ffaea248de6cba0").into();
        assert_eq!(addr, correct_addr);
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // take the last 20 bytes, we get "1851a0eae0060a132cf0f64a0ffaea248de6cba0"
    }
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST
