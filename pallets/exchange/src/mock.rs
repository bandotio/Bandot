#![cfg(test)]

use super::*; //use crate::{Module, Trait};
use frame_support::{impl_outer_origin, parameter_types};
use primitives::{Amount, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

impl_outer_origin! {
    pub enum Origin for Test {}
}

pub type AccountId = u128;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const BUSD: TokenSymbol = TokenSymbol::BUSD;
pub const DOT: TokenSymbol = TokenSymbol::DOT;
pub const BUSD_DOT_PAIR: TokenPair = TokenPair(BUSD, DOT);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: u32 = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Test {
    type BaseCallFilter = ();
    type AccountId = AccountId;
    type Call = ();
    type Lookup = IdentityLookup<Self::AccountId>;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Header = Header;
    type Event = ();
    type Origin = Origin;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = ();
    type SystemWeightInfo = ();
}

impl orml_tokens::Trait for Test {
    type Event = ();
    type Balance = Balance;
    type Amount = Amount;
    type CurrencyId = TokenSymbol;
    type OnReceived = ();
    type WeightInfo = ();
}
pub type Tokens = orml_tokens::Module<Test>;

parameter_types! {
    pub const ExchangeAccountId: ModuleId = ModuleId(*b"bandotex");
    //pub const InitialShares: Balance = 100;
    pub const ExchangeFeeRate: (u32, u32) = (3, 1000);
    pub AllowedExchangePairs:Vec<TokenPair> = vec![
        TokenPair::new(TokenSymbol::BUSD, TokenSymbol::DOT),
    ];
}

impl Trait for Test {
    type Event = ();
    type Currency = Tokens;
    //type InitialShares = InitialShares;
    type ExchangeFeeRate = ExchangeFeeRate;
    type ExchangeAccountId = ExchangeAccountId;
    type AllowedExchangePairs = AllowedExchangePairs;
}

pub type ExchangeModule = Module<Test>;

pub struct ExtBuilder {
    endowed_accounts: Vec<(AccountId, TokenSymbol, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            endowed_accounts: vec![(ALICE, BUSD, 1_000_000u128), (ALICE, DOT, 1_000_000u128)],
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        orml_tokens::GenesisConfig::<Test> {
            endowed_accounts: self.endowed_accounts,
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        storage.into()
    }
}