#![cfg(test)]

use super::*; //use crate::{Error, mock::*};
use frame_support::assert_ok;
use mock::*;

#[test]
fn initialize_new_works() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Tokens::free_balance(BUSD, &ExchangeModule::account_id()), 0);
        assert_eq!(Tokens::free_balance(BUSD, &ALICE), 1_000_000);
        assert_eq!(Tokens::free_balance(DOT, &ALICE), 1_000_000);
        let mut shares_map = BTreeMap::new();
        shares_map.insert(ALICE, 100);
        let pair_pool = PairPool::<Test> {
            fee_rate: (3, 1000),
            token_a_pool: 1000,
            token_b_pool: 1000,
            invariant: 1_000_000,
            total_shares: 100,
            shares: shares_map,
        };
        assert_eq!(
            PairPool::<Test>::initialize_new(1000, 1000, ALICE),
            pair_pool
        );
        // assert_eq!(
        //     Tokens::free_balance(BUSD, &ExchangeModule::account_id()),
        //     1000
        // );
        // assert_eq!(
        //     Tokens::free_balance(DOT, &ExchangeModule::account_id()),
        //     1000
        // );
    });
}
// #[test]
// fn calculate_a_to_b_works() {
//     ExtBuilder::default().build().execute_with(|| {
//         let pair = ExchangeModule::pair_structs(BUSD_DOT_PAIR);
//         assert_eq!(pair.calculate_a_to_b(1000, 1000, 100), (1100, 909, 91));
//     });
// }
#[test]
fn calculate_b_to_a_works() {
    ExtBuilder::default().build().execute_with(|| {
        let pair = ExchangeModule::pair_structs(BUSD_DOT_PAIR);
        assert_eq!(pair.calculate_b_to_a(1000, 1000, 100), (1100, 909, 91));
    });
}
// #[test]
// fn calculate_costs_works() {
//     ExtBuilder::default().build().execute_with(|| {
//         let pair = ExchangeModule::pair_structs(BUSD_DOT_PAIR);
//         assert_eq!(pair.calculate_costs(1000, 1000, 10), (100, 100));
//     });
// }
// #[test]
// fn swap_works() {
//     ExtBuilder::default().build().execute_with(|| {
//         assert_eq!(Tokens::free_balance(DOT, &BOB), 0);
//         assert_eq!(Tokens::free_balance(BUSD, &BOB), 0);
//         // let pair = ExchangeModule::pair_structs(BUSD_DOT_PAIR);
//         // assert_ok!(PairPool::<Test>::initialize_new(1000, 1000, ALICE));
//         assert_ok!(ExchangeModule::swap(
//             Origin::signed(ALICE),
//             BUSD,
//             100,
//             DOT,
//             90,
//             BOB
//         ));
//         assert_eq!(Tokens::free_balance(DOT, &BOB), 91);
//         assert_ok!(ExchangeModule::swap(
//             Origin::signed(ALICE),
//             BUSD,
//             100,
//             DOT,
//             90,
//             BOB
//         ));
//         assert_eq!(Tokens::free_balance(BUSD, &BOB), 91);
//     });
// }
