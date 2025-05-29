use futures::stream::Count;
use rand::random;
use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    // Create a file to store our binary data
    let name = "100M";
    let x = 100_000_000;
    y_generate(name, x);
    x_generate(name, x);
    // Return Ok if everything went fine
    Ok(())
}

fn y_generate(name: &str, count :i32) -> std::io::Result<()> {
    //combine 3 strings to create a file path
    let file_path = "./data/".to_string() + name + "_y.bin";
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);
    let mut value = 0 as f32;

    // Write 1 million floats in ascending order
    for i in 0..count {
        // Convert i to f32
        // Convert f32 to 4 bytes in little-endian order
        // generate random number between -5 and 5
        let rand = (rand::random::<f32>() * 10.0) - 5.0;
        value = value + rand;
        let bytes = (value).to_le_bytes();
        // let bytes = (i).to_le_bytes();
        // Write bytes into the file
        writer.write_all(&bytes)?;
    }
    Ok(())
}

fn x_generate(name: &str, count :i32) -> std::io::Result<()> {
    let file_path = "./data/".to_string() + name + "_x.bin";

    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);
    let mut value = 0 as f32;

    // Write 1 million floats in ascending order
    for i in 0..count {
        let bytes = (i as f32).to_le_bytes();
        // Write bytes into the file
        writer.write_all(&bytes)?;
    }
    Ok(())
}
