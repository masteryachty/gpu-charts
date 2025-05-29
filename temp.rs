use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    // Create a file to store our binary data
    let file = File::create("counts.bin")?;
    let mut writer = BufWriter::new(file);

    // Write 1 million floats in ascending order
    for i in 0..1_000_000 {
        // Convert i to f32
        let value = i as f32;
        // Convert f32 to 4 bytes in little-endian order
        let bytes = value.to_le_bytes();
        // Write bytes into the file
        writer.write_all(&bytes)?;
    }

    // Return Ok if everything went fine
    Ok(())
}