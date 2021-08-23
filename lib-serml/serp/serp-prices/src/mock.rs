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

//! Mocks for the prices module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use orml_traits::{parameter_type_with_key, DataFeeder};
use primitives::{Amount, TokenSymbol};
use sp_core::{H160, H256};
use sp_runtime::{
	testing::Header,
	traits::{IdentityLookup, One as OneT, Zero},
	DispatchError, FixedPointNumber,
};
use support::{mocks::MockCurrencyIdMapping, Ratio};

pub type AccountId = u128;
pub type BlockNumber = u64;

mod serp_prices {
	pub use super::super::*;
}

// Currencies constants - CurrencyId/TokenSymbol
pub const DNAR: CurrencyId = CurrencyId::Token(TokenSymbol::DNAR);
pub const DRAM: CurrencyId = CurrencyId::Token(TokenSymbol::DRAM);
pub const SETR: CurrencyId = CurrencyId::Token(TokenSymbol::SETR);
pub const SETUSD: CurrencyId = CurrencyId::Token(TokenSymbol::SETUSD);
pub const SETEUR: CurrencyId = CurrencyId::Token(TokenSymbol::SETEUR);
pub const SETGBP: CurrencyId = CurrencyId::Token(TokenSymbol::SETGBP);
pub const SETCHF: CurrencyId = CurrencyId::Token(TokenSymbol::SETCHF);
pub const SETSAR: CurrencyId = CurrencyId::Token(TokenSymbol::SETSAR);
pub const BTC: CurrencyId = CurrencyId::Token(TokenSymbol::RENBTC);


// LP tokens constants - CurrencyId/TokenSymbol : Dex Shares
pub const LP_BTC_SETUSD: CurrencyId =
CurrencyId::DexShare(DexShare::Token(TokenSymbol::RENBTC), DexShare::Token(TokenSymbol::SETUSD));
pub const LP_SETUSD_DRAM: CurrencyId =
CurrencyId::DexShare(DexShare::Token(TokenSymbol::SETUSD), DexShare::Token(TokenSymbol::DRAM));

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

pub struct MockDataProvider;
impl DataProvider<CurrencyId, Price> for MockDataProvider {
	fn get(currency_id: &CurrencyId) -> Option<Price> {
		match *currency_id {
			SETUSD => Some(Price::saturating_from_rational(99, 100)),
			BTC => Some(Price::saturating_from_integer(50000)),
			DNAR => Some(Price::saturating_from_integer(100)),
			DRAM => Some(Price::zero()),
			_ => None,
		}
	}
}

impl DataFeeder<CurrencyId, Price, AccountId> for MockDataProvider {
	fn feed_value(_: AccountId, _: CurrencyId, _: Price) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

pub struct MockDEX;
impl DEXManager<AccountId, CurrencyId, Balance> for MockDEX {
	fn get_liquidity_pool(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance) {
		match (currency_id_a, currency_id_b) {
			(SETUSD, DNAR) => (10000, 200),
			_ => (0, 0),
		}
	}

	fn get_liquidity_token_address(_currency_id_a: CurrencyId, _currency_id_b: CurrencyId) -> Option<H160> {
		unimplemented!()
	}

	fn get_swap_target_amount(
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		unimplemented!()
	}

	fn get_swap_supply_amount(
		_path: &[CurrencyId],
		_target_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		unimplemented!()
	}

	fn swap_with_exact_supply(
		_who: &AccountId,
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_min_target_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		unimplemented!()
	}

	fn swap_with_exact_target(
		_who: &AccountId,
		_path: &[CurrencyId],
		_target_amount: Balance,
		_max_supply_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		unimplemented!()
	}

	fn add_liquidity(
		_who: &AccountId,
		_currency_id_a: CurrencyId,
		_currency_id_b: CurrencyId,
		_max_amount_a: Balance,
		_max_amount_b: Balance,
		_min_share_increment: Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	fn remove_liquidity(
		_who: &AccountId,
		_currency_id_a: CurrencyId,
		_currency_id_b: CurrencyId,
		_remove_share: Balance,
		_min_withdrawn_a: Balance,
		_min_withdrawn_b: Balance,
	) -> DispatchResult {
		unimplemented!()
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ();
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

parameter_types! {
	pub const SetterCurrencyId: CurrencyId = SETR; // Setter currency ticker is SETR.
	pub const GetSetUSDCurrencyId: CurrencyId = SETUSD; // SetUSD currency ticker is SETUSD.
	pub const GetSetEURCurrencyId: CurrencyId = SETEUR; // SetEUR currency ticker is SETEUR.
	pub const GetSetGBPCurrencyId: CurrencyId = SETGBP; // SetGBP currency ticker is SETGBP.
	pub const GetSetCHFCurrencyId: CurrencyId = SETCHF; // SetCHF currency ticker is SETCHF.
	pub const GetSetSARCurrencyId: CurrencyId = SETSAR; // SetSAR currency ticker is SETSAR.
	pub FiatUsdFixedPrice: Price = Price::one(); // Fixed 1 USD Fiat denomination for pricing.
}

pub struct OffchainPriceMock;

impl FetchPriceFor for OffchainPriceMock {
	fn get_price_for(symbol: &[u8]) -> u64 {
		match *symbol {
			b"DNAR" => 100u64,
			b"DRAM" => 0u64,
			b"BTC" => 50000u64,
			b"SETR" => 1u64,
			b"SETUSD" => 1u64, // stable
			b"SETEUR" => 2u64, // up 50%
			b"SETGBP" => 1u64, // stable
			b"SETCHF" => 1u64, // down 50%
			b"SETSAR" => 2u64, // up 50%
			b"USD" => 1u64,
			b"EUR" => 1u64,
			b"GBP" => 1u64,
			b"CHF" => 2u64,
			b"SAR" => 1u64,
			_ => None,
		}
	}
}

impl Config for Runtime {
	type Event = Event;
	type Source = MockDataProvider;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type DirhamCurrencyId = DirhamCurrencyId;
	type SetterCurrencyId = SetterCurrencyId;
	type GetSetUSDCurrencyId = GetSetUSDCurrencyId;
	type GetSetEURCurrencyId = GetSetEURCurrencyId;
	type GetSetGBPCurrencyId = GetSetGBPCurrencyId;
	type GetSetCHFCurrencyId = GetSetCHFCurrencyId;
	type GetSetSARCurrencyId = GetSetSARCurrencyId;
	type SerpOcwOffchainPrice = OffchainPriceMock;
	type FiatUsdFixedPrice = FiatUsdFixedPrice;
	type LockOrigin = EnsureSignedBy<One, AccountId>;
	type DEX = MockDEX;
	type Currency = Tokens;
	type CurrencyIdMapping = MockCurrencyIdMapping;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		SerpOcw: serp_ocw::{Pallet, Storage, Call, Event<T>},
		SerpPrices: serp_prices::{Pallet, Storage, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
	}
);

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		t.into()
	}
}
