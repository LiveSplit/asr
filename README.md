# <img src="https://raw.githubusercontent.com/LiveSplit/LiveSplit/master/LiveSplit/Resources/Icon.png" alt="LiveSplit" height="42" width="45" align="top"/> asr

Helper crate to write auto splitters for LiveSplit One's auto splitting runtime.

## Example

```rust
#[no_mangle]
pub extern "C" fn update() {
    if let Some(process) = Process::attach("Notepad.exe") {
        asr::print_message("Hello World!");
        if let Ok(address) = process.get_module_address("Notepad.exe") {
            if let Ok(value) = process.read::<u32>(address) {
                if value > 0 {
                    asr::timer::start();
                }
            }
        }
    }
}
```

## License

Licensed under either of
  * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
    http://www.apache.org/licenses/LICENSE-2.0)
  * MIT license ([LICENSE-MIT](LICENSE-MIT) or
    http://opensource.org/licenses/MIT) at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
