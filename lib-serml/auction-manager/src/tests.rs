// بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيم
// ٱلَّذِينَ يَأْكُلُونَ ٱلرِّبَوٰا۟ لَا يَقُومُونَ إِلَّا كَمَا يَقُومُ ٱلَّذِى يَتَخَبَّطُهُ ٱلشَّيْطَـٰنُ مِنَ ٱلْمَسِّ ۚ ذَٰلِكَ بِأَنَّهُمْ قَالُوٓا۟ إِنَّمَا ٱلْبَيْعُ مِثْلُ ٱلرِّبَوٰا۟ ۗ وَأَحَلَّ ٱللَّهُ ٱلْبَيْعَ وَحَرَّمَ ٱلرِّبَوٰا۟ ۚ فَمَن جَآءَهُۥ مَوْعِظَةٌ مِّن رَّبِّهِۦ فَٱنتَهَىٰ فَلَهُۥ مَا سَلَفَ وَأَمْرُهُۥٓ إِلَى ٱللَّهِ ۖ وَمَنْ عَادَ فَأُو۟لَـٰٓئِكَ أَصْحَـٰبُ ٱلنَّارِ ۖ هُمْ فِيهَا خَـٰلِدُونَ

// This file is part of Setheum.

// Copyright (C) 2019-2021 Setheum Labs.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Unit tests for the auction manager module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Call as MockCall, Event, *};
use sp_core::offchain::{testing, DbExternalities, OffchainDbExt, OffchainWorkerExt, StorageKind, TransactionPoolExt};
use sp_io::offchain;
use sp_runtime::traits::One;

fn run_to_block_offchain(n: u64) {
	while System::block_number() < n {
		System::set_block_number(System::block_number() + 1);
		AuctionManagerModule::offchain_worker(System::block_number());
		// this unlocks the concurrency storage lock so offchain_worker will fire next block
		offchain::sleep_until(offchain::timestamp().add(Duration::from_millis(LOCK_DURATION + 200)));
	}
}

#[test]
fn get_auction_time_to_close_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionManagerModule::get_auction_time_to_close(2000, 1), 100);
		assert_eq!(AuctionManagerModule::get_auction_time_to_close(2001, 1), 50);
	});
}

#[test]
fn collateral_auction_methods() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert_eq!(
			AuctionModule::auctions(0),
			Some(orml_traits::AuctionInfo {
				bid: None,
				start: 0,
				end: Some(2000)
			})
		);
		let collateral_auction_with_positive_target = AuctionManagerModule::collateral_auctions(0).unwrap();
		assert!(!collateral_auction_with_positive_target.always_forward());
		assert!(!collateral_auction_with_positive_target.in_reverse_stage(99));
		assert!(collateral_auction_with_positive_target.in_reverse_stage(100));
		assert!(collateral_auction_with_positive_target.in_reverse_stage(101));
		assert_eq!(collateral_auction_with_positive_target.payment_amount(99), 99);
		assert_eq!(collateral_auction_with_positive_target.payment_amount(100), 100);
		assert_eq!(collateral_auction_with_positive_target.payment_amount(101), 100);
		assert_eq!(collateral_auction_with_positive_target.collateral_amount(80, 100), 10);
		assert_eq!(collateral_auction_with_positive_target.collateral_amount(100, 200), 5);

		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 0));
		let collateral_auction_with_zero_target = AuctionManagerModule::collateral_auctions(1).unwrap();
		assert!(collateral_auction_with_zero_target.always_forward());
		assert!(!collateral_auction_with_zero_target.in_reverse_stage(0));
		assert!(!collateral_auction_with_zero_target.in_reverse_stage(100));
		assert_eq!(collateral_auction_with_zero_target.payment_amount(99), 99);
		assert_eq!(collateral_auction_with_zero_target.payment_amount(101), 101);
		assert_eq!(collateral_auction_with_zero_target.collateral_amount(100, 200), 10);
	});
}

#[test]
fn new_collateral_auction_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let ref_count_0 = System::consumers(&ALICE);
		assert_noop!(
			AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 0, 100),
			Error::<Runtime>::InvalidAmount,
		);

		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::NewCollateralAuction(
			0, SERP, 10, 100,
		)));

		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 10);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 100);
		assert_eq!(AuctionModule::auctions_index(), 1);
		assert_eq!(System::consumers(&ALICE), ref_count_0 + 1);

		assert_noop!(
			AuctionManagerModule::new_collateral_auction(&ALICE, SERP, Balance::max_value(), Balance::max_value()),
			Error::<Runtime>::InvalidAmount,
		);
	});
}

#[test]
fn collateral_auction_bid_handler_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			AuctionManagerModule::collateral_auction_bid_handler(1, 0, (BOB, 4), None),
			Error::<Runtime>::AuctionNotExists,
		);

		assert_ok!(CDPTreasuryModule::deposit_collateral(&ALICE, SERP, 10));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert_eq!(CDPTreasuryModule::surplus_pool(), 0);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 1000);

		let bob_ref_count_0 = System::consumers(&BOB);

		assert_noop!(
			AuctionManagerModule::collateral_auction_bid_handler(1, 0, (BOB, 4), None),
			Error::<Runtime>::InvalidBidPrice,
		);
		assert!(AuctionManagerModule::collateral_auction_bid_handler(1, 0, (BOB, 5), None).is_ok(),);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 5);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 995);

		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 + 1);
		let carol_ref_count_0 = System::consumers(&CAROL);

		assert!(AuctionManagerModule::collateral_auction_bid_handler(2, 0, (CAROL, 10), Some((BOB, 5))).is_ok(),);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 10);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &CAROL), 990);
		assert_eq!(AuctionManagerModule::collateral_auctions(0).unwrap().amount, 10);

		let bob_ref_count_2 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_2, bob_ref_count_1 - 1);
		let carol_ref_count_1 = System::consumers(&CAROL);
		assert_eq!(carol_ref_count_1, carol_ref_count_0 + 1);

		assert!(AuctionManagerModule::collateral_auction_bid_handler(3, 0, (BOB, 200), Some((CAROL, 10))).is_ok(),);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 100);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 900);
		assert_eq!(Tokens::free_balance(SETUSD, &CAROL), 1000);
		assert_eq!(AuctionManagerModule::collateral_auctions(0).unwrap().amount, 5);

		let bob_ref_count_3 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_3, bob_ref_count_2 + 1);
		let carol_ref_count_2 = System::consumers(&CAROL);
		assert_eq!(carol_ref_count_2, carol_ref_count_1 - 1);
	});
}

#[test]
fn bid_when_soft_cap_for_collateral_auction_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert_eq!(
			AuctionManagerModule::on_new_bid(1, 0, (BOB, 100), None).auction_end_change,
			Change::NewValue(Some(101))
		);
		assert!(!AuctionManagerModule::on_new_bid(2001, 0, (CAROL, 10), Some((BOB, 5))).accept_bid,);
		assert_eq!(
			AuctionManagerModule::on_new_bid(2001, 0, (CAROL, 15), Some((BOB, 5))).auction_end_change,
			Change::NewValue(Some(2051))
		);
	});
}

#[test]
fn collateral_auction_end_handler_without_bid() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 100));
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(CAROL),
			SERP,
			SETUSD,
			100,
			1000,
			0
		));
		assert_eq!(DEXModule::get_liquidity_pool(SERP, SETUSD), (100, 1000));
		assert_eq!(DEXModule::get_swap_target_amount(&[SERP, SETUSD], 100).unwrap(), 498);

		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 100, 200));
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 100);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 200);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 100);
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1000);
		assert_eq!(CDPTreasuryModule::debit_pool(), 0);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 0);
		let alice_ref_count_0 = System::consumers(&ALICE);

		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		AuctionManagerModule::on_auction_ended(0, None);
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::DEXTakeCollateralAuction(
			0, SERP, 100, 497,
		)));

		assert_eq!(DEXModule::get_liquidity_pool(SERP, SETUSD), (200, 503));
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 0);
		assert_eq!(AuctionManagerModule::collateral_auctions(0), None);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 0);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 0);
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1297);
		assert_eq!(CDPTreasuryModule::debit_pool(), 297);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 498);
		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 - 1);
	});
}

#[test]
fn collateral_auction_end_handler_without_bid_and_swap_by_alternative_path() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 100));
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(BOB),
			SERP,
			DNAR,
			100,
			1000,
			0
		));
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(CAROL),
			DNAR,
			SETUSD,
			1000,
			1000,
			0
		));
		assert_eq!(DEXModule::get_liquidity_pool(SERP, DNAR), (100, 1000));
		assert_eq!(DEXModule::get_liquidity_pool(DNAR, SETUSD), (1000, 1000));
		assert_eq!(DEXModule::get_swap_target_amount(&[SERP, SETUSD], 100), None);
		assert_eq!(DEXModule::get_swap_target_amount(&[SERP, DNAR, SETUSD], 100), Some(331));

		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 100, 200));
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1000);

		AuctionManagerModule::on_auction_ended(0, None);
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::DEXTakeCollateralAuction(
			0, SERP, 100, 329,
		)));

		assert_eq!(DEXModule::get_liquidity_pool(SERP, DNAR), (100, 1000));
		assert_eq!(DEXModule::get_liquidity_pool(DNAR, SETUSD), (1498, 671));
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1129);
	});
}

#[test]
fn collateral_auction_end_handler_in_reverse_stage() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 100));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 100, 200));
		assert!(AuctionManagerModule::collateral_auction_bid_handler(2, 0, (BOB, 400), None).is_ok(),);
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 50);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 50);
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1050);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 800);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 200);

		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		AuctionManagerModule::on_auction_ended(0, Some((BOB, 400)));
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::CollateralAuctionDealt(
			0, SERP, 50, BOB, 200,
		)));

		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 0);
		assert_eq!(AuctionManagerModule::collateral_auctions(0), None);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 0);
		assert_eq!(Tokens::free_balance(SERP, &ALICE), 1050);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1050);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 800);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 200);

		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 - 1);
		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 - 1);
	});
}

#[test]
fn collateral_auction_end_handler_by_dealing_which_target_not_zero() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 100));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 100, 200));
		assert!(AuctionManagerModule::collateral_auction_bid_handler(1, 0, (BOB, 100), None).is_ok(),);
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 100);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 200);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 100);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 900);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 100);

		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		AuctionManagerModule::on_auction_ended(0, Some((BOB, 100)));
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::CollateralAuctionDealt(
			0, SERP, 100, BOB, 100,
		)));

		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 0);
		assert_eq!(AuctionManagerModule::collateral_auctions(0), None);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 0);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 0);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 0);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1100);

		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 - 1);
		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 - 1);
	});
}

#[test]
fn collateral_auction_end_handler_by_dex_which_target_not_zero() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 100));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 100, 200));
		assert!(AuctionManagerModule::collateral_auction_bid_handler(1, 0, (BOB, 20), None).is_ok(),);
		assert_ok!(DEXModule::add_liquidity(
			Origin::signed(CAROL),
			SERP,
			SETUSD,
			100,
			1000,
			0
		));
		assert_eq!(DEXModule::get_swap_target_amount(&[SERP, SETUSD], 100).unwrap(), 498);

		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 100);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 200);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 100);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 980);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1000);
		assert_eq!(CDPTreasuryModule::debit_pool(), 0);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 20);

		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		AuctionManagerModule::on_auction_ended(0, Some((BOB, 20)));
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::DEXTakeCollateralAuction(
			0, SERP, 100, 497,
		)));

		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 0);
		assert_eq!(AuctionManagerModule::collateral_auctions(0), None);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 0);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 0);
		assert_eq!(Tokens::free_balance(SERP, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 1000);
		assert_eq!(Tokens::free_balance(SETUSD, &ALICE), 1297);
		assert_eq!(CDPTreasuryModule::debit_pool(), 317);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 518);

		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 - 1);
		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 - 1);
	});
}

#[test]
fn swap_bidders_works() {
	ExtBuilder::default().build().execute_with(|| {
		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		AuctionManagerModule::swap_bidders(&BOB, None);

		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 + 1);

		AuctionManagerModule::swap_bidders(&ALICE, Some(&BOB));

		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 + 1);
		let bob_ref_count_2 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_2, bob_ref_count_1 - 1);

		AuctionManagerModule::swap_bidders(&BOB, Some(&ALICE));

		let alice_ref_count_2 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_2, alice_ref_count_1 - 1);
		let bob_ref_count_3 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_3, bob_ref_count_2 + 1);
	});
}

#[test]
fn cancel_collateral_auction_failed() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 10));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		MockPriceSource::set_relative_price(None);
		assert_noop!(
			AuctionManagerModule::cancel_collateral_auction(0, AuctionManagerModule::collateral_auctions(0).unwrap()),
			Error::<Runtime>::InvalidFeedPrice,
		);
		MockPriceSource::set_relative_price(Some(Price::one()));

		assert_ok!(AuctionModule::bid(Origin::signed(ALICE), 0, 100));
		let collateral_auction = AuctionManagerModule::collateral_auctions(0).unwrap();
		assert!(!collateral_auction.always_forward());
		assert_eq!(AuctionManagerModule::get_last_bid(0), Some((ALICE, 100)));
		assert!(collateral_auction.in_reverse_stage(100));
		assert_noop!(
			AuctionManagerModule::cancel_collateral_auction(0, collateral_auction),
			Error::<Runtime>::InReverseStage,
		);
	});
}

#[test]
fn cancel_collateral_auction_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(CDPTreasuryModule::deposit_collateral(&CAROL, SERP, 10));
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 10);
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 10);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 100);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 0);
		assert_eq!(CDPTreasuryModule::debit_pool(), 0);
		assert_ok!(AuctionModule::bid(Origin::signed(BOB), 0, 80));
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 920);
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 10);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 80);
		assert_eq!(CDPTreasuryModule::debit_pool(), 0);
		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 920);

		let alice_ref_count_0 = System::consumers(&ALICE);
		let bob_ref_count_0 = System::consumers(&BOB);

		mock_shutdown();
		assert_ok!(AuctionManagerModule::cancel(Origin::none(), 0));
		System::assert_last_event(Event::AuctionManagerModule(crate::Event::CancelAuction(0)));

		assert_eq!(Tokens::free_balance(SETUSD, &BOB), 1000);
		assert_eq!(AuctionManagerModule::total_collateral_in_auction(SERP), 0);
		assert_eq!(AuctionManagerModule::total_target_in_auction(), 0);
		assert_eq!(CDPTreasuryModule::total_collaterals(SERP), 10);
		assert_eq!(CDPTreasuryModule::debit_pool(), 80);
		assert_eq!(CDPTreasuryModule::surplus_pool(), 80);
		assert!(!AuctionManagerModule::collateral_auctions(0).is_some());
		assert!(!AuctionModule::auction_info(0).is_some());

		let alice_ref_count_1 = System::consumers(&ALICE);
		assert_eq!(alice_ref_count_1, alice_ref_count_0 - 1);
		let bob_ref_count_1 = System::consumers(&BOB);
		assert_eq!(bob_ref_count_1, bob_ref_count_0 - 1);
	});
}

#[test]
fn offchain_worker_cancels_auction_in_shutdown() {
	let (offchain, _offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let mut ext = ExtBuilder::default().build();
	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(OffchainDbExt::new(offchain.clone()));

	ext.execute_with(|| {
		System::set_block_number(1);
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		run_to_block_offchain(2);
		// offchain worker does not have any tx because shutdown is false
		assert!(!MockEmergencyShutdown::is_shutdown());
		assert!(pool_state.write().transactions.pop().is_none());
		mock_shutdown();
		assert!(MockEmergencyShutdown::is_shutdown());

		// now offchain worker will cancel auction as shutdown is true
		run_to_block_offchain(3);
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let MockCall::AuctionManagerModule(crate::Call::cancel(auction_id)) = tx.call {
			assert_ok!(AuctionManagerModule::cancel(Origin::none(), auction_id));
		}

		// auction is canceled
		assert!(AuctionManagerModule::collateral_auctions(0).is_none());
		assert!(pool_state.write().transactions.pop().is_none());
	});
}

#[test]
fn offchain_worker_max_iterations_check() {
	let (mut offchain, _offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let mut ext = ExtBuilder::default().build();
	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(OffchainDbExt::new(offchain.clone()));

	ext.execute_with(|| {
		System::set_block_number(1);
		// sets max iterations value to 1
		offchain.local_storage_set(StorageKind::PERSISTENT, OFFCHAIN_WORKER_MAX_ITERATIONS, &1u32.encode());
		assert_ok!(AuctionManagerModule::new_collateral_auction(&ALICE, SERP, 10, 100));
		assert_ok!(AuctionManagerModule::new_collateral_auction(&BOB, SERP, 10, 100));
		assert!(AuctionManagerModule::collateral_auctions(1).is_some());
		assert!(AuctionManagerModule::collateral_auctions(0).is_some());
		mock_shutdown();
		assert!(MockEmergencyShutdown::is_shutdown());

		run_to_block_offchain(2);
		// now offchain worker will cancel one auction but the other one will cancel next block
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let MockCall::AuctionManagerModule(crate::Call::cancel(auction_id)) = tx.call {
			assert_ok!(AuctionManagerModule::cancel(Origin::none(), auction_id));
		}
		assert!(
			AuctionManagerModule::collateral_auctions(1).is_some()
				|| AuctionManagerModule::collateral_auctions(0).is_some()
		);
		// only one auction canceled so offchain tx pool is empty
		assert!(pool_state.write().transactions.pop().is_none());

		run_to_block_offchain(3);
		// now offchain worker will cancel the next auction
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let MockCall::AuctionManagerModule(crate::Call::cancel(auction_id)) = tx.call {
			assert_ok!(AuctionManagerModule::cancel(Origin::none(), auction_id));
		}
		assert!(AuctionManagerModule::collateral_auctions(1).is_none());
		assert!(AuctionManagerModule::collateral_auctions(0).is_none());
		assert!(pool_state.write().transactions.pop().is_none());
	});
}

#[test]
fn offchain_default_max_iterator_works() {
	let (mut offchain, _offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let mut ext = ExtBuilder::lots_of_accounts().build();
	ext.register_extension(OffchainWorkerExt::new(offchain.clone()));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(OffchainDbExt::new(offchain.clone()));

	ext.execute_with(|| {
		System::set_block_number(1);
		// checks that max iterations is stored as none
		assert!(offchain
			.local_storage_get(StorageKind::PERSISTENT, OFFCHAIN_WORKER_MAX_ITERATIONS)
			.is_none());

		for i in 0..1001 {
			let account_id: AccountId = i;
			assert_ok!(AuctionManagerModule::new_collateral_auction(&account_id, SERP, 1, 10));
		}

		mock_shutdown();
		run_to_block_offchain(2);
		// should only run 1000 iterations stopping due to DEFAULT_MAX_ITERATION
		assert_eq!(pool_state.write().transactions.len(), 1000);
		run_to_block_offchain(3);
		// next block iterator starts where it left off and adds the final account to tx pool
		assert_eq!(pool_state.write().transactions.len(), 1001);
	});
}
