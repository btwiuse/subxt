use super::{
	AccountId, Balance, Balances, Call, Event, Origin, ParachainInfo, ParachainSystem, PolkadotXcm, Runtime,
	XcmpQueue,
};
use frame_support::{
	match_types, parameter_types,
	traits::{Everything, Nothing},
	weights::Weight,
};
use frame_support::weights::IdentityFee;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, IsConcrete, NativeAsset, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	UsingComponents,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	// TODO: hack: hardcoded Polkadot?
	pub const RelayNetwork: NetworkId = NetworkId::Rococo;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	pub UniversalLocation: InteriorMultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
	pub MaxAssetsIntoHolding: u32 = 64;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

//TODO: move DenyThenTry to polkadot's xcm module.
// /// Deny executing the xcm message if it matches any of the Deny filter regardless of anything else.
// /// If it passes the Deny, and matches one of the Allow cases then it is let through.
// pub struct DenyThenTry<Deny, Allow>(PhantomData<Deny>, PhantomData<Allow>)
// where
// 	Deny: ShouldExecute,
// 	Allow: ShouldExecute;
//
// impl<Deny, Allow> ShouldExecute for DenyThenTry<Deny, Allow>
// where
// 	Deny: ShouldExecute,
// 	Allow: ShouldExecute,
// {
// 	fn should_execute<Call>(
// 		origin: &MultiLocation,
// 		message: &mut Xcm<Call>,
// 		max_weight: Weight,
// 		weight_credit: &mut Weight,
// 	) -> Result<(), ()> {
// 		Deny::should_execute(origin, message, max_weight, weight_credit)?;
// 		Allow::should_execute(origin, message, max_weight, weight_credit)
// 	}
// }

// TODO: hacked
// See issue #5233
// pub struct DenyReserveTransferToRelayChain;
// impl ShouldExecute for DenyReserveTransferToRelayChain {
// 	fn should_execute<Call>(
// 		origin: &MultiLocation,
// 		message: &mut Xcm<Call>,
// 		_max_weight: Weight,
// 		_weight_credit: &mut Weight,
// 	) -> Result<(), ()> {
// 		if message.0.iter().any(|inst| {
// 			matches!(
// 				inst,
// 				InitiateReserveWithdraw {
// 					reserve: MultiLocation { parents: 1, interior: Here },
// 					..
// 				} | DepositReserveAsset { dest: MultiLocation { parents: 1, interior: Here }, .. } |
// 					TransferReserveAsset {
// 						dest: MultiLocation { parents: 1, interior: Here },
// 						..
// 					}
// 			)
// 		}) {
// 			return Err(()) // Deny
// 		}
//
// 		// An unexpected reserve transfer has arrived from the Relay Chain. Generally, `IsReserve`
// 		// should not allow this, but we just log it here.
// 		if matches!(origin, MultiLocation { parents: 1, interior: Here }) &&
// 			message.0.iter().any(|inst| matches!(inst, ReserveAssetDeposited { .. }))
// 		{
// 			log::warn!(
// 				target: "xcm::barriers",
// 				"Unexpected ReserveAssetDeposited from the Relay Chain",
// 			);
// 		}
// 		// Permit everything else
// 		Ok(())
// 	}
// }

match_types! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Unit, .. }) }
	};
}

// TOOD: hacked
// pub type Barrier = DenyThenTry<
// 	DenyReserveTransferToRelayChain,
// 	(
// 		TakeWeightCredit,
// 		AllowTopLevelPaidExecutionFrom<Everything>,
// 		AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
// 		// ^^^ Parent and its exec plurality get free execution
// 	),
// >;
pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
	// ^^^ Parent & its unit plurality gets free execution
);

/// XCM weigher type.
pub type XcmWeigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;

// TODO: hacked
// pub struct XcmConfig;
// impl xcm_executor::Config for XcmConfig {
// 	type Call = Call;
// 	type XcmSender = XcmRouter;
// 	// How to withdraw and deposit an asset.
// 	type AssetTransactor = LocalAssetTransactor;
// 	type OriginConverter = XcmOriginToTransactDispatchOrigin;
// 	type IsReserve = NativeAsset;
// 	type IsTeleporter = (); // Teleporting is disabled.
// 	type LocationInverter = LocationInverter<Ancestry>;
// 	type Barrier = Barrier;
// 	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
// 	type Trader =
// 		UsingComponents<WeightToFee, RelayLocation, AccountId, Balances, ToAuthor<Runtime>>;
// 	type ResponseHandler = PolkadotXcm;
// 	type AssetTrap = PolkadotXcm;
// 	type AssetClaims = PolkadotXcm;
// 	type SubscriptionService = PolkadotXcm;
// }
pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation of UNIT
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = XcmWeigher;
	type Trader = UsingComponents<IdentityFee<Balance>, RelayLocation, AccountId, Balances, ()>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

// TODO: hacked
// impl pallet_xcm::Config for Runtime {
// 	type Event = Event;
// 	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
// 	type XcmRouter = XcmRouter;
// 	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
// 	type XcmExecuteFilter = Nothing;
// 	// ^ Disable dispatchable execute on the XCM pallet.
// 	// Needs to be `Everything` for local testing.
// 	type XcmExecutor = XcmExecutor<XcmConfig>;
// 	type XcmTeleportFilter = Everything;
// 	type XcmReserveTransferFilter = Nothing;
// 	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
// 	type LocationInverter = LocationInverter<Ancestry>;
// 	type Origin = Origin;
// 	type Call = Call;
//
// 	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
// 	// ^ Override for AdvertisedXcmVersion default
// 	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
// }
impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = XcmWeigher;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = frame_support::traits::ConstU32<8>;
	type UniversalLocation = UniversalLocation;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}