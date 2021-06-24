#![cfg_attr(not(feature = "std"), no_std)]
use ink_env::Environment;
use ink_lang as ink;


#[ink::chain_extension]
pub trait FetchPrice{
    type ErrorCode = PriceReadErr;

    #[ink(extension = 1101, returns_result = false, handle_status = false)]
    fn fetch_price() -> [u8;4];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PriceReadErr {
    FailGetPrice,
}

impl ink_env::chain_extension::FromStatusCode for PriceReadErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::FailGetPrice),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;
    

    type ChainExtension = FetchPrice;
}




#[ink::contract(env= crate::CustomEnvironment)]
mod pricetest {
    use super::PriceReadErr;
   
    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Pricetest {
        
        value: [u8;4],
        
    }

    #[ink(event)]
    pub struct PriceUpdated{
        #[ink(topic)]
        new: [u8;4],
    }

    impl Pricetest {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(init_value: [u8;4]) -> Self {
            Self { value: init_value }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        /// A message that can be called on instantiated contracts.
        /// This one flips the value of the stored `bool` from `true`
        /// to `false` and vice versa.
        #[ink(message)]
        pub fn update(&mut self) -> Result<(), PriceReadErr>{
            let new_price = self.env().extension().fetch_price();
            
            self.value = new_price;
            self.env().emit_event(PriceUpdated{new: new_price});
            Ok(())
        }

        /// Simply returns the current value of our `bool`.
        #[ink(message)]
        pub fn get(&self) -> (u32,u32,u32) {
            u32::from_le_bytes(self.value)
            
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            let pricetest = Pricetest::default();
            assert_eq!(pricetest.get(), 0);
        }

    }
}
