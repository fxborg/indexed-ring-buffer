extern crate indexed_ring_buffer;

use indexed_ring_buffer::{indexed_ring_buffer, Consumer, Producer, Reader};

#[test]
fn test_create() {
    let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(0, 10);
    assert!(prod.is_empty());
    assert!(cons.is_empty());
    assert!(read.is_empty());
    assert!(!prod.is_full());
    assert!(!cons.is_full());
    assert!(!read.is_full());
}

#[test]
fn test_queue() {
    let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(0, 5);

    assert!(prod.push(0));
    assert!(prod.push(1));
    assert!(prod.push(2));
    assert!(prod.push(3));
    assert!(prod.push(4));

    assert_eq!(read.get(3), Some((3, 3)));
    assert_eq!(read.get_all(), Some((0, 4, [0, 1, 2, 3, 4].to_vec())));
    assert_eq!(read.get_from(1, 3), Some((1, 3, [1, 2, 3].to_vec())));
    assert_eq!(read.get_from(1, 1), Some((1, 1, [1].to_vec())));
    assert_eq!(read.get_from(1, 2), Some((1, 2, [1, 2].to_vec())));
    assert_eq!(read.get_from(1, 4), Some((1, 4, [1, 2, 3, 4].to_vec())));

    assert_eq!(cons.shift_to(3), Some((3_usize, vec![0, 1, 2, 3])));
    assert_eq!(cons.shift_to(13), None);
    assert_eq!(cons.shift(), Some((4, 4)));
    assert_eq!(cons.shift(), None);
}

#[test]
fn test_full_and_empty() {
    let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(0, 5);

    assert!(prod.push(0));
    assert!(prod.push(1));
    assert!(prod.push(2));
    assert!(prod.push(3));

    assert!(!prod.is_empty());
    assert!(!cons.is_empty());
    assert!(!read.is_empty());
    assert!(!prod.is_full());
    assert!(!cons.is_full());
    assert!(!read.is_full());

    assert!(prod.push(4));

    assert!(prod.is_full());
    assert!(cons.is_full());
    assert!(read.is_full());

    assert_eq!(cons.shift(), Some((0, 0)));

    assert!(!prod.is_full());
    assert!(!cons.is_full());
    assert!(!read.is_full());

    assert_eq!(cons.shift_to(2), Some((2_usize, vec![1, 2])));
    assert_eq!(cons.shift(), Some((3, 3)));
    assert_eq!(cons.shift(), Some((4, 4)));

    assert!(prod.is_empty());
    assert!(cons.is_empty());
    assert!(read.is_empty());
    assert!(!prod.is_full());
    assert!(!cons.is_full());
    assert!(!read.is_full());
}

#[test]
fn test_wrapping_add() {
    let mut n = usize::max_value() - 4;
    let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(n, 10);

    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    n = n.wrapping_add(1);
    assert!(prod.push(n));
    assert_eq!(
        read.get(18446744073709551611),
        Some((18446744073709551611, 18446744073709551611))
    );
    assert_eq!(
        read.get(18446744073709551612),
        Some((18446744073709551612, 18446744073709551612))
    );
    assert_eq!(
        read.get(18446744073709551613),
        Some((18446744073709551613, 18446744073709551613))
    );
    assert_eq!(
        read.get(18446744073709551614),
        Some((18446744073709551614, 18446744073709551614))
    );
    assert_eq!(
        read.get(18446744073709551615),
        Some((18446744073709551615, 18446744073709551615))
    );

    assert_eq!(read.get(0), Some((0, 0)));
    assert_eq!(read.get(1), Some((1, 1)));
    assert_eq!(read.get(2), Some((2, 2)));
    assert_eq!(read.get(3), Some((3, 3)));
}
