#![feature(portable_simd)]
use axum::extract::Path;
use hyper::{Body, Response};
use memmap2::{Mmap, MmapOptions};
use rayon::prelude::*;
use std::simd::f32x8;
use std::simd::num::SimdFloat;
use std::{fs::File, time::Instant};

pub async fn serve_file(Path(filename): Path<String>) -> Response<Body> {
    let start_time = Instant::now();
    println!("Served {filename} in {:?}", start_time);

    let file = File::open(&format!("./data/{filename}.bin")).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    let data = bytemuck::cast_slice::<u8, f32>(&mmap);
    let bucket_size = 2048;

    let minmax_bytes: Vec<[u8; 8]> = data
        .par_chunks(bucket_size)
        .map(|chunk| {
            const LANES: usize = 8;
            let mut min = f32::MAX;
            let mut max = f32::MIN;
            let mut i = 0;

            let align_offset = chunk.as_ptr().align_offset(std::mem::align_of::<f32x8>());
            let prefix_end = align_offset.min(chunk.len());
            for &val in &chunk[..prefix_end] {
                min = min.min(val);
                max = max.max(val);
            }
            i = prefix_end;

            let mut acc_min = f32x8::splat(f32::MAX);
            let mut acc_max = f32x8::splat(f32::MIN);

            while i + LANES <= chunk.len() {
                let vec = f32x8::from_slice(&chunk[i..i + LANES]);
                acc_min = acc_min.simd_min(vec);
                acc_max = acc_max.simd_max(vec);
                i += LANES;
            }

            min = min.min(acc_min.reduce_min());
            max = max.max(acc_max.reduce_max());

            for &val in &chunk[i..] {
                min = min.min(val);
                max = max.max(val);
            }

            let mut bytes = [0u8; 8];
            bytes[0..4].copy_from_slice(&min.to_le_bytes());
            bytes[4..8].copy_from_slice(&max.to_le_bytes());
            bytes
        })
        .collect();

    let summarized: Vec<u8> = minmax_bytes.into_iter().flatten().collect();

    let x = Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Length", summarized.len())
        .body(Body::from(summarized))
        .unwrap();
    println!("Served {filename} in {:?}", start_time.elapsed());
    x
}
