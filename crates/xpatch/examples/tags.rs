use xpatch::delta;

fn main() {
    let v1 = b"Hello";
    let v2 = b"Hello, World!";
    let v3 = b"Hello"; // Same as v1!

    println!("=== Naive Approach ===");
    // Always compare with immediate predecessor
    let delta_v1_to_v2 = delta::encode(0, v1, v2, false);
    println!("v1 -> v2 delta size: {} bytes", delta_v1_to_v2.len());

    let delta_v2_to_v3 = delta::encode(0, v2, v3, false);
    println!("v2 -> v3 delta size: {} bytes", delta_v2_to_v3.len());

    let naive_total = delta_v1_to_v2.len() + delta_v2_to_v3.len();
    println!("Naive total: {} bytes\n", naive_total);

    println!("=== Optimized Approach ===");
    // Compare v3 with v1 instead - they're identical!
    let delta_v1_to_v3 = delta::encode(1, v1, v3, false);
    println!("v1 -> v3 delta size: {} bytes", delta_v1_to_v3.len());
    println!("Tag=1 indicates base version\n");

    // Verify decoding works
    let reconstructed = delta::decode(v1, &delta_v1_to_v3[..]).unwrap();
    assert_eq!(reconstructed, v3);

    let tag = delta::get_tag(&delta_v1_to_v3[..]).unwrap();
    println!("Tag extracted: {}", tag);
}
