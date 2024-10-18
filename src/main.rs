use std::env;
use std::process;
use std::path::{Path, PathBuf};
use std::process::Command;
mod generate_component;

use generate_component::generate::{generate_typescript_content,read_and_parse_manifest,write_typescript_file};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 5 {  // 5 because in Rust, args[0] is the program name
        eprintln!("Usage: <MANIFEST_PATH> <OUTPUT_PATH> <RPC_URL> <WORLD_ADDRESS>");
        process::exit(1);
    }

        // Extract paths from command-line arguments
        let manifest_path: PathBuf = Path::new(&args[1]).canonicalize().expect("Failed to resolve manifest path");
        let js_file_path: PathBuf = Path::new(&args[2]).canonicalize().expect("Failed to resolve output path");
        let rpc_url: String = args[3].clone();
        let world_address: String = args[4].clone();
    
        // Check if 'sozo' command exists
        let sozo_exists = Command::new("sh")
            .arg("-c")
            .arg("command -v sozo")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !sozo_exists {
            eprintln!("unable to find `sozo` command. Please install using `dojoup`.");
            std::process::exit(0);
        }

        // Rest of your program...
        println!("Sozo command found. Continuing with the program...");


        let manifest = read_and_parse_manifest(&manifest_path)
        .expect("Failed to read or parse manifest");
        

        match generate_typescript_content(&manifest, &rpc_url, &world_address) {
            Ok(content) => {
                if let Err(e) = write_typescript_file(Path::new(&js_file_path), &content) {
                    eprintln!("Error writing file: {}", e);
                }
            },
            Err(e) => eprintln!("Error generating TypeScript content: {}", e),
        }




}