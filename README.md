# DelayQueue for Rust

[![Build Status](https://travis-ci.org/tbcardoso/rust-delay-queue.svg?branch=master)](https://travis-ci.org/tbcardoso/rust-delay-queue)
[![Build status](https://ci.appveyor.com/api/projects/status/7kehrfkbojgaiwyd/branch/master?svg=true)](https://ci.appveyor.com/project/tbcardoso/rust-delay-queue/branch/master)
[![Crates.io](https://img.shields.io/crates/v/delay-queue.svg)](https://crates.io/crates/delay-queue)
[![docs.rs](https://docs.rs/delay-queue/badge.svg)](https://docs.rs/delay-queue/)

A concurrent unbounded blocking queue where each element can only be removed when its delay expires.

## Example

```rust
extern crate delay_queue;

use std::time::{Duration, Instant};
use std::thread;
use delay_queue::{Delay, DelayQueue};

fn main() {
    let queue: DelayQueue<Delay<&str>> = DelayQueue::new();

    // Clone the queue and move it to the consumer thread
    let mut consumer_queue = queue.clone();
    let consumer_handle = thread::spawn(move || {
        // The pop() will block until an item is available and its delay has expired
        println!("First pop: {}", consumer_queue.pop().value); // Prints "First pop: now"
        println!("Second pop: {}", consumer_queue.pop().value); // Prints "Second pop: 3s"
    });

    // Clone the queue and move it to the producer thread
    let mut producer_queue = queue.clone();
    let producer_handle = thread::spawn(move || {
        // This item can only be popped after 3 seconds have passed
        producer_queue.push(Delay::for_duration("3s", Duration::from_secs(3)));

        // This item can be popped immediately
        producer_queue.push(Delay::until_instant("now", Instant::now()));
    });

    consumer_handle.join().unwrap();
    producer_handle.join().unwrap();

    assert!(queue.is_empty());
}
```

You can run this example with the command `cargo run --example basic_usage`


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
