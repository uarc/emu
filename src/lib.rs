use std::io::Read;

/// Core is a trait that defines the standard UARC bus interface with a core for emulation purposes.
/// Interacting with the core is synchronous and thread-safe. Beware of deadlock.
///
/// W is a word type for passing information along the bus
pub trait Core<W> {
    /// The privilege/address is for the core sending.
    fn stream<R: Read>(&mut self, privilege: u8, address: u32, src: &mut R);
    /// The privilege/address is for the core receiving.
    fn incept<R: Read>(&mut self, privilege: u8, address: u32, src: &mut R);
    /// The privilege/address is for the core sending.
    fn send(&mut self, privilege: u8, address: u32, value: W);
    /// This wont complete until it succeeds.
    /// The privilege/address is for the core sending.
    fn kill(&mut self, privilege: u8, address: u32);
    /// This only can determine if the core was running or not, but it is not synchronous.
    fn is_running(&self) -> bool;
}
