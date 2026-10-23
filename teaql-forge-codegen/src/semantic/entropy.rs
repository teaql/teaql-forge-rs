//! Controlled entropy is only compiled for explicitly enabled parity consumers.
#[cfg(feature = "parity-probes")]
thread_local! {
    static STATE: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

#[cfg(feature = "parity-probes")]
pub struct EntropyGuard(Option<u64>);

#[cfg(feature = "parity-probes")]
impl Drop for EntropyGuard {
    fn drop(&mut self) {
        STATE.with(|state| state.set(self.0));
    }
}

#[cfg(feature = "parity-probes")]
pub fn replay(seed: u64) -> EntropyGuard {
    EntropyGuard(STATE.with(|state| state.replace(Some(seed))))
}

pub(super) fn fill(bytes: &mut [u8]) -> Result<(), String> {
    #[cfg(feature = "parity-probes")]
    if STATE.with(|state| {
        let Some(mut value) = state.get() else {
            return false;
        };
        for byte in bytes.iter_mut() {
            value = value
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *byte = (value >> 56) as u8;
        }
        state.set(Some(value));
        true
    }) {
        return Ok(());
    }
    getrandom::fill(bytes).map_err(|error| error.to_string())
}
