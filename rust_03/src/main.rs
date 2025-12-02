use clap::{Parser, Subcommand, CommandFactory};
use rand::Rng;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::sync::{Arc, Mutex};


const P: u64 = 0xD87F_AE3E_291B_4C7F;

const G: u64 = 2;

const LCG_A: u64 = 1103515245;
const LCG_C: u64 = 12345;
const LCG_M: u64 = 1 << 32;
const BUFFER_SIZE: usize = 1024;

fn mod_pow(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    if modulus == 0 {
        return 0;
    }
    let modulus_128: u128 = modulus as u128;
    let mut result: u128 = 1;
    let mut base_128: u128 = (base as u128) % modulus_128;

    while exponent > 0 {
        if (exponent & 1) == 1 {
            result = (result * base_128) % modulus_128;
        }
        base_128 = (base_128 * base_128) % modulus_128;
        exponent >>= 1;
    }

    result as u64
}

fn lcg_keystream(seed: u64) -> impl Iterator<Item = u8> {
    let mut current_state = seed;
    
    std::iter::from_fn(move || {
        current_state = (LCG_A.wrapping_mul(current_state).wrapping_add(LCG_C)) % LCG_M;
        Some((current_state & 0xFF) as u8)
    })
}

fn xor_cipher(data: &[u8], keystream: &mut impl Iterator<Item = u8>, keystream_pos: usize) -> (Vec<u8>, Vec<u8>) {
    let mut key_bytes = Vec::with_capacity(data.len());
    let cipher_bytes: Vec<u8> = data.iter()
        .map(|&byte| {
            let key_byte = keystream.next().unwrap_or(0);
            key_bytes.push(key_byte);
            byte ^ key_byte
        })
        .collect();
    
    (cipher_bytes, key_bytes)
}

fn is_printable_ascii(byte: u8) -> bool {
    byte >= 0x20 && byte <= 0x7E
}

#[derive(Parser, Debug)]
#[clap(name = "streamchat", version = "1.0", about = "Stream cipher chat with DH key generation")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Server { port: u16 },
    Client { host: String, port: u16 },
}

fn dh_key_exchange(stream: &mut TcpStream, is_server: bool) -> Result<u64, io::Error> {
    println!("[DH] Starting key exchange...");
    println!("[DH] Using hardcoded DH parameters:");
    println!("p = {:X} (64-bit prime - public)", P);
    println!("g = {} (generator - public)", G);

    let mut rng = rand::thread_rng();
    let private_key: u64 = rng.gen();
    println!("[DH] Generating our keypair...");
    println!("private_key = {:X} (random 64-bit)", private_key);

    let public_key = mod_pow(G, private_key, P);
    println!("public_key = {}^private_key mod p", G);
    println!("= {:X}", public_key);

    let mut their_public_bytes = [0u8; 8];
    let public_bytes = public_key.to_be_bytes();

    println!("[DH] Exchanging keys...");
    
    if is_server {
        println!("[NETWORK] Sending public key (8 bytes)...");
        println!("+ Send our public: {:X}", public_key);
        stream.write_all(&public_bytes)?;
        
        println!("[NETWORK] Receive their public (8 bytes) ✓");
        stream.read_exact(&mut their_public_bytes)?;
    } else {
        println!("[NETWORK] Received public key (8 bytes) ✓");
        stream.read_exact(&mut their_public_bytes)?;

        println!("- Receive their public: {:X}", u64::from_be_bytes(their_public_bytes));
        println!("[NETWORK] Sending public key (8 bytes)...");
        println!("+ Send our public: {:X}", public_key);
        stream.write_all(&public_bytes)?;
    }
    
    let their_public = u64::from_be_bytes(their_public_bytes);
    println!("- Receive their public: {:X}", their_public);

    let shared_secret = mod_pow(their_public, private_key, P);
    
    println!("[DH] Computing shared secret...");
    println!("Formula: secret = (their_public)^(our_private) mod p");
    println!("secret = ({:X})^({:X}) mod p", their_public, private_key);
    println!("= {:X}", shared_secret);

    println!("[VERIFY] Both sides computed the same secret ✓");
    
    Ok(shared_secret)
}

fn start_chat_thread(mut stream_clone: TcpStream, keystream: Arc<Mutex<Box<dyn Iterator<Item = u8> + Send>>>, keystream_pos: Arc<Mutex<usize>>, log_prefix: &'static str) {
    let mut buffer = [0u8; BUFFER_SIZE];
    
    loop {
        match stream_clone.read(&mut buffer) {
            Ok(0) => {
                println!("[NETWORK] Peer disconnected.");
                break;
            }
            Ok(bytes_read) => {
                let cipher_bytes = &buffer[..bytes_read];
                let mut keystream_guard = keystream.lock().unwrap();
                let mut pos_guard = keystream_pos.lock().unwrap();
                let mut key_bytes = Vec::with_capacity(bytes_read);
                let plain_bytes: Vec<u8> = cipher_bytes.iter()
                    .map(|&byte| {
                        let key_byte = keystream_guard.next().unwrap_or(0);
                        key_bytes.push(key_byte);
                        byte ^ key_byte
                    })
                    .collect();
                
                println!("\n[DECRYPT]");
                println!("Cipher: {}", cipher_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>());
                println!("Key: {} (keystream position: {})", key_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>(), *pos_guard);
                
                let plain_hex = plain_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                let plain_ascii: String = plain_bytes.iter().map(|&b| if is_printable_ascii(b) { b as char } else { '.' }).collect();
                
                println!("Plain: {} -> \"{}\"", plain_hex, plain_ascii);
                
                *pos_guard += bytes_read;

                println!("[{}] {}", log_prefix, String::from_utf8_lossy(&plain_bytes).trim());
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break;
            }
        }
    }
}

fn handle_chat(stream: TcpStream, shared_secret: u64, is_server: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("[STREAM] Generating keystream from secret...");
    println!("Algorithm: LCG (a={}, c={}, m=2^32)", LCG_A, LCG_C);
    println!("Seed: secret = {:X}", shared_secret);

    let send_keystream = Arc::new(Mutex::new(Box::new(lcg_keystream(shared_secret)) as Box<dyn Iterator<Item = u8> + Send>));
    let recv_keystream = Arc::new(Mutex::new(Box::new(lcg_keystream(shared_secret)) as Box<dyn Iterator<Item = u8> + Send>));
    
    let send_keystream_pos = Arc::new(Mutex::new(0usize));
    let recv_keystream_pos = Arc::new(Mutex::new(0usize));

    let keystream_preview: Vec<u8> = lcg_keystream(shared_secret).take(10).collect();
    print!("Keystream: ");
    for byte in keystream_preview {
        print!("{:02x} ", byte);
    }
    println!("...");
    
    println!("✓ Secure channel established!");
    
    let mut stream_write = stream.try_clone()?;

    let log_prefix = if is_server { "SERVER" } else { "CLIENT" };
    let recv_thread = thread::spawn({
        let stream_clone = stream.try_clone()?;
        let recv_keystream_clone = recv_keystream.clone();
        let recv_pos_clone = recv_keystream_pos.clone();
        move || {
            start_chat_thread(stream_clone, recv_keystream_clone, recv_pos_clone, log_prefix);
        }
    });

    println!("[CHAT] Type message:");
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush()?;
        
        let mut message = String::new();
        io::stdin().read_line(&mut message)?;
        let message = message.trim();
        
        if message.is_empty() { continue; }
        if message == "quit" { break; }
        
        let plain_bytes = message.as_bytes();

        let (ciphertext, key_bytes) = {
            let mut keystream_guard = send_keystream.lock().unwrap();
            let mut pos_guard = send_keystream_pos.lock().unwrap();
            
            let (ciphertext, key_bytes) = xor_cipher(plain_bytes, &mut *keystream_guard, *pos_guard);
            *pos_guard += plain_bytes.len();
            (ciphertext, key_bytes)
        };

        println!("[ENCRYPT]");
        println!("Plain: {} (\"{}\")", plain_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>(), message);
        println!("Key: {} (keystream position: {})", key_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>(), *send_keystream_pos.lock().unwrap() - plain_bytes.len());
        println!("Cipher: {}", ciphertext.iter().map(|b| format!("{:02x}", b)).collect::<String>());

        println!("[NETWORK] Sending encrypted message ({} bytes)...", ciphertext.len());
        match stream_write.write_all(&ciphertext) {
            Ok(_) => println!("[-] Sent {} bytes", ciphertext.len()),
            Err(e) => {
                eprintln!("Failed to send message: {}", e);
                break;
            }
        }
    }

    let _ = stream_write.shutdown(Shutdown::Both);
    let _ = recv_thread.join();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Server { port } => {
            println!("[SERVER] Listening on 0.0.0.0:{}", port);
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;

            let (mut stream, addr) = listener.accept()?;
            println!("[CLIENT] Connected from {}:{}", addr.ip(), addr.port());

            let shared_secret = dh_key_exchange(&mut stream, true)?;
            
            handle_chat(stream, shared_secret, true)?;
        }

        Commands::Client { host, port } => {
            println!("[CLIENT] connecting to {}:{}...", host, port);
            let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
            println!("[CLIENT] Connected!");

            let shared_secret = dh_key_exchange(&mut stream, false)?;

            handle_chat(stream, shared_secret, false)?;
        }
    }

    Ok(())
}