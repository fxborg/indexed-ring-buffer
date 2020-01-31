Indexed Ring Buffer
====
An indexed multiple readable spsc ring buffer.  

# Overview
 - access by an absolute index.
 - a single-producer single-consumer with multi-reader.
 - using RwLock of parking_lot.

# Examples
```rust
extern crate indexed_ring_buffer;
use indexed_ring_buffer::*;

let (mut p, mut c, r) = indexed_ring_buffer::<usize>(0, 5);
for i in 0..101 {
    p.push(i);
    c.shift();
}
for i in 101..106 {
    p.push(i);
}
let (start, end, data) = r.get_from(101,5).unwrap();
assert_eq!(data,vec![101,102,103,104,105]);
assert_eq!(start,101);
assert_eq!(end,105);
c.shift_to(105);
let rslt = r.get_all();
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
