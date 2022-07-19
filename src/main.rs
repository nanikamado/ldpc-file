use clap::{Parser, Subcommand};
use dpc_pariter::IteratorExt;
use labrador_ldpc::LDPCCode;
use std::{
    fs::File,
    io::{self, stdout, Read, Write},
    process::exit,
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
    let mut stdin = io::stdin();
    let mut stdout = stdout();
    let mut working = vec![0i8; CODE.decode_ms_working_len()];
    let mut working_u8 = vec![0u8; CODE.decode_ms_working_u8_len()];
    let mut working_bf = vec![0u8; CODE.decode_bf_working_len()];
    let mut data = vec![0u8; CODE.output_len()];
    let mut code = vec![0u8; CODE.n() / 8];
    stdin.read_exact(&mut code).unwrap();
    decode_data(
        &code,
        &mut data,
        &mut working_bf,
        &mut working,
        &mut working_u8,
    );
    let file_size = usize::from_be_bytes(data[LIMIT - 8..LIMIT].try_into().unwrap());
    eprintln!("size of original file: {file_size}");
    let count = num::Integer::div_ceil(&file_size, &LIMIT) - 1;
    (0..count)
        .map(|_| {
            let mut code = vec![0u8; CODE.n() / 8];
            io::stdin().read_exact(&mut code).unwrap();
            code
        })
        .parallel_map(|code| {
            let mut data = vec![0u8; CODE.output_len()];
            let mut working = vec![0i8; CODE.decode_ms_working_len()];
            let mut working_u8 = vec![0u8; CODE.decode_ms_working_u8_len()];
            let mut working_bf = vec![0u8; CODE.decode_bf_working_len()];
            decode_data(
                &code,
                &mut data,
                &mut working_bf,
                &mut working,
                &mut working_u8,
            );
            data
        })
        .for_each(|mut output| stdout.write_all(&mut output[..LIMIT]).unwrap());
    for _ in 0..count {}
    stdin.read_exact(&mut code).unwrap();
    decode_data(
        &code,
        &mut data,
        &mut working_bf,
        &mut working,
        &mut working_u8,
    );
    stdout
        .write_all(&mut data[..file_size - LIMIT * count])
        .unwrap();
}

fn decode_data(
    input: &[u8],
    output: &mut [u8],
    working_bf: &mut [u8],
    working: &mut [i8],
    working_u8: &mut [u8],
) {
    let (success, _) = CODE.decode_bf(input, output, working_bf, 1000);
    if !success {
        let mut llrs = vec![0i8; CODE.n()];
        CODE.hard_to_llrs(input, &mut llrs);
        let (success, _) = CODE.decode_ms(&llrs, output, working, working_u8, 1000);
        if !success {
            eprintln!("decoding failed.");
            exit(1);
        }
    }
}
