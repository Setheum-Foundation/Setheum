// بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيم

// This file is part of Setheum.

// Copyright (C) 2019-Present Setheum Labs.
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

use codec::{Decode, Encode};
use sp_runtime::{
	traits::{Lazy, Verify},
	AccountId32, MultiSigner, RuntimeDebug,
};

use sp_core::{crypto::Public, ecdsa, ed25519, sr25519};

use sp_std::{convert::TryFrom, prelude::*};

#[derive(Eq, PartialEq, Clone, Encode, Decode, RuntimeDebug)]
pub enum SetheumMultiSignature {
	/// An Ed25519 signature.
	Ed25519(ed25519::Signature),
	/// An Sr25519 signature.
	Sr25519(sr25519::Signature),
	/// An ECDSA/SECP256k1 signature.
	Ecdsa(ecdsa::Signature),
	// An Ethereum compatible SECP256k1 signature.
	Ethereum([u8; 65]),
	// An Ethereum SECP256k1 signature using Eip712 for message encoding.
	SetheumEip712([u8; 65]),
}

impl From<ed25519::Signature> for SetheumMultiSignature {
	fn from(x: ed25519::Signature) -> Self {
		Self::Ed25519(x)
	}
}

impl TryFrom<SetheumMultiSignature> for ed25519::Signature {
	type Error = ();
	fn try_from(m: SetheumMultiSignature) -> Result<Self, Self::Error> {
		if let SetheumMultiSignature::Ed25519(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl From<sr25519::Signature> for SetheumMultiSignature {
	fn from(x: sr25519::Signature) -> Self {
		Self::Sr25519(x)
	}
}

impl TryFrom<SetheumMultiSignature> for sr25519::Signature {
	type Error = ();
	fn try_from(m: SetheumMultiSignature) -> Result<Self, Self::Error> {
		if let SetheumMultiSignature::Sr25519(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl From<ecdsa::Signature> for SetheumMultiSignature {
	fn from(x: ecdsa::Signature) -> Self {
		Self::Ecdsa(x)
	}
}

impl TryFrom<SetheumMultiSignature> for ecdsa::Signature {
	type Error = ();
	fn try_from(m: SetheumMultiSignature) -> Result<Self, Self::Error> {
		if let SetheumMultiSignature::Ecdsa(x) = m {
			Ok(x)
		} else {
			Err(())
		}
	}
}

impl Default for SetheumMultiSignature {
	fn default() -> Self {
		Self::Ed25519(Default::default())
	}
}

impl Verify for SetheumMultiSignature {
	type Signer = MultiSigner;
	fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &AccountId32) -> bool {
		match (self, signer) {
			(Self::Ed25519(ref sig), who) => sig.verify(msg, &ed25519::Public::from_slice(who.as_ref())),
			(Self::Sr25519(ref sig), who) => sig.verify(msg, &sr25519::Public::from_slice(who.as_ref())),
			(Self::Ecdsa(ref sig), who) => {
				let m = sp_io::hashing::blake2_256(msg.get());
				match sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m) {
					Ok(pubkey) => &sp_io::hashing::blake2_256(pubkey.as_ref()) == <dyn AsRef<[u8; 32]>>::as_ref(who),
					_ => false,
				}
			}
			_ => false, // Arbitrary message verification is not supported
		}
	}
}
