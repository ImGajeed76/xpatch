use xpatch::delta;

fn main() {
    let base_data = b"Hello, world!";
    let new_data = b"Hello, beautiful world!";

    // Encode the difference
    let tag = 0; // User-defined metadata
    let enable_zstd = true;
    let delta = delta::encode(tag, base_data, new_data, enable_zstd);

    println!("Original size: {} bytes", base_data.len());
    println!("Delta size: {} bytes", delta.len());
    println!(
        "Compression ratio: {:.2}%",
        (1.0 - delta.len() as f64 / new_data.len() as f64) * 100.0
    );

    // Decode to reconstruct new_data
    let reconstructed = delta::decode(base_data, &delta[..]).unwrap();
    assert_eq!(reconstructed, new_data);

    // Extract metadata without decoding
    let extracted_tag = delta::get_tag(&delta[..]).unwrap();
    assert_eq!(extracted_tag, tag);
}
