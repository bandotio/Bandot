#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, traits::Get,
};
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use orml_utilities::with_transaction_result;
use primitives::{Balance, TokenPair, TokenSymbol, DOLLARS};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

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
            token_a_pool: Balance::default(),
            token_b_pool: Balance::default(),
            invariant: Balance::default(),
            total_shares: Balance::default(),
            shares: BTreeMap::new(),
        }
    }
}

impl<T: Trait> PairPool<T> {
    pub fn ensure_launch(
        &self,
        token_a_amount: Balance,
        token_b_amount: Balance,
    ) -> dispatch::DispatchResult {
        ensure!(token_a_amount > Zero::zero(), Error::<T>::LowTokenAmount);
        ensure!(token_b_amount > Zero::zero(), Error::<T>::LowTokenAmount);
        ensure!(self.invariant == Zero::zero(), Error::<T>::TokenpoolExists);
        ensure!(
            self.total_shares == Zero::zero(),
            Error::<T>::TotalShareIsNotNull
        );
        Ok(())
    }

    pub fn initialize_new(
        token_a_amount: Balance,
        token_b_amount: Balance,
        sender: T::AccountId,
    ) -> Self {
        let mut shares_map = BTreeMap::new();
        shares_map.insert(
            sender.clone(),
            T::InitialShares::get().saturating_mul(DOLLARS),
        );
        Self {
            fee_rate: T::ExchangeFeeRate::get(),
            token_a_pool: token_a_amount,
            token_b_pool: token_b_amount,
            invariant: token_a_amount
                .checked_div(DOLLARS)
                .unwrap()
                .saturating_mul(token_b_amount),
            total_shares: T::InitialShares::get().saturating_mul(DOLLARS),
            shares: shares_map,
        }
    }

    pub fn calculate_a_to_b(
        &self,
        _token_a_pool: Balance,
        _token_b_pool: Balance,
        token_a_amount: Balance,
    ) -> (Balance, Balance, Balance) {
        let new_token_a_pool: Balance = _token_a_pool.saturating_add(token_a_amount);
        let temp_token_a_pool: Balance = (_token_a_pool.saturating_mul(self.fee_rate.1.into()))
            .saturating_add(
                token_a_amount
                    .saturating_mul(self.fee_rate.1.saturating_sub(self.fee_rate.0).into()),
            );
        let new_token_b_pool: Balance = self
            .invariant
            .saturating_mul(self.fee_rate.1.into())
            .checked_div(temp_token_a_pool)
            .unwrap()
            .into();
        let tokens_out_b: Balance = _token_b_pool
            .checked_div(DOLLARS)
            .unwrap()
            .saturating_sub(new_token_b_pool);
        let ab: Balance = _token_b_pool.checked_div(DOLLARS).unwrap();
        (new_token_a_pool, new_token_b_pool, tokens_out_b)
    }
    pub fn calculate_b_to_a(
        &self,
        _token_a_pool: Balance,
        _token_b_pool: Balance,
        token_b_amount: Balance,
    ) -> (Balance, Balance, Balance) {
        let new_token_b_pool: Balance = _token_b_pool.saturating_add(token_b_amount);
        let temp_token_b_pool: Balance = (_token_b_pool.saturating_mul(self.fee_rate.1.into()))
            .saturating_add(
                token_b_amount
                    .saturating_mul(self.fee_rate.1.saturating_sub(self.fee_rate.0).into()),
            );
        let new_token_a_pool: Balance = self
            .invariant
            .saturating_mul(self.fee_rate.1.into())
            .checked_div(temp_token_b_pool)
            .unwrap()
            .into();
        let tokens_out_a: Balance = _token_a_pool
            .checked_div(DOLLARS)
            .unwrap()
            .saturating_sub(new_token_a_pool);
        (new_token_a_pool, new_token_b_pool, tokens_out_a)
    }

    pub fn update_pools(&mut self, new_token_a_pool: Balance, new_token_b_pool: Balance) {
        self.token_a_pool = new_token_a_pool;
        self.token_b_pool = new_token_b_pool;
        self.invariant = self.token_a_pool.saturating_mul(self.token_b_pool);
    }

    pub fn calculate_costs(
        &self,
        _token_a_pool: Balance,
        _token_b_pool: Balance,
        shares: Balance,
    ) -> (Balance, Balance) {
        let token_b_per_share: Balance = _token_b_pool.checked_div(self.total_shares).unwrap();
        let token_b_cost: Balance = token_b_per_share.saturating_mul(shares);
        let token_a_per_share: Balance = _token_a_pool.checked_div(self.total_shares).unwrap();
        let token_a_cost: Balance = token_a_per_share.saturating_mul(shares);
        (token_a_cost, token_b_cost)
        // let token_b_cost = _token_b_pool * shares / self.total_shares;
        //let token_a_cost = _token_a_pool * shares / self.total_shares;
    }

    pub fn invest(
        &mut self,
        token_a_cost: Balance,
        token_b_cost: Balance,
        shares: Balance,
        sender: T::AccountId,
    ) {
        let updated_shares = if let Some(prev_shares) = self.shares.get(&sender) {
            prev_shares.saturating_add(shares)
        } else {
            shares
        };
        self.shares.insert(sender.clone(), updated_shares);
        self.total_shares = self.total_shares.saturating_add(shares);
        self.token_b_pool = self.token_b_pool.saturating_add(token_b_cost);
        self.token_a_pool = self.token_a_pool.saturating_add(token_a_cost);
        self.invariant = self.token_b_pool.saturating_mul(self.token_a_pool);
        // let updated_shares = if let Some(prev_shares) = self.shares.get(&sender) {
        //     *prev_shares + shares
        // } else {
        //     shares
        // };
        // self.shares.insert(sender.clone(), updated_shares);
        // self.total_shares += shares;
        // self.token_b_pool += token_b_cost;
        // self.token_a_pool += token_a_cost;
        // self.invariant = self.token_b_pool.saturating_mul(self.token_a_pool); //.checked_div(DOLLARS).unwrap()放s前面
    }

    pub fn devest(
        &mut self,
        token_a_cost: Balance,
        token_b_cost: Balance,
        shares_burned: Balance,
        sender: T::AccountId,
    ) {
        if let Some(share) = self.shares.get_mut(&sender) {
            *share = share.saturating_sub(shares_burned);
        }
        self.total_shares = self.total_shares.saturating_sub(shares_burned);
        self.token_b_pool = self.token_b_pool.saturating_sub(token_b_cost);
        self.token_a_pool = self.token_a_pool.saturating_sub(token_a_cost);
        if self.total_shares == Zero::zero() {
            self.invariant = Zero::zero();
        } else {
            self.invariant = self.token_a_pool.saturating_mul(self.token_b_pool);
        }
        // if let Some(share) = self.shares.get_mut(&sender) {
        //     *share -= shares_burned;
        // }
        // self.total_shares -= shares_burned;
        // self.token_b_pool -= token_b_cost;
        // self.token_a_pool -= token_a_cost;
        // if self.total_shares == Zero::zero() {
        //     self.invariant = Zero::zero();
        // } else {
        //     self.invariant = self.token_a_pool.saturating_mul(self.token_b_pool);
        //     //.checked_div(DOLLARS).unwrap()放s前面
        // }
    }

    pub fn ensure_burned_shares(
        &self,
        sender: T::AccountId,
        shares_burned: Balance,
    ) -> dispatch::DispatchResult {
        ensure!(shares_burned > Zero::zero(), Error::<T>::InvalidShares);
        if let Some(shares) = self.shares.get(&sender) {
            ensure!(*shares >= shares_burned, Error::<T>::InsufficientShares);
            Ok(())
        } else {
            return Err(Error::<T>::DoesNotOwnShare.into());
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
        Swapped(CurrencyId, Balance, CurrencyId, Balance),
        Invested(AccountId, CurrencyId, CurrencyId, Shares),
        Devested(AccountId, CurrencyId, CurrencyId, Shares),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PairNotAllowed,
        LowTokenAmount,
        TokenpoolExists,
        TotalShareIsNotNull,
        PairNotExists,
        SamePair,
        LowAmountReceived,
        InsufficientPool,
        InvalidShares,
        InsufficientShares,
        DoesNotOwnShare,
        LowAmountOut,
        Overflow,
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
        ) -> dispatch::DispatchResult {
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;

                let _pair = TokenPair::new(token_a, token_b);
                ensure!(T::AllowedExchangePairs::get().contains(&_pair), Error::<T>::PairNotAllowed);
                Self::pair_structs(_pair).ensure_launch(token_a_amount, token_b_amount)?;

                let (_token_a_amount, _token_b_amount) = if token_a == _pair.0 {
                    (token_a_amount, token_b_amount)
                } else {
                    (token_b_amount, token_a_amount)
                };

                T::Currency::ensure_can_withdraw(token_a, &sender, token_a_amount)?;
                T::Currency::ensure_can_withdraw(token_b, &sender, token_b_amount)?;

                T::Currency::transfer(token_a, &sender, &T::ExchangeAccountId::get(), token_a_amount)?;
                T::Currency::transfer(token_b, &sender, &T::ExchangeAccountId::get(), token_b_amount)?;

                let pool = PairPool::<T>::initialize_new(_token_a_amount, _token_b_amount, sender.clone());
                PairStructs::<T>::insert(_pair, pool);
                Self::deposit_event(RawEvent::Initialized(sender, token_a, token_b, T::InitialShares::get()));
                Ok(())
            })?;
            Ok(())
        }

        #[weight = 10_000]
        pub fn swap(
            origin,
            input_token: TokenSymbol,
            #[compact] input_token_amount: Balance,
            output_token: TokenSymbol,
            #[compact] min_token_received: Balance,
            receiver: T::AccountId,// new point
        ) -> dispatch::DispatchResult{
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;

                Self::ensure_valid_token_pair(input_token, output_token)?;
                let _pair = TokenPair::new(input_token, output_token);

                let (_token_a_pool, _token_b_pool) = Self::corresponding_pools(input_token, output_token);

                let (new_token_a_pool, new_token_b_pool, tokens_out) = if input_token == _pair.0 {
                    Self::pair_structs(_pair).calculate_a_to_b(_token_a_pool, _token_b_pool, input_token_amount)
                } else {
                    Self::pair_structs(_pair).calculate_b_to_a(_token_a_pool, _token_b_pool, input_token_amount)
                };

                ensure!(tokens_out >= min_token_received, Error::<T>::LowAmountReceived);
                ensure!(tokens_out <= _token_b_pool, Error::<T>::InsufficientPool);

                T::Currency::ensure_can_withdraw(input_token, &sender, input_token_amount)?;
                T::Currency::ensure_can_withdraw(output_token, &T::ExchangeAccountId::get(), tokens_out)?;

                T::Currency::transfer(input_token, &sender, &T::ExchangeAccountId::get(), input_token_amount)?;
                T::Currency::transfer(output_token, &T::ExchangeAccountId::get(), &receiver, tokens_out)?;

                <PairStructs<T>>::mutate(_pair, |pool| {
                    pool.update_pools(new_token_a_pool, new_token_b_pool)
                });

                Self::deposit_event(RawEvent::Swapped(input_token, input_token_amount, output_token, tokens_out));
                Ok(())
            })?;
            Ok(())
        }

        #[weight = 10_000]
        pub fn invest_liquidity(
            origin,
            token_a: TokenSymbol,
            token_b: TokenSymbol,
            #[compact] shares: Balance,
        ) -> dispatch::DispatchResult {
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;

                Self::ensure_valid_token_pair(token_a, token_b)?;
                let _pair = TokenPair::new(token_a, token_b);

                let (_token_a_pool, _token_b_pool) = Self::corresponding_pools(token_a, token_b);
                let (token_a_cost, token_b_cost) = Self::pair_structs(_pair).calculate_costs(_token_a_pool, _token_b_pool, shares);

                T::Currency::ensure_can_withdraw(_pair.0, &sender, token_a_cost)?;
                T::Currency::ensure_can_withdraw(_pair.1, &sender, token_b_cost)?;

                T::Currency::transfer(_pair.0, &sender, &T::ExchangeAccountId::get(), token_a_cost)?;
                T::Currency::transfer(_pair.1, &sender, &T::ExchangeAccountId::get(), token_b_cost)?;

                <PairStructs<T>>::mutate(_pair, |pool|{
                    pool.invest(token_a_cost, token_b_cost, shares, sender.clone())
                });

                Self::deposit_event(RawEvent::Invested(sender, token_a, token_b, shares));
                Ok(())
            })?;
            Ok(())
        }

        #[weight = 10_000]
        pub fn devest_liquidity(
            origin,
            token_a: TokenSymbol,
            token_b: TokenSymbol,
            #[compact] shares_burned: Balance,
            #[compact] min_token_a_received: Balance,
            #[compact] min_token_b_received: Balance
        ) -> dispatch::DispatchResult {
            with_transaction_result(|| {
                let sender = ensure_signed(origin)?;

                Self::ensure_valid_token_pair(token_a, token_b)?;
                let _pair = TokenPair::new(token_a, token_b);
                let pool = Self::pair_structs(_pair);
                pool.ensure_burned_shares(sender.clone(), shares_burned)?;

                let (_token_a_pool, _token_b_pool) = Self::corresponding_pools(token_a, token_b);
                let (token_a_cost, token_b_cost) = pool.calculate_costs(_token_a_pool, _token_b_pool, shares_burned);

                ensure!(token_b_cost >= min_token_b_received, Error::<T>::LowAmountOut);
                ensure!(token_a_cost >= min_token_a_received, Error::<T>::LowAmountOut);

                T::Currency::ensure_can_withdraw(_pair.0, &T::ExchangeAccountId::get(), token_a_cost)?;
                T::Currency::ensure_can_withdraw(_pair.1, &T::ExchangeAccountId::get(), token_b_cost)?;

                T::Currency::transfer(_pair.0, &T::ExchangeAccountId::get(), &sender, token_a_cost)?;
                T::Currency::transfer(_pair.1, &T::ExchangeAccountId::get(), &sender, token_b_cost)?;

                <PairStructs<T>>::mutate(_pair, |pool| {
                    pool.devest(token_a_cost, token_b_cost, shares_burned, sender.clone())
                });

                Self::deposit_event(RawEvent::Devested(sender, token_a, token_b, shares_burned));
                Ok(())
            })?;
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn corresponding_pools(token_a: TokenSymbol, token_b: TokenSymbol) -> (Balance, Balance) {
        let _pair = TokenPair::new(token_a, token_b);
        let pool = Self::pair_structs(_pair);
        let (_token_a_pool, _token_b_pool) = if token_a == _pair.0 {
            (pool.token_a_pool, pool.token_b_pool)
        } else {
            (pool.token_b_pool, pool.token_a_pool)
        };
        (_token_a_pool, _token_b_pool)
    }

    fn ensure_valid_token_pair(
        token_a: TokenSymbol,
        token_b: TokenSymbol,
    ) -> dispatch::DispatchResult {
        ensure!(token_a != token_b, Error::<T>::SamePair);
        let _pair = TokenPair::new(token_a, token_b);
        ensure!(
            PairStructs::<T>::contains_key(_pair),
            Error::<T>::PairNotExists
        );
        ensure!(
            T::AllowedExchangePairs::get().contains(&_pair),
            Error::<T>::PairNotAllowed
        );
        Ok(())
    }
}