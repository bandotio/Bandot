#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
pub use serde::{Deserialize, Serialize};

pub type Balance = u128;
pub type Amount = i128;

pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000_000_000
pub const MILLICENTS: Balance = CENTS / 1000; // 10_000_000_000_000
pub const MICROCENTS: Balance = MILLICENTS / 1000; // 10_000_000_000

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenSymbol {
    BDT = 0,
    BUSD = 1,
    DOT = 2,
}
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TokenPair(pub TokenSymbol, pub TokenSymbol);

impl TokenPair {
    pub fn new(token_id_a: TokenSymbol, token_id_b: TokenSymbol) -> Self {
        if token_id_a > token_id_b {
            TokenPair(token_id_b, token_id_a)
        } else {
            TokenPair(token_id_a, token_id_b)
        }
    }
}
