extern crate indexed_ring_buffer;
use criterion::{Criterion,Bencher};
use std::time::Duration;

use indexed_ring_buffer::{indexed_ring_buffer};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::sync::atomic::{AtomicUsize, Ordering};

const SIZE: usize = 10000;
const BUF_SIZE: usize = 500;
const READER_CNT: usize = 100;
const READ_SIZE: usize = 500;
const PREV_ID: usize = 0;
const INIT_ID: usize = 1;


pub fn run_threads(b: &mut Bencher) {
   
    b.iter(|| {
        let (mut prod, mut cons, read) = indexed_ring_buffer::<usize>(INIT_ID, BUF_SIZE);

        let mut progress = Vec::new();
        progress.resize_with(READER_CNT, || AtomicUsize::new(PREV_ID));
        let progress = Arc::new(progress);

         
        let in_data = (0..SIZE).map(|i| i).collect::<Vec<usize>>();
        let in_data_copy = in_data.clone();


        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for n in 0..READER_CNT{
            let r = read.clone();
            let p = progress.clone();
            handles.push(thread::spawn(move||{
                let mut recv_data = Vec::with_capacity(SIZE);
                let mut last_id: usize = PREV_ID;
                while recv_data.len() < SIZE {
                    
                    if let Some((_, to, v)) = r.get_from(last_id.wrapping_add(1), READ_SIZE) {
                            recv_data.extend_from_slice(&v);
                            last_id = to;
                            
                            let v=unsafe{p.get_unchecked(n)};
                            v.store(to, Ordering::Release);
                            

                    }else{
                        thread::sleep(Duration::from_millis(1));
                    }

                }
            }));
        }

        handles.push(thread::spawn(move || {
            let mut out_data = Vec::<usize>::with_capacity(SIZE);
            let mut i = 0_usize;
        
            while out_data.len() < SIZE {
                if i < SIZE && prod.push(in_data_copy[i]){
                    i+=1; 
                    continue;
                }

                if let Some(target) = progress.iter().map(|r| r.load(Ordering::Relaxed)).min(){
                    if let Some((_, v)) = cons.shift_to(target) {
                        out_data.extend_from_slice(&v);
                    }else{
                        thread::sleep(Duration::from_millis(1));

                    }
                }
            }
        }));


        for handle in handles {
            handle.join().unwrap();
        }
    });
}

fn main(){
    let mut c = Criterion::default().sample_size(10);
    c.bench_function("run threads", run_threads);
}
