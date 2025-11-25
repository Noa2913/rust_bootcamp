use clap::Parser;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use clap::CommandFactory;

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err(format!("Chaîne hexadécimale impaire: {}", hex));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Hex invalide à la position {}: {}", i, e))
        })
        .collect()
}

fn display_hex_dump_line(offset: u64, buffer: &[u8]) {
    print!("{:08X}: ", offset);

    for (i, byte) in buffer.iter().enumerate() {
        print!("{:02X}", byte);
        if i % 2 == 1 {
            print!(" ");
        }
    }
    for _ in buffer.len()..16 {
        print!("  ");
    }
    print!("| ");

    for byte in buffer.iter() {
        if *byte >= 0x20 && *byte <= 0x7E {
            print!("{}", *byte as char);
        } else {
            print!(".");
        }
    }
    println!("|");
}

#[derive(Parser, Debug)]

#[clap(
    author,
    version,
    about = "Lecture et écriture de fichiers binaires en hexadécimal."
)]
struct Cli {
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    filename: String,

    #[arg(short = 'r', long = "read", action)]
    read_mode: bool,
    
    #[arg(short = 'w', long = "write", value_name = "HEX_STRING")]
    write_hex: Option<String>,

    #[arg(short = 'o', long = "offset", default_value = "0", value_parser = parse_offset)]
    offset: u64,

    #[arg(short = 's', long = "size")]
    size: Option<usize>,
}

fn parse_offset(src: &str) -> Result<u64, String> {
    if src.starts_with("0x") || src.starts_with("0X") {
        u64::from_str_radix(&src[2..], 16).map_err(|e| format!("Offset hex invalide: {}", e))
    } else {
        src.parse::<u64>().map_err(|e| format!("Offset décimal invalide: {}", e))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    if !args.read_mode && args.write_hex.is_none() {
        Cli::command().print_help()?;
        return Ok(());
    }

    if let Some(hex_data) = args.write_hex {
        let bytes_to_write = match hex_to_bytes(&hex_data) {
            Ok(b) => b,
            Err(e) => return Err(format!("Erreur hexadécimale: {}", e).into()),
        };
        let write_size = bytes_to_write.len();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&args.filename)?;

        file.seek(SeekFrom::Start(args.offset))?;

        file.write_all(&bytes_to_write)?;
        
        println!("writing {} bytes at offset 0x{:08X}", write_size, args.offset);
        println!("Hex: {}", hex_data);
        println!("ASCII: {}", String::from_utf8_lossy(&bytes_to_write).trim());
        println!("✓ Successfully written");

    } else if args.read_mode {
        let mut file = File::open(&args.filename)?;

        file.seek(SeekFrom::Start(args.offset))?;

        let read_size = args.size.unwrap_or(32);
        let mut buffer = vec![0u8; read_size];
        
        let bytes_read = file.read(&mut buffer)?;
        
        let mut current_offset = args.offset;
        for chunk in buffer[..bytes_read].chunks(16) {
            display_hex_dump_line(current_offset, chunk);
            current_offset += chunk.len() as u64;
        }
    }

    Ok(())
}
