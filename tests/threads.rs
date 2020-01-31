extern crate indexed_ring_buffer;
use indexed_ring_buffer::{indexed_ring_buffer, Consumer, Producer, Reader};

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use std::sync::atomic::{AtomicUsize, Ordering};

const SIZE: usize = 100000;
const BUF_SIZE: usize = 200;
const READER_CNT: usize = 100;
const READ_SIZE: usize = 30;
const PREV_ID: usize = 18446744073709551600;
const INIT_ID: usize = 18446744073709551601;

fn reader_thread(n: usize, r: Reader<usize>, p: Arc<Vec<AtomicUsize>>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut recv_data = Vec::with_capacity(SIZE);
        let mut last_id: usize = PREV_ID;
        while recv_data.len() < SIZE {
            if let Some((fm, to, v)) = r.get_from(last_id.wrapping_add(1), READ_SIZE) {
                recv_data.extend_from_slice(&v);
                last_id = to;
                p[n].store(to, Ordering::Release);
            }
            thread::sleep(Duration::from_millis(1));
        }
        let in_data = (0..SIZE).map(|i| i).collect::<Vec<usize>>();
        assert_eq!(in_data, recv_data);
    })
}
#[test]
fn test_thread() {
    let mut progress = Vec::new();
    progress.resize_with(READER_CNT, || AtomicUsize::new(PREV_ID));
    let progress = Arc::new(progress);

    let in_data = (0..SIZE).map(|i| i).collect::<Vec<usize>>();
    let in_data_copy = in_data.clone();
    let arc_data = Arc::new(in_data.clone());
    let mut out_data = Vec::<usize>::with_capacity(SIZE);

    let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(INIT_ID, BUF_SIZE);
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    handles.push(thread::spawn(move || {
        for i in 0..SIZE {
            while !prod.push(in_data_copy[i]) {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }));

    for n in 0..READER_CNT {
        handles.push(reader_thread(n, read.clone(), progress.clone()));
    }
    handles.push(thread::spawn(move || {
        let mut drop_id = PREV_ID;

        while out_data.len() < SIZE {
            if let Some(target) = progress
                .iter()
                .map(|r| r.load(Ordering::Relaxed))
                .filter(|&x| x >= drop_id)
                .min()
            {
                if let Some((id, v)) = cons.shift_to(target) {
                    out_data.extend_from_slice(&v);
                    drop_id = id;
                }
            } else {
                drop_id = 0;
            }
        }
        assert_eq!(in_data, out_data);
    }));

    for handle in handles {
        handle.join().unwrap();
    }
}
