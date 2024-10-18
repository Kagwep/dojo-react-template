use clap::Parser;
use std::process::Command;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use toml::Value;
mod generate_component;

use generate_component::generate::{generate_typescript_content, read_and_parse_manifest, write_typescript_file};

#[derive(Parser)]
struct Args {
    /// The name of the new project
    project_name: String,
    /// The Dojo profile to use (e.g., 'env' or 'sepolia')
    profile: String,
}

fn main() {
    let args = Args::parse();
    let project_name = args.project_name;
    let profile = args.profile;

    // Step 1: Create React app
    create_react_app(&project_name);

    // Step 2: Generate Dojo TypeScript file
    generate_dojo_typescript(&project_name, &profile);

    println!("Project setup complete!");
}

fn create_react_app(project_name: &str) {
    println!("Creating a new Vite React TypeScript app: {}", project_name);
    Command::new("npm")
        .arg("create")
        .arg("vite@latest")
        .arg(project_name)
        .arg("--")
        .arg("--template")
        .arg("react-ts")
        .status()
        .expect("Failed to create Vite app");

    // Navigate into the project directory
    std::env::set_current_dir(project_name).expect("Failed to change directory");

    // Install dependencies
    println!("Installing dependencies...");
    Command::new("npm")
        .arg("install")
        .status()
        .expect("Failed to install dependencies");

    // Add custom dependencies
    println!("Adding additional dependencies...");
    Command::new("npm")
        .arg("install")
        .arg("react-router-dom")
        .arg("axios")
        .status()
        .expect("Failed to install additional dependencies");

    // Create custom files and folders
    create_custom_files();
    create_src_structure();
}


fn generate_dojo_typescript(project_name: &str, profile: &str) {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let root_dir = current_dir.parent().unwrap();
    let scarb_toml_path = root_dir.join("Scarb.toml");

    // Read and parse Scarb.toml
    let scarb_toml_content = fs::read_to_string(&scarb_toml_path)
        .expect("Failed to read Scarb.toml");
    let scarb_toml: Value = toml::from_str(&scarb_toml_content)
        .expect("Failed to parse Scarb.toml");

    // Extract values from Scarb.toml based on the profile
    let profile_config = scarb_toml["tool"]["dojo"][profile].as_table()
        .expect(&format!("Failed to find profile '{}' in Scarb.toml", profile));

    let rpc_url = profile_config["rpc_url"].as_str()
        .expect("Failed to find rpc_url in profile config")
        .to_string();
    let world_address = profile_config["world_address"].as_str()
        .expect("Failed to find world_address in profile config")
        .to_string();

    // Look for the manifest file in the manifest/{profile}/ directory
    let manifest_path = root_dir.join("manifest").join(profile).join("manifest.json");
    if !manifest_path.exists() {
        eprintln!("manifest.json not found at path: {:?}. Please ensure it exists.", manifest_path);
        std::process::exit(1);
    }

    let manifest = read_and_parse_manifest(&manifest_path)
        .expect("Failed to read or parse manifest");

    let dojo_dir = Path::new("src/dojo/generated");
    fs::create_dir_all(dojo_dir).expect("Failed to create src/dojo/generated folder");

    let ts_file_path = dojo_dir.join("dojoConfig.ts");

    match generate_typescript_content(&manifest, &rpc_url, &world_address) {
        Ok(content) => {
            if let Err(e) = write_typescript_file(&ts_file_path, &content) {
                eprintln!("Error writing file: {}", e);
            } else {
                println!("Generated Dojo TypeScript file: {:?}", ts_file_path);
            }
        },
        Err(e) => eprintln!("Error generating TypeScript content: {}", e),
    }
}

fn create_custom_files() {
    let mut file = File::create("README.md").expect("Failed to create README.md");
    file.write_all(b"# My Vite React TypeScript App with Dojo\n\nThis project was scaffolded using the Dojo React App Generator.")
        .expect("Failed to write to README.md");
    println!("Custom files generated.");
}

fn create_src_structure() {
    // Create a 'components' folder in 'src'
    let components_dir = Path::new("src/components");
    fs::create_dir_all(components_dir).expect("Failed to create src/components folder");

    // Create a 'utils' folder in 'src'
    let utils_dir = Path::new("src/utils");
    fs::create_dir_all(utils_dir).expect("Failed to create src/utils folder");

    // Create a React component file (e.g., src/components/HelloWorld.tsx)
    let component_file = components_dir.join("HelloWorld.tsx");
    let mut file = File::create(component_file).expect("Failed to create HelloWorld.tsx");
    file.write_all(b"import React from 'react';\n\nconst HelloWorld: React.FC = () => (\n  <h1>Hello, World!</h1>\n);\n\nexport default HelloWorld;")
        .expect("Failed to write to HelloWorld.tsx");

    // Create a utility file (e.g., src/utils/helpers.ts)
    let util_file = utils_dir.join("helpers.ts");
    let mut file = File::create(util_file).expect("Failed to create helpers.ts");
    file.write_all(b"// Helper functions\n\nexport const greet = (name: string): string => `Hello, ${name}!`;\n")
        .expect("Failed to write to helpers.ts");

    println!("Custom src structure created.");
}