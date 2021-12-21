// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use frame_support::{BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound};
use sp_std::{collections::btree_set::BTreeSet, fmt::Debug};

use crate::Verifier;
use codec::{Decode, Encode, MaxEncodedLen};
pub use frame_election_provider_support::PageIndex;
use frame_election_provider_support::{BoundedSupports, ElectionDataProvider, ElectionProvider};
use scale_info::TypeInfo;
pub use sp_npos_elections::{ElectionResult, ElectionScore, NposSolution};
use sp_runtime::SaturatedConversion;

/// The supports that's returned from a given [`Verifier`]. TODO: rename this
pub type SupportsOf<V> = BoundedSupports<
	<V as Verifier>::AccountId,
	<V as Verifier>::MaxSupportsPerPage,
	<V as Verifier>::MaxBackersPerSupport,
>;

/// The solution type used by this crate.
pub type SolutionOf<T> = <T as crate::Config>::Solution;

/// The voter index. Derived from [`SolutionOf`].
pub type SolutionVoterIndexOf<T> = <SolutionOf<T> as NposSolution>::VoterIndex;
/// The target index. Derived from [`SolutionOf`].
pub type SolutionTargetIndexOf<T> = <SolutionOf<T> as NposSolution>::TargetIndex;
/// The accuracy of the election, when submitted from offchain. Derived from [`SolutionOf`].
pub type SolutionAccuracyOf<T> = <SolutionOf<T> as NposSolution>::Accuracy;
/// The fallback election type.
pub type FallbackErrorOf<T> = <<T as crate::Config>::Fallback as ElectionProvider>::Error;

/// The relative distribution of a voter's stake among the winning targets.
pub type AssignmentOf<T> =
	sp_npos_elections::Assignment<<T as frame_system::Config>::AccountId, SolutionAccuracyOf<T>>;

#[derive(
	TypeInfo,
	Encode,
	Decode,
	RuntimeDebugNoBound,
	CloneNoBound,
	EqNoBound,
	PartialEqNoBound,
	MaxEncodedLen,
)]
#[codec(mel_bound(T: crate::Config))]
#[scale_info(skip_type_params(T))]
pub struct PagedRawSolution<T: crate::Config> {
	pub solution_pages: BoundedVec<SolutionOf<T>, T::Pages>,
	pub score: ElectionScore,
	pub round: u32,
}

/// A helper trait to deal with the page index of partial solutions.
///
/// This should only be called on the `Vec<Solution>` or similar types. If the solution is *full*,
/// then it returns a normal iterator that is just mapping the index (usize) to `PageIndex`.
///
/// if the solution is partial, it shifts the indices sufficiently so that the most significant page
/// of the solution matches with the most significant page of the snapshot onchain.
pub trait Pagify<T> {
	fn pagify(&self, bound: PageIndex) -> Box<dyn Iterator<Item = (PageIndex, &T)> + '_>;
	fn into_pagify(self, bound: PageIndex) -> Box<dyn Iterator<Item = (PageIndex, T)>>;
}

impl<T> Pagify<T> for Vec<T> {
	fn pagify(&self, bound: PageIndex) -> Box<dyn Iterator<Item = (PageIndex, &T)> + '_> {
		Box::new(
			self.into_iter()
				.enumerate()
				.map(|(p, s)| (p.saturated_into::<PageIndex>(), s))
				.map(move |(p, s)| {
					let bound_usize = bound as usize;
					debug_assert!(self.len() <= bound_usize);
					let padding = bound_usize.saturating_sub(self.len());
					let new_page = p.saturating_add(padding.saturated_into::<PageIndex>());
					(new_page, s)
				}),
		)
	}

	fn into_pagify(self, _: PageIndex) -> Box<dyn Iterator<Item = (PageIndex, T)>> {
		todo!()
	}
}

impl<T: crate::Config> Default for PagedRawSolution<T> {
	fn default() -> Self {
		Self { round: 1, score: Default::default(), solution_pages: Default::default() }
	}
}

impl<T: crate::Config> PagedRawSolution<T> {
	/// Get the total number of voters, assuming that voters in each page are unique.
	pub fn voter_count(&self) -> usize {
		self.solution_pages
			.iter()
			.map(|page| page.voter_count())
			.fold(0usize, |acc, x| acc.saturating_add(x))
	}

	/// Get the total number of winners, assuming that there's only a single page of targets.
	pub fn winner_count_single_page_target_snapshot(&self) -> usize {
		self.solution_pages
			.iter()
			.map(|page| page.unique_targets())
			.into_iter()
			.flatten()
			.collect::<BTreeSet<_>>()
			.len()
	}

	/// Get the total number of edges.
	pub fn edge_count(&self) -> usize {
		self.solution_pages
			.iter()
			.map(|page| page.edge_count())
			.fold(0usize, |acc, x| acc.saturating_add(x))
	}
}

/// Alias for a voter in the npos system.
///
/// This type is bounded.
pub type VoterOf<T> = (
	<T as frame_system::Config>::AccountId,
	sp_npos_elections::VoteWeight,
	BoundedVec<
		<T as frame_system::Config>::AccountId,
		<<T as crate::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter,
	>,
);

/// Alias for a voter in the npos system.
///
/// This type is unbounded.
pub type UnboundedVoterOf<T> = (
	<T as frame_system::Config>::AccountId,
	sp_npos_elections::VoteWeight,
	Vec<<T as frame_system::Config>::AccountId>,
);

/// Encodes the length of a solution or a snapshot.
///
/// This is stored automatically on-chain, and it contains the **size of the entire snapshot**.
/// This is also used in dispatchables as weight witness data and should **only contain the size of
/// the presented solution**, not the entire snapshot.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Debug, Default, TypeInfo, MaxEncodedLen)]
pub struct SolutionOrSnapshotSize {
	/// The length of voters.
	#[codec(compact)]
	pub voters: u32,
	/// The length of targets.
	#[codec(compact)]
	pub targets: u32,
}

/// The type of `Computation` that provided this election data.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, Debug, TypeInfo, MaxEncodedLen)]
pub enum ElectionCompute {
	/// Election was computed on-chain.
	OnChain,
	/// Election was computed with a signed submission.
	Signed,
	/// Election was computed with an unsigned submission.
	Unsigned,
	/// Election was computed with emergency status.
	Emergency,
}

impl Default for ElectionCompute {
	fn default() -> Self {
		ElectionCompute::OnChain
	}
}

/// Current phase of the pallet.
#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, MaxEncodedLen, Debug, TypeInfo)]
pub enum Phase<Bn> {
	/// Nothing, the election is not happening.
	Off,
	/// Signed phase is open.
	Signed,
	/// Unsigned phase. First element is whether it is active or not, second the starting block
	/// number.
	///
	/// We do not yet check whether the unsigned phase is active or passive. The intent is for the
	/// blockchain to be able to declare: "I believe that there exists an adequate signed
	/// solution," advising validators not to bother running the unsigned offchain worker.
	///
	/// As validator nodes are free to edit their OCW code, they could simply ignore this advisory
	/// and always compute their own solution. However, by default, when the unsigned phase is
	/// passive, the offchain workers will not bother running.
	Unsigned((bool, Bn)),
	/// The emergency phase. This is enabled upon a failing call to `T::ElectionProvider::elect`.
	/// After that, the only way to leave this phase is through a successful
	/// `T::ElectionProvider::elect`.
	Emergency,
	/// Snapshot is being created. No other operation is allowed. This can be one or more blocks.
	/// Inner value is `0` if the snapshot is complete and we are ready to move on. Otherwise, it
	/// indicates hte remaining pages for each of which we need 1 block.
	Snapshot(PageIndex),
	/// The first call to `ElectionProvider::elect` has happened, and we are expecting more calls.
	/// No further operation is permitted, freeze all storage items and export `QueuedSolution`.
	/// This can be one or more blocks.
	Export(PageIndex),
}

impl<Bn> Default for Phase<Bn> {
	fn default() -> Self {
		Phase::Off
	}
}

impl<Bn: PartialEq + Eq> Phase<Bn> {
	/// Whether the phase is emergency or not.
	pub fn is_emergency(&self) -> bool {
		matches!(self, Phase::Emergency)
	}

	/// Whether the phase is signed or not.
	pub fn is_signed(&self) -> bool {
		matches!(self, Phase::Signed)
	}

	/// Whether the phase is unsigned or not.
	pub fn is_unsigned(&self) -> bool {
		matches!(self, Phase::Unsigned(_))
	}

	/// Whether the phase is unsigned and open or not, with specific start.
	pub fn is_unsigned_open_at(&self, at: Bn) -> bool {
		matches!(self, Phase::Unsigned((true, real)) if *real == at)
	}

	/// Whether the phase is unsigned and open or not.
	pub fn is_unsigned_open(&self) -> bool {
		matches!(self, Phase::Unsigned((true, _)))
	}

	/// Whether the phase is off or not.
	pub fn is_off(&self) -> bool {
		matches!(self, Phase::Off)
	}
}

#[cfg(test)]
mod pagify {
	use super::Pagify;

	#[cfg(test)]
	fn pagify_works() {
		// is a noop when you have the same length
		assert_eq!(
			vec![10, 11, 12].pagify(3).collect::<Vec<_>>(),
			vec![(0, &10), (1, &11), (2, &12)]
		);

		// pads the values otherwise
		assert_eq!(vec![10, 11].pagify(3).collect::<Vec<_>>(), vec![(1, &10), (2, &11)]);
		assert_eq!(vec![10].pagify(3).collect::<Vec<_>>(), vec![(2, &10)]);
	}
}