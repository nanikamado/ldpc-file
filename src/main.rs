use clap::{Parser, Subcommand};
use labrador_ldpc::LDPCCode;
use std::{
    fs::File,
    io::{stdin, stdout, Read, Write},
};

const LIMIT: usize = 512;
const CODE: LDPCCode = LDPCCode::TM8192;

#[derive(Parser)]
#[clap(author, version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode
    E { path: String },
    /// Decode
    D,
}

fn main() {
    match Cli::parse().command {
        Commands::E { path } => encode(path),
        Commands::D => decode(),
    }
}

fn encode(path: String) {
    let mut file = File::open(path).unwrap();
    let file_size = file.metadata().unwrap().len();
    let mut stdout = stdout();
    let mut data = vec![0; LIMIT];
    let mut code = vec![0u8; CODE.n() / 8];
    for (b, l) in data.iter_mut().rev().zip(file_size.to_le_bytes()) {
        *b = l;
    }
    CODE.copy_encode(&data, &mut code);
    stdout.write_all(&mut code).unwrap();
    loop {
        data.clear();
        let data_len = Read::by_ref(&mut file)
            .take(LIMIT as u64)
            .read_to_end(&mut data)
            .unwrap();
        if data_len < LIMIT {
            data.resize(LIMIT, 0);
        }
        if data_len == 0 {
            break;
        }
        CODE.copy_encode(&data, &mut code);
        stdout.write_all(&mut code).unwrap();
    }
}

fn decode() {
    let mut stdin = stdin();
    let mut stdout = stdout();
    let mut working = vec![0u8; CODE.decode_bf_working_len()];
    let mut data = vec![0u8; CODE.output_len()];
    let mut code = vec![0u8; CODE.n() / 8];
    stdin.read_exact(&mut code).unwrap();
    CODE.decode_bf(&code, &mut data, &mut working, 200);
    let file_size = usize::from_be_bytes(data[LIMIT - 8..LIMIT].try_into().unwrap());
    let count = num::Integer::div_ceil(&file_size, &LIMIT) - 1;
    for _ in 0..count {
        stdin.read_exact(&mut code).unwrap();
        CODE.decode_bf(&code, &mut data, &mut working, 200);
        stdout.write_all(&mut data[..LIMIT]).unwrap();
    }
    stdin.read_exact(&mut code).unwrap();
    CODE.decode_bf(&code, &mut data, &mut working, 200);
    stdout
        .write_all(&mut data[..file_size - LIMIT * count])
        .unwrap();
}