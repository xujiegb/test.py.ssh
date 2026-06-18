use rand::RngCore;
use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const COMMIT_HEX: &str = "90243a7416f52151a8c6cecf633500dceb366895";
const FIXED_SUFFIX: [u8; 4] = [0xca, 0x01, 0x01, 0x50];

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn format_uuid(uuid: &[u8; 16]) -> String {
    let h = hex::encode(uuid);
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32],
    )
}

fn check(hash: &[u8]) -> bool {
    // 前 33 bit 全 0 = 前 4 字节全 0，并且第 5 字节最高 bit 为 0
    hash[0] == 0
        && hash[1] == 0
        && hash[2] == 0
        && hash[3] == 0
        && (hash[4] & 0x80) == 0
}

fn worker(prefix_bytes: [u8; 4], found: Arc<AtomicBool>) {
    let mut rng = rand::thread_rng();

    while !found.load(Ordering::Relaxed) {
        let mut uuid = [0u8; 16];
        rng.fill_bytes(&mut uuid);

        // 第一段固定为 commit 后 8 位
        uuid[0..4].copy_from_slice(&prefix_bytes);

        // UUIDv4: 第三段必须是 4xxx
        uuid[6] = (uuid[6] & 0x0f) | 0x40;

        // UUID variant: 第四段必须是 yxxx，y = 8/9/a/b
        uuid[8] = (uuid[8] & 0x3f) | 0x80;

        // 末 8 位固定 ca010150
        uuid[12..16].copy_from_slice(&FIXED_SUFFIX);

        let out = format_uuid(&uuid);

        let mut hasher = Sha512::new();
        hasher.update(out.as_bytes());
        let result = hasher.finalize();

        if check(&result) {
            if !found.swap(true, Ordering::Relaxed) {
                println!("/answer {}", out);
            }
            break;
        }
    }
}

fn main() {
    let prefix = &COMMIT_HEX[COMMIT_HEX.len() - 8..];
    let prefix_vec = hex_to_bytes(prefix);
    let prefix_bytes: [u8; 4] = prefix_vec.try_into().unwrap();

    let cores = num_cpus::get();
    println!("[*] commit suffix: {}", prefix);
    println!("[*] using {} cores", cores);

    let found = Arc::new(AtomicBool::new(false));

    (0..cores).into_par_iter().for_each(|_| {
        worker(prefix_bytes, found.clone());
    });
}
