//! Here's my attempt to optimise the "1 billion row challenge" without
//! sacrificing too much readability and not using parallelism.
//!
//! Inspiration taken from
//! <https://github.com/tumdum/1brc/blob/main/src/main.rs> and
//! <https://github.com/thebracket/one_billion_rows/blob/main/having_fun/src/lib.rs>

use std::{
    fmt::Display,
    fs::File,
    io::{BufRead, BufReader},
};

use bstr::BStr;
use memmap2::Mmap;

// FxHashMap is noticably faster than a vanilla HashMap.
use rustc_hash::FxHashMap as HashMap;
// use std::collections::HashMap;

#[derive(Debug)]
pub struct CityDetails {
    /// The minimum * 10
    min: i16,

    /// The maximum * 10
    max: i16,

    /// The sum * 10
    sum: i32,

    /// The number of measurements
    count: u32,
}

impl Default for CityDetails {
    fn default() -> Self {
        CityDetails {
            min: i16::MAX,
            max: i16::MIN,
            sum: 0,
            count: 0,
        }
    }
}

impl Display for CityDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}/{:.1}/{:.1}",
            self.min as f64 / 10.0,
            self.sum as f64 / self.count as f64 / 10.0,
            self.max as f64 / 10.0
        )
    }
}

impl CityDetails {
    fn update(&mut self, meas: i16) {
        if self.min > meas {
            self.min = meas;
        }
        if self.max < meas {
            self.max = meas;
        }
        self.sum += i32::from(meas);
        self.count += 1;
    }
}

// Strategy:
// - Use a HashMap over a BTreeMap. The BTreeMap has the advantage of sorting
//   the names for us, but is (empirically) much more expensive than a HashMap
//   to access entries from. This even includes the cost of creating a Vec from
//   the HashMap then sorting it.
// - Use a BufReader to access lines.
// - Re-use a pre-allocated String for each line.
// - Read the measurements as floats, but convert them to ints. We know that all
//   measurements have only 1 decimal place, so we can multiply the values by 10
//   and use small ints to get fast checks and arithmetic.
//
// Remarks: The performance is actually pretty good. However, using specialised
// routines (e.g. memchr, manually parsing bytes as an int vs. parsing as a
// float then casting to int) makes things significantly faster.
pub fn read_naive(file: File) -> Vec<(String, CityDetails)> {
    let mut map: HashMap<String, CityDetails> = HashMap::default();

    let mut f = BufReader::new(file);
    let mut line = String::new();
    while f.read_line(&mut line).unwrap() > 0 {
        let (city, meas) = line.split_once(';').unwrap();
        let meas = (meas.trim_end().parse::<f64>().unwrap() * 10.0) as i16;
        map.entry(city.to_string()).or_default().update(meas);

        line.clear();
    }

    let mut map = map.into_iter().collect::<Vec<_>>();
    map.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    map
}

fn parse_digits(bytes: &[u8]) -> i16 {
    let mut mul = 1;
    let mut acc = 0;
    for byte in bytes.iter().copied() {
        match byte {
            b'0'..=b'9' => {
                acc = acc * 10 + i16::from(byte - b'0');
            }

            b'-' => {
                mul = -1;
            }

            b'.' => (),
            _ => (),
        }
    }
    mul * acc
}

// Strategy:
// - Use an mmap to access the file. This has the advantage of avoiding String
//   allocations and directly references bytestrings.
// - Use memchr to find delimiters. This is much faster than naively matching
//   for the characters with "vanilla Rust".
// - Manually read the numeric component in as an int. Another small speed
//   boost.
//
// Remarks: While only single-threaded, this seems to be pretty fast (roughly 2x
// faster than the naive function above).
pub fn read_mmap(mmap: &Mmap) -> Vec<(&BStr, CityDetails)> {
    let mut map: HashMap<&BStr, CityDetails> = HashMap::default();

    let mut city: &BStr;
    let mut numeric: &BStr;
    let mut city_start = 0;
    let mut numeric_start;
    while city_start < mmap.len() {
        let i = memchr::memchr(b';', &mmap[city_start..]).unwrap();
        city = (&mmap[city_start..city_start + i]).into();
        numeric_start = city_start + i + 1;
        let i = memchr::memchr(b'\n', &mmap[numeric_start..]).unwrap();
        numeric = (&mmap[numeric_start..numeric_start + i]).into();
        city_start = numeric_start + i + 1;

        let meas = parse_digits(numeric);
        map.entry(city).or_default().update(meas);
    }

    let mut map = map.into_iter().collect::<Vec<_>>();
    map.sort_unstable_by(|a, b| a.0.cmp(b.0));
    map
}

// Strategy:
// - The same as above, but using unsafe variants. This is fine so long as the
//   input file is correctly formatted.
//
// Remarks: This seems to make performance very slightly better.
pub fn read_mmap_unsafe(mmap: &Mmap) -> Vec<(&BStr, CityDetails)> {
    let mut map: HashMap<&BStr, CityDetails> = HashMap::default();

    let mut city: &BStr;
    let mut numeric: &BStr;
    let mut city_start = 0;
    let mut numeric_start;
    unsafe {
        while city_start < mmap.len() {
            let i = memchr::memchr(b';', mmap.get_unchecked(city_start..)).unwrap_unchecked();
            city = mmap.get_unchecked(city_start..city_start + i).into();
            numeric_start = city_start + i + 1;
            let i = memchr::memchr(b'\n', mmap.get_unchecked(numeric_start..)).unwrap_unchecked();
            numeric = mmap.get_unchecked(numeric_start..numeric_start + i).into();
            city_start = numeric_start + i + 1;

            let meas = parse_digits(numeric);
            map.entry(city).or_default().update(meas);
        }
    }

    let mut map = map.into_iter().collect::<Vec<_>>();
    map.sort_unstable_by(|a, b| a.0.cmp(b.0));
    map
}

pub fn print(map: impl IntoIterator<Item = (impl Display, CityDetails)>) {
    let mut map = map.into_iter();

    print!("{{");
    if let Some((city, details)) = map.next() {
        print!("{city}={}", details);
    }
    for (city, details) in map {
        print!(", {city}={}", details);
    }
    println!("}}");
}
