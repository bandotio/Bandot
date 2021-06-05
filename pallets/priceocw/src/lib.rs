
#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SendSignedTransaction,
		SignedPayload, SigningTypes, Signer, SubmitTransaction,
	}
};
use frame_support::traits::Get;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	RuntimeDebug,
	offchain::{http, Duration, storage::StorageValueRef},
	traits::Zero,
	transaction_validity::{InvalidTransaction, ValidTransaction, TransactionValidity},
};
use codec::{Encode, Decode};
use sp_std::vec::Vec;
use lite_json::json::JsonValue;

#[cfg(test)]
mod tests;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dot!");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSigner,
		MultiSignature,
	};
	use sp_core::sr25519::Signature as Sr25519Signature;
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use super::*;

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;

		// Configuration parameters

		/// A grace period after we send transaction.
		///
		/// To avoid sending too many transactions, we only attempt to send one
		/// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
		/// sending between distinct runs of this offchain worker.
		#[pallet::constant]
		type GracePeriod: Get<Self::BlockNumber>;

		/// Number of blocks of cooldown after unsigned transaction is included.
		///
		/// This ensures that we only accept unsigned transactions once, every `UnsignedInterval` blocks.
		#[pallet::constant]
		type UnsignedInterval: Get<Self::BlockNumber>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			// Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM. The `sp-api` crate also provides a feature `disable-logging` to disable
			// all logging and thus, remove any logging from the WASM.
			log::info!("Hello World from offchain workers!");

			// Since off-chain workers are just part of the runtime code, they have direct access
			// to the storage and other included pallets.
			//
			// We can easily import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			// It's a good practice to keep `fn offchain_worker()` function minimal, and move most
			// of the code to separate `impl` block.
			// Here we call a helper function to calculate current average price.
			// This function reads storage entries of the current state.
			let average: Option<u32> = Self::average_price();
			log::debug!("Current price: {:?}", average);

			// For this example we are going to send both signed and unsigned transactions
			// depending on the block number.
			// Usually it's enough to choose one or the other.
			
			let res = Self::fetch_price_and_send_unsigned_for_any_account(block_number);
				
			if let Err(e) = res {
				log::error!("Error: {}", e);
			}
		}
	}

	/// A public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit new price to the list.
		///
		/// This method is a public function of the module and can be called from within
		/// a transaction. It appends given `price` to current list of prices.
		/// In our example the `offchain worker` will create, sign & submit a transaction that
		/// calls this function passing the price.
		///
		/// The transaction needs to be signed (see `ensure_signed`) check, so that the caller
		/// pays a fee to execute it.
		/// This makes sure that it's not easy (or rather cheap) to attack the chain by submitting
		/// excesive transactions, but note that it doesn't ensure the price oracle is actually
		/// working and receives (and provides) meaningful data.
		/// This example is not focused on correctness of the oracle itself, but rather its
		/// purpose is to showcase offchain worker capabilities.
		
	

		#[pallet::weight(0)]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, T::BlockNumber>,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// Add the price to the on-chain list, but mark it as coming from an empty address.
			Self::add_price(Default::default(), price_payload.price);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
			<LastestPrice<T>>::put(price_payload.price);
			Ok(().into())
		}
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event generated when new price is accepted to contribute to the average.
		/// \[price, who\]
		NewPrice(u32, T::AccountId),
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(
			_source: TransactionSource,
			call: &Self::Call,
		) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::submit_price_unsigned_with_signed_payload(
				ref payload, ref signature
			) = call {
				let signature_valid = SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}
				Self::validate_transaction_parameters(&payload.block_number, &payload.price)
			} 
			else {
				InvalidTransaction::Call.into()
			}
		}
	}

	/// A vector of recently submitted prices.
	///
	/// This is used to calculate average price, should have bounded size.
	#[pallet::storage]
	#[pallet::getter(fn prices)]
	pub(super) type Prices<T: Config> = StorageValue<_, Vec<u32>, ValueQuery>;

	/// Defines the block when next unsigned transaction will be accepted.
	///
	/// To prevent spam of unsigned (and unpayed!) transactions on the network,
	/// we only allow one transaction every `T::UnsignedInterval` blocks.
	/// This storage entry defines when new transaction is going to be accepted.
	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;


	///Define the lastest price for contract to read
	#[pallet::storage]
	#[pallet::getter(fn lastest_price)]
	pub(super) type LastestPrice<T: Config> = StorageValue<_, u32, ValueQuery>;
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayload<Public, BlockNumber> {
	block_number: BlockNumber,
	price: u32,
	public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PricePayload<T::Public, T::BlockNumber> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

enum TransactionType {
	Signed,
	UnsignedForAny,
	UnsignedForAll,
	Raw,
	None,
}

impl<T: Config> Pallet<T> {
	



	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_any_account(block_number: T::BlockNumber) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction")
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = match Self::fetch_price(){
			Ok(price) => price,
			//if fetch price err the lastest price should be the average price
			Err(_)	=> Self::average_price().unwrap_or(0),
		};
		


		// -- Sign using any account
		let (_, result) = Signer::<T, T::AuthorityId>::any_account().send_unsigned_transaction(
			|account| PricePayload {
				price,
				block_number,
				public: account.public.clone()
			},
			|payload, signature| {
				Call::submit_price_unsigned_with_signed_payload(payload, signature)
			}
		).ok_or("No local accounts accounts available.")?;
		result.map_err(|()| "Unable to submit transaction")?;

		Ok(())
	}



	/// Fetch current price and return the result in cents.
	fn fetch_price() -> Result<u32, http::Error> {
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
		// Initiate an external HTTP GET request.
		// This is using high-level wrappers from `sp_runtime`, for the low-level calls that
		// you can find in `sp_io`. The API is trying to be similar to `reqwest`, but
		// since we are running in a custom WASM execution environment we can't simply
		// import the library here.
		let request = http::Request::get(
			"https://min-api.cryptocompare.com/data/price?fsym=DOT&tsyms=USD"
		);
		// We set the deadline for sending of the request, note that awaiting response can
		// have a separate deadline. Next we send the request, before that it's also possible
		// to alter request headers or stream body content in case of non-GET requests.
		let pending = request
			.deadline(deadline)
			.send()
			.map_err(|_| http::Error::IoError)?;

		// The request is already being processed by the host, we are free to do anything
		// else in the worker (we can send multiple concurrent requests too).
		// At some point however we probably want to check the response though,
		// so we can block current thread and wait for it to finish.
		// Note that since the request is being driven by the host, we don't have to wait
		// for the request to have it complete, we will just not read the response.
		let response = pending.try_wait(deadline)
			.map_err(|_| http::Error::DeadlineReached)??;
		// Let's check the status code before we proceed to reading the response.
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown);
		}

		// Next we want to fully read the response body and collect it to a vector of bytes.
		// Note that the return object allows you to read the body in chunks as well
		// with a way to control the deadline.
		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		let price = match Self::parse_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			}
		}?;

		log::warn!("Got price: {} cents", price);

		Ok(price)
	}

	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `None` when parsing failed or `Some(price in cents)` when parsing is successful.
	fn parse_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter()
					.find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.checked_sub(2).unwrap_or(0);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

	/// Add new price to the list.
	fn add_price(who: T::AccountId, price: u32) {
		log::info!("Adding to the average: {}", price);
		<Prices<T>>::mutate(|prices| {
			const MAX_LEN: usize = 64;

			if prices.len() < MAX_LEN {
				prices.push(price);
			} else {
				prices[price as usize % MAX_LEN] = price;
			}
		});

		let average = Self::average_price()
			.expect("The average is not empty, because it was just mutated; qed");
		log::info!("Current average price is: {}", average);
		// here we are raising the NewPrice event
		Self::deposit_event(Event::NewPrice(price, who));
	}

	/// Calculate current average price.
	fn average_price() -> Option<u32> {
		let prices = <Prices<T>>::get();
		if prices.is_empty() {
			None
		} else {
			Some(prices.iter().fold(0_u32, |a, b| a.saturating_add(*b)) / prices.len() as u32)
		}
	}

	fn validate_transaction_parameters(
		block_number: &T::BlockNumber,
		new_price: &u32,
	) -> TransactionValidity {
		// Now let's check if the transaction has any chance to succeed.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if &next_unsigned_at > block_number {
			return InvalidTransaction::Stale.into();
		}
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into();
		}

		// We prioritize transactions that are more far away from current average.
		//
		// Note this doesn't make much sense when building an actual oracle, but this example
		// is here mostly to show off offchain workers capabilities, not about building an
		// oracle.
		let avg_price = Self::average_price()
			.map(|price| if &price > new_price { price - new_price } else { new_price - price })
			.unwrap_or(0);

		ValidTransaction::with_tag_prefix("ExampleOffchainWorker")
			// We set base priority to 2**20 and hope it's included before any other
			// transactions in the pool. Next we tweak the priority depending on how much
			// it differs from the current average. (the more it differs the more priority it
			// has).
			.priority(T::UnsignedPriority::get().saturating_add(avg_price as _))
			// This transaction does not require anything else to go before into the pool.
			// In theory we could require `previous_unsigned_at` transaction to go first,
			// but it's not necessary in our case.
			//.and_requires()
			// We set the `provides` tag to be the same as `next_unsigned_at`. This makes
			// sure only one transaction produced after `next_unsigned_at` will ever
			// get to the transaction pool and will end up in the block.
			// We can still have multiple transactions compete for the same "spot",
			// and the one with higher priority will replace other one in the pool.
			.and_provides(next_unsigned_at)
			// The transaction is only valid for next 5 blocks. After that it's
			// going to be revalidated by the pool.
			.longevity(5)
			// It's fine to propagate that transaction to other peers, which means it can be
			// created even by nodes that don't produce blocks.
			// Note that sometimes it's better to keep it for yourself (if you are the block
			// producer), since for instance in some schemes others may copy your solution and
			// claim a reward.
			.propagate(true)
			.build()
	}
}