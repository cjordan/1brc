use std::{fs::File, path::PathBuf};

use memmap2::MmapOptions;

#[allow(unused_imports)]
use one_brc::{print, read_mmap, read_mmap_unsafe, read_naive};

const MEASUREMENTS: &str = "measurements.txt";

fn main() {
    let f = PathBuf::from(MEASUREMENTS);
    if !f.exists() {
        eprintln!("expected to find '{MEASUREMENTS}' in the 'rust' directory; cannot continue");
        std::process::exit(1);
    }

    #[cfg(feature = "timings")]
    let duration = std::time::Instant::now();

    let f = File::open(MEASUREMENTS).unwrap();
    #[allow(unused_variables)]
    let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };

    // let map = read_naive(f);
    // let map = read_mmap(&mmap);
    let map = read_mmap_unsafe(&mmap);

    #[cfg(feature = "timings")]
    println!("reading: {:?}", duration.elapsed());

    #[cfg(feature = "timings")]
    let duration = std::time::Instant::now();

    print(map);

    #[cfg(feature = "timings")]
    println!("printing: {:?}", duration.elapsed());
}
