//! A concurrent unbounded blocking queue where each element can only be removed when
//! its delay expires.

#![warn(missing_docs)]

mod delayed;
mod delay_queue;

pub use delayed::{Delay, Delayed};
pub use delay_queue::DelayQueue;
