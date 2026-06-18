use rand::RngCore;
use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const COMMIT_HEX: &str = "90243a7416f52151a8c6cecf633500dceb366895";
const FIXED_SUFFIX: [u8; 4] = [0xd3, 0x8c, 0x59, 0xb6];

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
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

    // 预先分配 UUID 的内部字节数组，并填入固定部分
    let mut uuid = [0u8; 16];
    uuid[0..4].copy_from_slice(&prefix_bytes);
    uuid[12..16].copy_from_slice(&FIXED_SUFFIX);

    // 预先准备好最终 36 字节的 UUID 字符串 Buffer
    let mut out = *b"00000000-0000-0000-0000-000000000000";

    // 初始化首尾固定的 Hex 字符
    // 第一段: commit 后 8 位
    hex::encode_to_slice(&uuid[0..4], &mut out[0..8]).unwrap();
    // 末 8 位: 固定 d38c59b6
    hex::encode_to_slice(&uuid[12..16], &mut out[28..36]).unwrap();

    while !found.load(Ordering::Relaxed) {
        // 只随机化中间的字节（4..12）
        rng.fill_bytes(&mut uuid[4..12]);

        // 维持 UUIDv4 的特性
        uuid[6] = (uuid[6] & 0x0f) | 0x40; // 第三段必须是 4xxx
        uuid[8] = (uuid[8] & 0x3f) | 0x80; // 第四段必须是 yxxx，y = 8/9/a/b

        // 原地更新变动部分的 Hex 字符，零内存分配
        hex::encode_to_slice(&uuid[4..6], &mut out[9..13]).unwrap();
        hex::encode_to_slice(&uuid[6..8], &mut out[14..18]).unwrap();
        hex::encode_to_slice(&uuid[8..10], &mut out[19..23]).unwrap();
        hex::encode_to_slice(&uuid[10..12], &mut out[24..28]).unwrap();

        // 直接对字符数组进行 Hash，避免 String 转换开销
        let mut hasher = Sha512::new();
        hasher.update(&out);
        let result = hasher.finalize();

        if check(&result) {
            // 如果成功找到，且是第一个找到的线程
            if !found.swap(true, Ordering::Relaxed) {
                // 将结果字节数组转换为字符串打印
                let answer = std::str::from_utf8(&out).unwrap();
                println!("/answer {}", answer);
            }
            break;
        }
    }
}

fn main() {
    // 提取 commit 后 8 位
    let prefix = &COMMIT_HEX[COMMIT_HEX.len() - 8..];
    let prefix_vec = hex_to_bytes(prefix);
    let prefix_bytes: [u8; 4] = prefix_vec.try_into().unwrap();

    let cores = num_cpus::get();
    println!("[*] Target prefix: {}", prefix);
    println!("[*] Target suffix: d38c59b6");
    println!("[*] Required PoW : 33 bits leading zero in SHA512");
    println!("[*] Using {} cores for mining...", cores);

    let found = Arc::new(AtomicBool::new(false));

    // 使用 rayon 并发利用所有核心
    (0..cores).into_par_iter().for_each(|_| {
        worker(prefix_bytes, found.clone());
    });
}
