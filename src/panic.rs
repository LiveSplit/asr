/// Defines a panic handler for the auto splitter that aborts execution. By
/// default it will only print the panic message in debug builds. A stack based
/// buffer of 1024 bytes is used by default. If the message is too long, it will
/// be truncated. All of this can be configured.
///
/// # Usage
///
/// ```no_run
/// asr::panic_handler! {
///     /// When to print the panic message.
///     /// Default: debug
///     print: never | debug | always,
///
///     /// The size of the stack based buffer in bytes.
///     /// Default: 1024
///     buffer: <number>,
/// }
/// ```
///
/// # Example
///
/// The default configuration will print a message in debug builds:
/// ```no_run
/// asr::panic_handler!();
/// ```
///
/// A message will always be printed with a buffer size of 512 bytes:
/// ```no_run
/// asr::panic_handler! {
///     print: always,
///     buffer: 512,
/// }
/// ```
///
/// A message will always be printed with the default buffer size:
/// ```no_run
/// asr::panic_handler! {
///     print: always,
/// }
/// ```
///
/// A message will never be printed:
/// ```no_run
/// asr::panic_handler! {
///     print: never,
/// }
/// ```
///
/// A message will be printed in debug builds, with a buffer size of 512 bytes:
/// ```no_run
/// asr::panic_handler! {
///     buffer: 512,
/// }
/// ```
#[macro_export]
macro_rules! panic_handler {
    () => { $crate::panic_handler!(print: debug); };
    (print: never $(,)?) => {
        #[cfg(all(not(test), target_family = "wasm"))]
        #[panic_handler]
        fn panic(_: &core::panic::PanicInfo) -> ! {
            #[cfg(target_arch = "wasm32")]
            core::arch::wasm32::unreachable();
            #[cfg(target_arch = "wasm64")]
            core::arch::wasm64::unreachable();
        }
    };
    (buffer: $N:expr $(,)?) => { $crate::panic_handler!(print: debug, buffer: $N); };
    (print: always $(,)?) => { $crate::panic_handler!(print: always, buffer: 1024); };
    (print: always, buffer: $N:expr $(,)?) => {
        #[cfg(all(not(test), target_family = "wasm"))]
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            asr::print_limited::<$N>(info);
            #[cfg(target_arch = "wasm32")]
            core::arch::wasm32::unreachable();
            #[cfg(target_arch = "wasm64")]
            core::arch::wasm64::unreachable();
        }
    };
    (print: debug $(,)?) => { $crate::panic_handler!(print: debug, buffer: 1024); };
    (print: debug, buffer: $N:expr $(,)?) => {
        #[cfg(all(not(test), target_family = "wasm"))]
        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            #[cfg(debug_assertions)]
            asr::print_limited::<$N>(_info);
            #[cfg(target_arch = "wasm32")]
            core::arch::wasm32::unreachable();
            #[cfg(target_arch = "wasm64")]
            core::arch::wasm64::unreachable();
        }
    };
}
