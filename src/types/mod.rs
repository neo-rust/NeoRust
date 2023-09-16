use crypto::ripemd160::{Ripemd160,Digest};
use crypto::sha2::Sha256;
use futures::TryFutureExt;
use hex::FromHexError;
use p256::{PublicKey, SecretKey};
use p256::elliptic_curve::group::prime::PrimeCurveAffine;
use p256::elliptic_curve::sec1::FromEncodedPoint;
use p256::pkcs8::der::{Decode, Encode};
use primitive_types::H160;
use sha2::Digest;
use crate::crypto::wif::Wif;

pub mod call_flags;
pub mod contract_parameter;
pub mod contract_parameter_type;
pub mod vm_state;
pub mod plugin_type;

// Bring EC types into scope

pub type PrivateKey = SecretKey;

pub type Address = String;

pub type Byte = u8;
pub type Bytes = Vec<u8>;

pub trait H160Externsion{

     fn to_string(&self) -> String;

     fn from_slice(slice: &[u8]) -> Result<Self, &'static str> ;

     fn from_hex(hex: &str) -> Result<Self, hex::FromHexError>;
     fn from_address(address: &str) -> Result<Self, &'static str>;

     fn from_public_key(public_key: &PublicKey) -> Self;
    fn to_address(&self) -> String;
    fn to_vec(&self) -> Vec<u8>;
    fn from_script(script: &[u8]) -> Self;
}

impl H160Externsion for H160 {
    fn to_string(&self) -> String {
        bs58::encode(self.0).into_string()
    }

    fn from_slice(slice: &[u8]) -> Result<Self, &'static str> {
        if slice.len() != 20 {
            return Err("Invalid length");
        }

        let mut arr = [0u8; 20];
        arr.copy_from_slice(slice);
        Ok(Self(arr))
    }

    fn from_hex(hex: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(hex)?;
        Ok(Self::from_slice(&bytes))
    }

    fn from_address(address: &str) -> Result<Self, &'static str> {
        let bytes = bs58::decode(address)
            .into_vec()
            .map_err(|_| "Invalid address")?;

        Ok(Self::from_slice(&bytes))
    }

    fn from_public_key(public_key: &PublicKey) -> Self {
        let mut sha = Sha256::new();
        sha.update(public_key.as_bytes());
        let hash = sha.finalize();

        let mut ripemd = Ripemd160::new();
        ripemd.update(&hash);
        let result = ripemd.finalize();

        let mut arr = [0u8; 20];
        arr.copy_from_slice(&result.into_bytes());
        Self(arr)
    }

    fn to_address(&self) -> String {
            bs58::encode(&self.0).into_string()
        }

         fn to_vec(&self) -> Vec<u8> {
            self.0.to_vec().unwrap()
        }

         fn from_script(script: &[u8]) -> Self {
            let mut hasher = Sha256::new();
            hasher.update(script);
            let hash = hasher.finalize();

            let mut ripemd = Ripemd160::new();
            ripemd.update(&hash);
            let result = ripemd.finalize();
            let mut arr = [0u8; 20];
            arr.copy_from_slice(&result.into_bytes());
            Self(arr)
        }
}

trait PublicKeyExtension {
    fn to_address(&self) -> String;
    fn to_vec(&self) -> Vec<u8>;

    fn from_slice(slice: &[u8]) -> Result<Self, &'static str>;
    fn from_hex(hex: &str) -> Result<Self, hex::FromHexError>;
    fn from_private_key(private_key: &SecretKey) -> Self;
}


trait PrivateKeyExtension {
    fn to_address(&self) -> String;
    fn to_vec(&self) -> Vec<u8>;

    fn to_wif(&self) -> String;

    fn from_slice(slice: &[u8]) -> Result<Self, &'static str>;
    fn from_hex(hex: &str) -> Result<Self, hex::FromHexError>;
}

impl PublicKeyExtension for PublicKey{
    fn to_address(&self) -> String {
        H160::from_public_key(self).to_address()
    }

    fn to_vec(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_slice(slice: &[u8]) -> Result<Self, &'static str> {
        if slice.len() != 64 {
            return Err("Invalid length");
        }

        let mut arr = [0u8; 64];
        arr.copy_from_slice(slice);
        Ok(Self::from_encoded_point(&arr).map_err(|_| "Invalid point")?)
    }

    fn from_hex(hex: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(hex)?;
        Ok(Self::from_slice(&bytes)?)
    }

    fn from_private_key(private_key: &SecretKey) -> Self {
       private_key.public_key()
    }
}

impl PrivateKeyExtension for PrivateKey{
    fn to_address(&self) -> String {
        self.public_key().to_address()
    }

    fn to_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn to_wif(&self) -> String {
        self.to_vec().as_slice().to_wif()
    }

    fn from_slice(slice: &[u8]) -> Result<Self, &'static str> {
        if slice.len() != 32 {
            return Err("Invalid length");
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(slice);
        Ok(Self::from_bytes(&arr).map_err(|_| "Invalid point")?)
    }

    fn from_hex(hex: &str) -> Result<Self, FromHexError> {
        let bytes = hex::decode(hex)?;
        Ok(Self::from_slice(&bytes)?)
    }
}