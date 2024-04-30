# <img src="https://raw.githubusercontent.com/LiveSplit/LiveSplit/master/res/Icon.svg" alt="LiveSplit" height="42" width="45" align="top"/> asr


Helper crate to write auto splitters for LiveSplit One's auto splitting
runtime.

[API Documentation](https://livesplit.org/asr/asr/)

There are two ways of defining an auto splitter.

## Defining an `update` function

You can define an `update` function that will be called every frame. This is
the simplest way to define an auto splitter. The function must have the
following signature:
```rust
#[no_mangle]
pub extern "C" fn update() {}
```

The advantage of this approach is that you have full control over what
happens on every tick of the runtime. However, it's much harder to keep
state around as you need to store all state in global variables as you need
to return out of the function on every tick.

### Example

```rust
#[no_mangle]
pub extern "C" fn update() {
    if let Some(process) = Process::attach("explorer.exe") {
        asr::print_message("Hello World!");
        if let Ok(address) = process.get_module_address("explorer.exe") {
            if let Ok(value) = process.read::<u32>(address) {
                if value > 0 {
                    asr::timer::start();
                }
            }
        }
    }
}
```

## Defining an asynchronous `main` function

You can use the `async_main` macro to define an asynchronous `main`
function.

Similar to using an `update` function, it is important to constantly yield
back to the runtime to communicate that the auto splitter is still alive.
All asynchronous code that you await automatically yields back to the
runtime. However, if you want to write synchronous code, such as the main
loop handling of a process on every tick, you can use the
`next_tick` function to yield back to the runtime and
continue on the next tick.

The main low level abstraction is the `retry` function, which wraps any code
that you want to retry until it succeeds, yielding back to the runtime between
each try.

So if you wanted to attach to a Process you could for example write:

```rust
let process = retry(|| Process::attach("MyGame.exe")).await;
```

This will try to attach to the process every tick until it succeeds. This
specific example is exactly how the `Process::wait_attach` method is
implemented. So if you wanted to attach to any of multiple processes, you could
for example write:

```rust
let process = retry(|| {
   ["a.exe", "b.exe"].into_iter().find_map(Process::attach)
}).await;
```

### Example

Here is a full example of how an auto splitter could look like using the
`async_main` macro:

Usage on stable Rust:
```rust
async_main!(stable);
```

Usage on nightly Rust:
```rust
#![feature(type_alias_impl_trait, const_async_blocks)]

async_main!(nightly);
```

The asynchronous main function itself:
```rust
async fn main() {
    // TODO: Set up some general state and settings.
    loop {
        let process = Process::wait_attach("explorer.exe").await;
        process.until_closes(async {
            // TODO: Load some initial information from the process.
            loop {
                // TODO: Do something on every tick.
               next_tick().await;
            }
        }).await;
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
