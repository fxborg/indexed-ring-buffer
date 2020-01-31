extern crate parking_lot;

use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::mem::{self, MaybeUninit};
use std::ops::Range;
use std::sync::Arc;
use std::cmp::Ordering;
// Maximum buffer size
const MAX_BUFFER_SIZE: usize = 2_147_483_647;

/// An indexed multiple readable spsc ring buffer.
///
/// - access by an absolute index.
/// - a Single-Producer Single-Consumer with Multi-Reader
/// - using RwLock of parking_lot
///
/// ```
/// extern crate indexed_ring_buffer;
/// use indexed_ring_buffer::*;
///
/// let (mut p, mut c, r) = indexed_ring_buffer::<usize>(0, 5);
/// for i in 0..101 {
///     p.push(i);
///     c.shift();
/// }
/// for i in 101..106 {
///     p.push(i);
/// }
/// let (start, end, data) = r.get_from(101,5).unwrap();
/// assert_eq!(data,vec![101,102,103,104,105]);
/// assert_eq!(start,101);
/// assert_eq!(end,105);
/// c.shift_to(105);
/// let rslt = r.get_all();
/// ```
/// Ring buffer itself.
pub struct RingBuffer<T> {
    pub(crate) data: UnsafeCell<Box<[MaybeUninit<T>]>>,
    pub(crate) head: RwLock<(usize, usize)>,
    pub(crate) tail: RwLock<usize>,
}

impl<T> RingBuffer<T>
where
    T: Sized + Default + Clone + Copy,
{
    /// Creates a new instance of a ring buffer.
    pub fn new(offset: usize, size: usize) -> RingBuffer<T> {
        let sz = std::cmp::min(MAX_BUFFER_SIZE, size);
        let mut data = Vec::new();
        data.resize_with(sz + 1, MaybeUninit::uninit);

        Self {
            data: UnsafeCell::new(data.into_boxed_slice()),
            head: RwLock::new((offset, 0)),
            tail: RwLock::new(0),
        }
    }
    pub fn get_ref(&self) -> &[MaybeUninit<T>] {
        unsafe { &*self.data.get() }
    }
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut [MaybeUninit<T>] {
        unsafe { &mut *self.data.get() }
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        let (_, head) = *self.head.read();
        let tail = *self.tail.read();
        head == tail
    }

    /// Checks if the ring buffer is full.
    pub fn is_full(&self) -> bool {
        let (_, head) = *self.head.read();
        let tail = *self.tail.read();
        let capacity = self.get_ref().len();
        (tail + 1) % capacity == head
    }
}

///  
struct IndexUtil;
impl IndexUtil {
    /// Returns the ranges.
    pub fn calc_range(head: usize, tail: usize, len: usize) -> (Range<usize>, Range<usize>) {
        match head.partial_cmp(&tail){
            Some(Ordering::Less) =>(head..tail, 0..0),
            Some(Ordering::Greater) =>(head..len, 0..tail),
            Some(Ordering::Equal) => (0..0, 0..0),
            None =>(0..0, 0..0)
        }
    }
    /// Checks if the exists index.
    pub fn exists_index(idx: usize, offset: usize, filled_size: usize) -> Option<usize> {
        let mut rslt = None;
        if idx >= offset {
            let i = idx - offset;
            if i < filled_size {
                rslt = Some(i);
            }
        } else {
            let dist_to_max = usize::max_value() - offset;
            if filled_size - 1 > dist_to_max {
                let over_size = (filled_size - 1) - dist_to_max;
                if idx < over_size {
                    rslt = Some(dist_to_max + 1 + idx);
                }
            }
        }
        rslt
    }
}

/// Producer part of ring buffer.
pub struct Producer<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Producer<T>
where
    T: Sized + Default + Clone + Copy,
{
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// Pushes a new item into the buffer.
    pub fn push(&mut self, v: T) -> bool {
        let head_guard = self.buffer.head.read();
        let head = head_guard.1;
        drop(head_guard);

        let mut tail_guard = self.buffer.tail.write();
        let tail = *tail_guard;
        let mut new_tail = tail + 1;

        let buf: &mut [MaybeUninit<T>] = self.buffer.get_mut();
        let capacity = buf.len();

        if new_tail == capacity {
            new_tail = 0;
        }

        if head == new_tail {
            return false;
        }

        unsafe {
            mem::replace(buf.get_unchecked_mut(tail), MaybeUninit::new(v));
        }

        *tail_guard = new_tail;
        true
    }
}

unsafe impl<T> Sync for Producer<T> {}
unsafe impl<T> Send for Producer<T> {}

/// Consumer part of ring buffer.
pub struct Consumer<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Consumer<T>
where
    T: Sized + Default + Clone + Copy,
{
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }

    /// Removes and returns to multiple items from the buffer
    pub fn shift_to(&mut self, to: usize) -> Option<(usize, Vec<T>)> {
        let tail_guard = self.buffer.tail.read();
        let tail = *tail_guard;
        drop(tail_guard);

        let mut head_guard = self.buffer.head.write();
        let (offset, head) = *head_guard;

        if head == tail {
            return None;
        }

        let buf: &mut [MaybeUninit<T>] = self.buffer.get_mut();
        let capacity = buf.len();
        let filled_size = (tail + capacity - head) % capacity;
        let rslt = IndexUtil::exists_index(to, offset, filled_size);
        let i = rslt?;
        let new_offset = to.wrapping_add(1);
        let new_head = (head + i + 1) % capacity;

        let (a, b) = IndexUtil::calc_range(head, new_head, capacity);
        let mut temp_a = Vec::new();
        let mut temp_b = Vec::new();
        temp_a.resize_with(a.len(), MaybeUninit::uninit);
        temp_b.resize_with(b.len(), MaybeUninit::uninit);

        buf[a].swap_with_slice(&mut temp_a);
        buf[b].swap_with_slice(&mut temp_b);

        let temp = [temp_a, temp_b].concat();
        let v: Vec<T> = unsafe { mem::transmute(temp) };

        *head_guard = (new_offset, new_head);

        Some((to, v))
    }

    /// Removes and returns the first item from the buffer
    pub fn shift(&mut self) -> Option<(usize, T)> {
        let tail_guard = self.buffer.tail.read();
        let tail = *tail_guard;
        drop(tail_guard);

        let mut head_guard = self.buffer.head.write();
        let (offset, head) = *head_guard;

        if head == tail {
            return None;
        }

        let mut new_head = head + 1;

        let buf: &mut [MaybeUninit<T>] = self.buffer.get_mut();
        let capacity = buf.len();
        if new_head == capacity {
            new_head = 0;
        }

        let mut temp = MaybeUninit::uninit();

        mem::swap(unsafe { buf.get_unchecked_mut(head) }, &mut temp);
        let temp = unsafe { temp.assume_init() };

        *head_guard = (offset.wrapping_add(1), new_head);

        Some((offset, temp))
    }
}

unsafe impl<T> Sync for Consumer<T> {}
unsafe impl<T> Send for Consumer<T> {}

/// Reader part of ring buffer.
#[derive(Clone)]
pub struct Reader<T> {
    buffer: Arc<RingBuffer<T>>,
}

impl<T> Reader<T>
where
    T: Sized + Default + Clone + Copy,
{
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.is_full()
    }
    ///　Returns the single item from the buffer.
    pub fn get(&self, idx: usize) -> Option<(usize, T)> {
        let (offset, head, tail) = self.read_index();
        if head == tail {
            return None;
        }
        let buf: &[MaybeUninit<T>] = self.buffer.get_ref();
        let capacity = buf.len();
        let filled_size = (tail + capacity - head) % capacity;
        let pos;
        if let Some(i) = IndexUtil::exists_index(idx, offset, filled_size) {
            pos = (head + i) % capacity;
        } else {
            return None;
        }
        let v: &T = unsafe {
            &*(buf.get_unchecked(pos) as *const std::mem::MaybeUninit<T> as *const T)
        };
        Some((idx, *v))
    }

    ///　Returns the all items from the buffer.
    pub fn get_all(&self) -> Option<(usize, usize, Vec<T>)> {
        let (offset, head, tail) = self.read_index();

        let buf: &[MaybeUninit<T>] = self.buffer.get_ref();
        let capacity = buf.len();
        let (a, b) = IndexUtil::calc_range(head, tail, capacity);

        let buf_a: &[T] = unsafe { 
            &*(&buf[a] as *const [std::mem::MaybeUninit<T>] as *const [T])
        };
        let buf_b: &[T] = unsafe {
            &*(&buf[b] as *const [std::mem::MaybeUninit<T>] as *const [T])
        };
        let v = [buf_a, buf_b].concat().to_vec();
        if !v.is_empty(){
            Some((offset, offset.wrapping_add(v.len() - 1), v))
        } else {
            None
        }
    }

    ///　Returns the range items from the buffer.
    pub fn get_from(&self, idx: usize, len: usize) -> Option<(usize, usize, Vec<T>)> {
        let (offset, head, tail) = self.read_index();
        if head == tail {
            return None;
        }
        let buf: &[MaybeUninit<T>] = self.buffer.get_ref();
        let capacity = buf.len();
        let filled_size = (tail + capacity - head) % capacity;

        let range_head;
        let range_tail;

        if let Some(i1) = IndexUtil::exists_index(idx, offset, filled_size) {
            range_head = (head + i1) % capacity;
            if len == 0 || i1 + len > filled_size {
                range_tail = tail;
            } else {
                range_tail = (head + i1 + len) % capacity;
            }
        } else {
            return None;
        }

        let (a, b) = IndexUtil::calc_range(range_head, range_tail, capacity);
        let buf_a: &[T] = unsafe {
            &*(&buf[a] as *const [std::mem::MaybeUninit<T>] as *const [T])
        };
        let buf_b: &[T] = unsafe {
            &*(&buf[b] as *const [std::mem::MaybeUninit<T>] as *const [T])
        };

        let v = [buf_a, buf_b].concat().to_vec();
        let v_len = v.len();
        if v_len > 0 {
            Some((idx, idx.wrapping_add(v_len - 1), v))
        } else {
            None
        }
    }

    fn read_index(&self) -> (usize, usize, usize) {
        let (offset, head) = *self.buffer.head.read();
        let tail = *self.buffer.tail.read();
        (offset, head, tail)
    }
}

unsafe impl<T> Sync for Reader<T> {}
unsafe impl<T> Send for Reader<T> {}

/// Create a new Indexed ring buffer sets.
pub fn indexed_ring_buffer<T>(
    initial_index: usize,
    capacity: usize,
) -> (Producer<T>, Consumer<T>, Reader<T>)
where
    T: Sized + Default + Clone + Copy,
{
    let rb = Arc::new(RingBuffer::<T>::new(initial_index, capacity));

    let tx = Producer::<T> { buffer: rb.clone() };

    let rx = Consumer::<T> { buffer: rb.clone() };

    let rdr = Reader::<T> { buffer: rb };

    (tx, rx, rdr)
}
