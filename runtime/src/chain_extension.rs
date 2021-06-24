#[cfg_attr(not(feature="std"), no_std)]
use codec::Encode;
use frame_support::log::{
    error,
    trace,
    debug,
    info,
};
use pallet_contracts::chain_extension::{
    ChainExtension,
    Environment,
    Ext,
    InitState,
    RetVal,
    SysConfig,
    UncheckedFrom,
};
use sp_runtime::DispatchError;

/// Contract extension for `FetchRandom`
pub struct FetchPriceExtension;

impl <C: pallet_contracts::Config> ChainExtension<C> for FetchPriceExtension {
    fn call<E: Ext>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<RetVal, DispatchError>
    where
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        match func_id {
            1101 => {
                log::info!("enter extension");
                let mut env = env.buf_in_buf_out();
                log::info!("initial buffer");
                let price = crate::PriceOcw::lastest_price();
                log::info!("get price");
                let price_slice = price.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:}",
                    func_id
                );
                log::warn!("price:::::::::{:?}",price);
                log::warn!("price:::::::::{:?}",price_slice);
                log::info!("print trace");
                env.write(&price_slice, false, None).map_err(|_| {
                    log::info!("write into buffer");
                    DispatchError::Other("ChainExtension failed to call random")
                })?;
                log::info!("exit");
            }

            _ => {
                error!("Called an unregistered `func_id`: {:}", func_id);
                return Err(DispatchError::Other("Unimplemented func_id"))
            }
        }
        Ok(RetVal::Converging(0))
    }

    fn enabled() -> bool {
        true
    }
}
