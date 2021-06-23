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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::upper_case_acronyms)]

use codec::{Decode, Encode, FullCodec, HasCompact};
use frame_support::pallet_prelude::{DispatchClass, Pays, Weight};
use primitives::{
	evm::{CallInfo, EvmAddress},
	CurrencyId,
};
use sp_core::H160;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize},
	transaction_validity::TransactionValidityError,
	DispatchError, DispatchResult, FixedU128, RuntimeDebug,
};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::TryInto,
	fmt::Debug,
	prelude::*,
};

pub mod mocks;

pub type BlockNumber = u32;
pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;
pub type Rate = FixedU128;

pub trait StandardManager<AccountId, CurrencyId, Balance, StandardBalance> {
	fn check_position_valid(
		currency_id: CurrencyId,
		reserve_balance: Balance,
		standard_balance: StandardBalance,
	) -> DispatchResult;
}

impl<AccountId, CurrencyId, Balance: Default, StandardBalance> StandardManager<AccountId, CurrencyId, Balance, Balance>
	for ()
{
	fn check_position_valid(
		_currency_id: CurrencyId,
		_reserve_balance: Balance,
		_standard_balance: StandardBalance,
	) -> DispatchResult {
		Ok(())
	}
}

pub trait SetheumDEXManager<AccountId, CurrencyId, Balance> {
	fn get_liquidity_pool(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance);

	fn get_swap_target_amount(
		path: &[CurrencyId],
		supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance>;

	fn get_swap_supply_amount(
		path: &[CurrencyId],
		target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance>;

	fn swap_with_exact_supply(
		who: &AccountId,
		path: &[CurrencyId],
		supply_amount: Balance,
		min_target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError>;

	fn swap_with_exact_target(
		who: &AccountId,
		path: &[CurrencyId],
		target_amount: Balance,
		max_supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError>;
}

impl<AccountId, CurrencyId, Balance> SetheumDEXManager<AccountId, CurrencyId, Balance> for ()
where
	Balance: Default,
{
	fn get_liquidity_pool(_currency_id_a: CurrencyId, _currency_id_b: CurrencyId) -> (Balance, Balance) {
		Default::default()
	}

	fn get_swap_target_amount(
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Some(Default::default())
	}

	fn get_swap_supply_amount(
		_path: &[CurrencyId],
		_target_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Some(Default::default())
	}

	fn swap_with_exact_supply(
		_who: &AccountId,
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_min_target_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Ok(Default::default())
	}

	fn swap_with_exact_target(
		_who: &AccountId,
		_path: &[CurrencyId],
		_target_amount: Balance,
		_max_supply_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Ok(Default::default())
	}
}

/// An abstraction of serp treasury for the SERP (Setheum Elastic Reserve Protocol).
pub trait SerpTreasury<AccountId> {
	type Amount;
	type Balance;
	type CurrencyId;
	type BlockNumber;

	fn get_adjustment_frequency() -> Self::BlockNumber;

	/// get reserve asset amount of serp treasury
	fn get_total_setter() -> Self::Balance;

	/// calculate the proportion of specific standard amount for the whole system
	fn get_standard_proportion(amount: Self::Balance) -> Ratio;

	/// SerpUp ratio for Serplus Auctions / Swaps
	fn get_serplus_serpup(amount: Self::Balance, currency_id: Self::CurrencyId) -> DispatchResult;

	/// SerpUp ratio for SettPay Cashdrops
	fn get_settpay_serpup(amount: Self::Balance, currency_id: Self::CurrencyId) -> DispatchResult;

	/// SerpUp ratio for Setheum Treasury
	fn get_treasury_serpup(amount: Self::Balance, currency_id: Self::CurrencyId) -> DispatchResult;

	/// SerpUp ratio for Setheum Foundation's Charity Fund
	fn get_charity_fund_serpup(amount: Self::Balance, currency_id: Self::CurrencyId) -> DispatchResult;

	/// issue serpup surplus(stable currencies) to their destinations according to the serpup_ratio.
	fn on_surpup(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult;

	/// buy back and burn surplus(stable currencies) with auction
	/// Create the necessary serp down parameters and starts new auction.
	fn on_surpdown(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult;

	/// Triggers SERP-TES for Serping to stabilize stablecoin prices.
	fn on_serp_tes() -> DispatchResult;

	/// Determines whether to SerpUp or SerpDown based on price swing (+/-)).
	/// positive means "Serp Up", negative means "Serp Down".
	/// Then it calls the necessary option to serp the currency supply (up/down).
	fn serp_tes(currency_id: Self::CurrencyId) -> DispatchResult;

	/// issue standard to `who`
	fn issue_standard(currency_id: Self::CurrencyId, who: &AccountId, standard: Self::Balance) -> DispatchResult;

	/// burn standard(stable currency) of `who`
	fn burn_standard(currency_id: Self::CurrencyId, who: &AccountId, standard: Self::Balance) -> DispatchResult;

	/// issue propper(stable currency) of amount propper to `who`
	fn issue_propper(currency_id: Self::CurrencyId, who: &AccountId, propper: Self::Balance) -> DispatchResult;

	/// burn propper(stable currency) of `who`
	fn burn_propper(currency_id: Self::CurrencyId, who: &AccountId, propper: Self::Balance) -> DispatchResult;

	/// issue setter of amount setter to `who`
	fn issue_setter(who: &AccountId, setter: Self::Balance) -> DispatchResult;

	/// burn setter of `who`
	fn burn_setter(who: &AccountId, setter: Self::Balance) -> DispatchResult;

	/// Issue Dexer (`SDEX` in Setheum or `HALAL` in Neom). `dexer` here just referring to the DEX token balance.
	fn issue_dexer(who: &AccountId, dexer: Self::Balance) -> DispatchResult;

	/// Burn Dexer (`SDEX` in Setheum or `HALAL` in Neom). `dexer` here just referring to the DEX token balance.
	fn burn_dexer(who: &AccountId, dexer: Self::Balance) -> DispatchResult;

	/// deposit surplus(propperstable currency) to serp treasury by `from`
	fn deposit_surplus(currency_id: Self::CurrencyId, from: &AccountId, surplus: Self::Balance) -> DispatchResult;

	/// deposit reserve asset (Setter (SETT)) to serp treasury by `who`
	fn deposit_setter(from: &AccountId, amount: Self::Balance) -> DispatchResult;

	/// Burn Reserve asset (Setter (SETT))
	fn burn_setter(who, amount: Self::Balance) -> DispatchResult;
}

pub trait SerpTreasuryExtended<AccountId>: SerpTreasury<AccountId> {
	fn swap_exact_setter_in_auction_to_settcurrency(
		currency_id: Self::CurrencyId,
		supply_amount: Self::Balance,
		min_target_amount: Self::Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Self::Balance, DispatchError>;

	fn swap_setter_not_in_auction_with_exact_settcurrency(
		currency_id: Self::CurrencyId,
		target_amount: Self::Balance,
		max_supply_amount: Self::Balance,
		price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Self::Balance, DispatchError>;

	fn create_reserve_auctions(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
		target: Self::Balance,
		refund_receiver: AccountId,
		splited: bool,
	) -> DispatchResult;
}

pub trait PriceProvider<CurrencyId> {
	fn get_fiat_price(fiat_currency_id: CurrencyId) -> Option<Price>;
	fn get_peg_price(currency_id: CurrencyId) -> Option<Price>;
	fn get_setheum_usd_fixed_price() -> Option<Price>;
	fn get_stablecoin_fixed_price(currency_id: CurrencyId) -> Option<Price>;
	fn get_stablecoin_market_price(currency_id: CurrencyId) -> Option<Price>;
	fn get_relative_price(base: CurrencyId, quote: CurrencyId) -> Option<Price>;
	fn get_coin_to_peg_relative_price(currency_id: CurrencyId) -> Option<Price>;
	fn get_setter_basket_peg_price() -> Option<Price>;
	fn get_setter_fixed_price() -> Option<Price>;
	fn get_price(currency_id: CurrencyId) -> Option<Price>;
	fn lock_price(currency_id: CurrencyId);
	fn unlock_price(currency_id: CurrencyId);
}

pub trait ExchangeRateProvider {
	fn get_exchange_rate() -> ExchangeRate;
}

pub trait DEXIncentives<AccountId, CurrencyId, Balance> {
	fn do_deposit_dex_share(who: &AccountId, lp_currency_id: CurrencyId, amount: Balance) -> DispatchResult;
	fn do_withdraw_dex_share(who: &AccountId, lp_currency_id: CurrencyId, amount: Balance) -> DispatchResult;
}

impl<AccountId, CurrencyId, Balance> DEXIncentives<AccountId, CurrencyId, Balance> for () {
	fn do_deposit_dex_share(_: &AccountId, _: CurrencyId, _: Balance) -> DispatchResult {
		Ok(())
	}

	fn do_withdraw_dex_share(_: &AccountId, _: CurrencyId, _: Balance) -> DispatchResult {
		Ok(())
	}
}

pub trait TransactionPayment<AccountId, Balance, NegativeImbalance> {
	fn reserve_fee(who: &AccountId, weight: Weight) -> Result<Balance, DispatchError>;
	fn unreserve_fee(who: &AccountId, fee: Balance);
	fn unreserve_and_charge_fee(
		who: &AccountId,
		weight: Weight,
	) -> Result<(Balance, NegativeImbalance), TransactionValidityError>;
	fn refund_fee(who: &AccountId, weight: Weight, payed: NegativeImbalance) -> Result<(), TransactionValidityError>;
	fn charge_fee(
		who: &AccountId,
		len: u32,
		weight: Weight,
		tip: Balance,
		pays_fee: Pays,
		class: DispatchClass,
	) -> Result<(), TransactionValidityError>;
}

#[cfg(feature = "std")]
use frame_support::traits::Imbalance;
#[cfg(feature = "std")]
impl<AccountId, Balance: Default + Copy, NegativeImbalance: Imbalance<Balance>>
	TransactionPayment<AccountId, Balance, NegativeImbalance> for ()
{
	fn reserve_fee(_who: &AccountId, _weight: Weight) -> Result<Balance, DispatchError> {
		Ok(Default::default())
	}

	fn unreserve_fee(_who: &AccountId, _fee: Balance) {}

	fn unreserve_and_charge_fee(
		_who: &AccountId,
		_weight: Weight,
	) -> Result<(Balance, NegativeImbalance), TransactionValidityError> {
		Ok((Default::default(), Imbalance::zero()))
	}

	fn refund_fee(
		_who: &AccountId,
		_weight: Weight,
		_payed: NegativeImbalance,
	) -> Result<(), TransactionValidityError> {
		Ok(())
	}

	fn charge_fee(
		_who: &AccountId,
		_len: u32,
		_weight: Weight,
		_tip: Balance,
		_pays_fee: Pays,
		_class: DispatchClass,
	) -> Result<(), TransactionValidityError> {
		Ok(())
	}
}

pub trait Contains<T> {
	fn contains(t: &T) -> bool;
}
