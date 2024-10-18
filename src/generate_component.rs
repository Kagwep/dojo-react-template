

pub mod generate{

    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::collections::HashMap;
    use std::collections::HashSet;
    use serde_json::Value;

    fn parse_model_name(model: &str) -> String {
        // Define a set of known acronyms
        let acronyms: HashSet<_> = vec!["ERC"].into_iter().collect();
    
        model
            .split("::")
            .last()
            .unwrap_or("")
            .split('_')
            .map(|part| {
                // If the part is a known acronym, keep it in uppercase
                if acronyms.contains(&part.to_uppercase().as_str()) {
                    part.to_uppercase()
                }
                // If the part is fully numeric, keep it as is
                else if part.parse::<i32>().is_ok() {
                    part.to_string()
                }
                // Capitalize the first letter and make the rest lowercase
                else {
                    let mut chars = part.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
                    }
                }
            })
            .collect::<String>()
    }
    
    fn create_cairo_to_recs_type_map() -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        map.insert("bool".to_string(), "RecsType.Boolean".to_string());
        map.insert("u8".to_string(), "RecsType.Number".to_string());
        map.insert("u16".to_string(), "RecsType.Number".to_string());
        map.insert("u32".to_string(), "RecsType.Number".to_string());
        map.insert("u64".to_string(), "RecsType.BigInt".to_string());
        map.insert("usize".to_string(), "RecsType.Number".to_string());
        map.insert("u128".to_string(), "RecsType.BigInt".to_string());
        map.insert("u256".to_string(), "RecsType.BigInt".to_string());
        map.insert("felt252".to_string(), "RecsType.BigInt".to_string());
        map.insert("contractaddress".to_string(), "RecsType.BigInt".to_string());
        map.insert("enum".to_string(), "RecsType.String".to_string());
        map.insert("array".to_string(), "RecsType.StringArray".to_string());
        map.insert("bytearray".to_string(), "RecsType.String".to_string());
    
        map
    }
    
    
    pub fn read_and_parse_manifest(manifest_path: &std::path::Path) -> Result<Value, Box<dyn std::error::Error>> {
        // Read the file contents
        let manifest_str = fs::read_to_string(manifest_path)?;
        
        // Parse the JSON
        let manifest: Value = serde_json::from_str(&manifest_str)?;
        
        Ok(manifest)
    }
    
    
    fn parse_model_schema_to_recs(schema: &Value, types: &mut Vec<String>, custom_types: &mut Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        if schema["type"] != "struct" {
            return Err("unsupported root schema type".into());
        }
    
        parse_schema_struct(&schema["content"], types, custom_types)
    }
    
    fn parse_model_schema_to_recs_impl(schema: &Value, types: &mut Vec<String>, custom_types: &mut Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        let type_str = schema["type"].as_str().ok_or("Missing 'type' field")?;
        let content = &schema["content"];
    
        match type_str {
            "primitive" => parse_schema_primitive(content, types),
            "struct" => {
                custom_types.push(content["name"].as_str().unwrap_or_default().to_string());
                parse_schema_struct(content, types, custom_types)
            },
            "enum" => {
                types.push("enum".to_string());
                custom_types.push(content["name"].as_str().unwrap_or_default().to_string());
                parse_schema_enum(content)
            },
            "tuple" => parse_schema_tuple(content, types, custom_types),
            "array" => Ok("RecsType.StringArray".to_string()),
            "bytearray" => Ok("RecsType.String".to_string()),
            _ => Err(format!("Unsupported type: {}", type_str).into()),
        }
    }
    
    fn parse_schema_primitive(content: &Value, types: &mut Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        let scalar_type = content["scalar_type"].as_str().ok_or("Missing 'scalar_type' field")?.to_lowercase();
        types.push(scalar_type.clone());
        
        let cairo_to_recs_type = create_cairo_to_recs_type_map();
        Ok(cairo_to_recs_type.get(&scalar_type).unwrap_or(&"RecsType.String".to_string()).clone())
    }
    
    fn parse_schema_struct(content: &Value, types: &mut Vec<String>, custom_types: &mut Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        let children = content["children"].as_array().ok_or("Missing 'children' field")?;
        let members: Result<Vec<String>, Box<dyn std::error::Error>> = children.iter().map(|member| {
            let name = member["name"].as_str().ok_or("Missing 'name' field")?;
            let member_type = parse_model_schema_to_recs_impl(&member["member_type"], types, custom_types)?;
            Ok(format!("{}: {}", name, member_type))
        }).collect();
    
        Ok(format!("{{ {} }}", members?.join(", ")))
    }
    
    
    fn parse_schema_enum(_content: &Value) -> Result<String, Box<dyn std::error::Error>> {
        Ok("RecsType.Number".to_string())
    }
    
    fn parse_schema_tuple(content: &Value, types: &mut Vec<String>, custom_types: &mut Vec<String>) -> Result<String, Box<dyn std::error::Error>> {
        let tuple_types: Result<Vec<String>, _> = content.as_array()
            .ok_or("Tuple content should be an array")?
            .iter()
            .map(|schema| parse_model_schema_to_recs_impl(schema, types, custom_types))
            .collect();
    
        Ok(format!("[ {} ]", tuple_types?.join(", ")))
    }
    
    pub fn generate_typescript_content(manifest: &Value, rpc_url: &str, world_address: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut file_content = String::from("/* Autogenerated file. Do not edit manually. */\n\n");
        file_content.push_str("import { defineComponent, Type as RecsType, World } from \"@dojoengine/recs\";\n\n");
        file_content.push_str("export type ContractComponents = Awaited<ReturnType<typeof defineContractComponents>>;\n\n");
        file_content.push_str("export function defineContractComponents(world: World) {\n  return {\n");
    
        let models = manifest["models"].as_array().ok_or("No models found in manifest")?;
    
        for model in models {
            let mut types = Vec::new();
            let mut custom_types = Vec::new();
    
            let model_name = parse_model_name(model["name"].as_str().unwrap_or_default());
            
            let output = Command::new("sozo")
                .args(&["model", "schema", &model_name, "--rpc-url", rpc_url, "--json", "--world", world_address])
                .output()?;
    
            if !output.status.success() {
                eprintln!("Error when fetching schema for model '{}' from {}: {:?}", model_name, rpc_url, String::from_utf8_lossy(&output.stderr));
                continue;
            }
    
            let schema: Value = serde_json::from_slice(&output.stdout)?;
            let recs_type_object = parse_model_schema_to_recs(&schema, &mut types, &mut custom_types)?;
    
            file_content.push_str(&format!("    {}: (() => {{\n", model_name));
            file_content.push_str("      return defineComponent(\n");
            file_content.push_str("        world,\n");
            file_content.push_str(&format!("        {},\n", recs_type_object));
            file_content.push_str("        {\n");
            file_content.push_str("          metadata: {\n");
            file_content.push_str(&format!("            name: \"{}\",\n", model_name));
            file_content.push_str(&format!("            types: {},\n", serde_json::to_string(&types)?));
            file_content.push_str(&format!("            customTypes: {},\n", serde_json::to_string(&custom_types)?));
            file_content.push_str("          },\n");
            file_content.push_str("        }\n");
            file_content.push_str("      );\n");
            file_content.push_str("    })(),\n");
        }
    
        file_content.push_str("  };\n}\n");
    
        Ok(file_content)
    }    

    pub fn write_typescript_file(js_file_path: &Path, file_content: &str) -> Result<(), std::io::Error> {
        fs::write(js_file_path, file_content)?;
        println!("Components file generated successfully: {:?}", js_file_path);
        Ok(())
    }
}