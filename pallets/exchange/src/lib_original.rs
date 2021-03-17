#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, traits::Get,
};
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use primitives::{Balance, TokenPair, TokenSymbol};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
pub use serde::{Deserialize, Serialize};

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        Balance = Balance,
        CurrencyId = TokenSymbol,
    >;
    type InitialShares: Get<Balance>;
    type ExchangeFeeRate: Get<(u32, u32)>;
    type ExchangeAccountId: Get<Self::AccountId>;
    type AllowedExchangePairs: Get<Vec<TokenPair>>;
}
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PairPool<T: Trait> {
    fee_rate: (u32, u32),
    token_a_pool: Balance,
    token_b_pool: Balance,
    invariant: Balance,
    total_shares: Balance,
    shares: BTreeMap<T::AccountId, Balance>,
}

impl<T: Trait> Default for PairPool<T> {
    fn default() -> Self {
        Self {
            fee_rate: (3, 1000),
            token_a_pool: Zero::zero(),
            token_b_pool: Zero::zero(),
            invariant: Zero::zero(),
            total_shares: Zero::zero(),
            shares: BTreeMap::new(),
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as ExchangeModule {
        pub PairStructs get(fn pair_structs): map hasher(twox_64_concat) TokenPair =>  PairPool<T>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        CurrencyId = TokenSymbol,
        Shares = Balance,
        Balance = Balance,
    {
        Initialized(AccountId, CurrencyId, CurrencyId, Shares),
        ABSwapped(CurrencyId, Balance, CurrencyId, Balance),
        BASwapped(CurrencyId, Balance, CurrencyId, Balance),
        Invested(AccountId, CurrencyId, CurrencyId, Shares),
        Devested(AccountId, CurrencyId, CurrencyId, Shares),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PairNotAllowed,
        LowTokenAmount,
        TokenAExists,
        TotalShareIsNotNull,
        PairNotExists,
        SamePair,
        LowAmountReceived,
        InsufficientPool,
        InvalidShares,
        InsufficientShares,
        DoesNotOwnShare,
        LowAmountOut,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        const AllowedExchangePairs: Vec<TokenPair> = T::AllowedExchangePairs::get();
        const GetExchangeFee: (u32, u32) = T::ExchangeFeeRate::get();

        #[weight = 10_000]
        pub fn initialize_exchange(
            origin,
            token_a: TokenSymbol,
            #[compact] token_a_amount: Balance,
            token_b: TokenSymbol,
            #[compact] token_b_amount: Balance,
        ) -> dispatch::DispatchResult{
            let sender = ensure_signed(origin)?;
            ensure!(token_a_amount > Zero::zero(), Error::<T>::LowTokenAmount);
            ensure!(token_b_amount > Zero::zero(), Error::<T>::LowTokenAmount);
            let _pair = TokenPair::new(token_a, token_b);
            ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);

            let token_pool = Self::pair_structs(_pair);
            ensure!(token_pool.invariant == Zero::zero(), Error::<T>::TokenAExists);
            ensure!(token_pool.total_shares == Zero::zero(), Error::<T>::TotalShareIsNotNull);

            let (_token_a_amount, _token_b_amount) = if token_a == _pair.0 {
                (token_a_amount, token_b_amount)
            } else {
                (token_b_amount, token_a_amount)
            };

            T::Currency::transfer(token_a, &sender, &T::ExchangeAccountId::get(), _token_a_amount)?;
            T::Currency::transfer(token_b, &sender, &T::ExchangeAccountId::get(), _token_b_amount)?;

            let mut shares_map = BTreeMap::new();
            shares_map.insert(sender.clone(), T::InitialShares::get());
            let pool = PairPool::<T> {
                fee_rate: T::ExchangeFeeRate::get(),
                token_a_pool: _token_a_amount,
                token_b_pool: _token_b_amount,
                invariant: _token_a_amount * _token_b_amount,
                total_shares: T::InitialShares::get(),
                shares: shares_map,
            };
            PairStructs::<T>::insert(_pair, pool);

            Self::deposit_event(RawEvent::Initialized(sender, token_a, token_b, T::InitialShares::get()));
            Ok(())
        }

        #[weight = 10_000]
        pub fn a_to_b_swap(
            origin,
            token_a: TokenSymbol,
            #[compact] token_a_amount: Balance,
            token_b: TokenSymbol,
            #[compact] min_token_b_received: Balance,
            receiver: T::AccountId,// new point
        ) -> dispatch::DispatchResult{
            let sender = ensure_signed(origin)?;
            ensure!(token_a != token_b, Error::<T>::SamePair);
            let _pair = TokenPair::new(token_a, token_b);
            ensure!(PairStructs::<T>::contains_key(_pair), Error::<T>::PairNotExists);
            ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);

            let pool = Self::pair_structs(_pair);

            let (_token_a_pool, _token_b_pool) = if token_a == _pair.0 {
                (pool.token_a_pool, pool.token_b_pool)
            } else {
                (pool.token_b_pool, pool.token_a_pool)
            };

            let fee:Balance = token_a_amount * (pool.fee_rate.0/ pool.fee_rate.1) as Balance;
            let new_token_a_pool = _token_a_pool + token_a_amount;
            let temp_token_a_pool = new_token_a_pool - fee;
            let new_token_b_pool = pool.invariant / temp_token_a_pool;
            let tokens_out_b = _token_b_pool - new_token_b_pool;
            ensure!(tokens_out_b >= min_token_b_received, Error::<T>::LowAmountReceived);
            ensure!(tokens_out_b <= _token_b_pool, Error::<T>::InsufficientPool);

            T::Currency::transfer(token_a, &sender, &T::ExchangeAccountId::get(), token_a_amount)?;
            T::Currency::transfer(token_b, &T::ExchangeAccountId::get(), &receiver, tokens_out_b)?;

            <PairStructs<T>>::mutate(_pair, |pool| {
                pool.token_a_pool = new_token_a_pool;
                pool.token_b_pool = new_token_b_pool;
                pool.invariant = pool.token_a_pool * pool.token_b_pool;
            });

            Self::deposit_event(RawEvent::ABSwapped(token_a, token_a_amount, token_b, tokens_out_b));
            Ok(())
        }

        #[weight = 10_000]
        pub fn b_to_a_swap(
            origin,
            token_a: TokenSymbol,
            #[compact] min_token_a_received: Balance,
            token_b: TokenSymbol,
            #[compact] token_b_amount: Balance,
            receiver: T::AccountId,
        ) -> dispatch::DispatchResult{
            let sender = ensure_signed(origin)?;
            ensure!(token_a != token_b, Error::<T>::SamePair);
            let _pair = TokenPair::new(token_a, token_b);
            ensure!(PairStructs::<T>::contains_key(_pair), Error::<T>::PairNotExists);
            ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);

            let pool = Self::pair_structs(_pair);

            let (_token_a_pool, _token_b_pool) = if token_a == _pair.0 {
                (pool.token_a_pool, pool.token_b_pool)
            } else {
                (pool.token_b_pool, pool.token_a_pool)
            };

            let fee:Balance = token_b_amount * (pool.fee_rate.0/ pool.fee_rate.1) as Balance; //might have a issue
            let new_token_b_pool = _token_b_pool + token_b_amount;
            let temp_token_b_pool = new_token_b_pool - fee;
            let new_token_a_pool = pool.invariant / temp_token_b_pool;
            let tokens_out_a = _token_a_pool - new_token_a_pool;
            ensure!(tokens_out_a >= min_token_a_received, Error::<T>::LowAmountReceived);
            ensure!(tokens_out_a <= _token_a_pool, Error::<T>::InsufficientPool);

            T::Currency::transfer(token_b, &sender, &T::ExchangeAccountId::get(), token_b_amount)?;
            T::Currency::transfer(token_a, &T::ExchangeAccountId::get(), &receiver, tokens_out_a)?;

            <PairStructs<T>>::mutate(_pair, |pool| {
                pool.token_a_pool = new_token_a_pool;
                pool.token_b_pool = new_token_b_pool;
                pool.invariant = pool.token_a_pool * pool.token_b_pool;
            });

            Self::deposit_event(RawEvent::BASwapped(token_b, token_b_amount, token_a, tokens_out_a));
            Ok(())
        }

        #[weight = 10_000]
        pub fn invest_liquidity(
            origin,
            token_a: TokenSymbol,
            token_b: TokenSymbol,
            shares: Balance,
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(token_a != token_b, Error::<T>::SamePair);
            let _pair = TokenPair::new(token_a, token_b);
            ensure!(PairStructs::<T>::contains_key(_pair), Error::<T>::PairNotExists);
            ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);

            let pool = Self::pair_structs(_pair);

            let (_token_a_pool, _token_b_pool) = if token_a == _pair.0 {
                (pool.token_a_pool, pool.token_b_pool)
            } else {
                (pool.token_b_pool, pool.token_a_pool)
            };

            let token_b_per_share = _token_b_pool/pool.total_shares;
            let token_b_cost = token_b_per_share * shares;
            let token_a_per_share = _token_a_pool/pool.total_shares;
            let token_a_cost = token_a_per_share * shares;

            T::Currency::transfer(token_a, &sender, &T::ExchangeAccountId::get(), token_a_cost)?;
            T::Currency::transfer(token_b, &sender, &T::ExchangeAccountId::get(), token_b_cost)?;

            <PairStructs<T>>::mutate(_pair, |pool|{
                let updated_shares = if let Some(prev_shares) = pool.shares.get(&sender){
                    *prev_shares + shares
                } else {
                    shares
                };
                pool.shares.insert(sender.clone(), updated_shares);
                pool.total_shares += shares;
                pool.token_b_pool += token_b_cost;
                pool.token_a_pool += token_a_cost;
                pool.invariant = pool.token_b_pool * pool.token_a_pool;
            });

            Self::deposit_event(RawEvent::Invested(sender, token_a, token_b, shares));
            Ok(())

        }

        #[weight = 10_000]
        pub fn devest_liquidity(
            origin,
            token_a: TokenSymbol,
            token_b: TokenSymbol,
            shares_burned: Balance,
            min_token_a_received: Balance,
            min_token_b_received: Balance
        ) -> dispatch::DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(token_a != token_b, Error::<T>::SamePair);
            let _pair = TokenPair::new(token_a, token_b);
            ensure!(PairStructs::<T>::contains_key(_pair), Error::<T>::PairNotExists);
            ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);

            ensure!(shares_burned > Zero::zero(), Error::<T>::InvalidShares);
            let pool = Self::pair_structs(_pair);

            let (_token_a_pool, _token_b_pool) = if token_a == _pair.0 {
                (pool.token_a_pool, pool.token_b_pool)
            } else {
                (pool.token_b_pool, pool.token_a_pool)
            };

            if let Some(shares) = pool.shares.get(&sender){
                ensure!(*shares >= shares_burned, Error::<T>::InsufficientShares);
            } else {
                return Err(Error::<T>::DoesNotOwnShare.into());
            }

            let token_b_per_share = _token_b_pool/pool.total_shares;
            let token_b_cost = token_b_per_share * shares_burned;
            let token_a_per_share = _token_a_pool/pool.total_shares;
            let token_a_cost = token_a_per_share * shares_burned;

            ensure!(token_b_cost >= min_token_b_received, Error::<T>::LowAmountOut);
            ensure!(token_a_cost >= min_token_a_received, Error::<T>::LowAmountOut);

            T::Currency::transfer(token_a, &T::ExchangeAccountId::get(), &sender, token_a_cost)?;
            T::Currency::transfer(token_b, &T::ExchangeAccountId::get(), &sender, token_b_cost)?;

            <PairStructs<T>>::mutate(_pair, |pool| {
                if let Some(share) = pool.shares.get_mut(&sender){
                    *share -= shares_burned;
                }
                pool.total_shares -= shares_burned;
                pool.token_b_pool -= token_b_cost;
                pool.token_a_pool -= token_a_cost;
                if pool.total_shares == Zero::zero(){
                    pool.invariant = Zero::zero();
                } else {
                    pool.invariant = pool.token_a_pool * pool.token_b_pool;
                }
            });

            Self::deposit_event(RawEvent::Devested(sender, token_a, token_b, shares_burned));
            Ok(())

        }
    }
}