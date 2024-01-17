
/* TODO Script abstract */

/// jlic --help
fn main() -> Fallible<()> {    // Begin procedure

    // Define program options
    let opts = clap::command!()
        .arg( clap::arg!(-j --prefix ... "Enable 'J' prefix in generated filename") )
        .arg( clap::arg!(-d --debug ... "Enable verbose logging") )
        .arg( clap::arg!(-c --update ... "Update license information in Cargo.toml") )
        .get_matches();
    
    // Retrieve & process args
    match opts.get_one::<u8>("debug") { 
        None    => { init_logs(false);  log::debug!("Verbose logging disabled") },
        Some(0) => { init_logs(false);  log::debug!("Verbose logging disabled") },
        Some(_) => { init_logs(true);   log::debug!("Verbose logging enabled") },
    }
    let j: bool = match opts.get_one::<u8>("prefix") {   
        None    => { false },
        Some(0) => { false },
        Some(1) => { true },
        Some(_) => { true },
    };
    log::debug!("'J' prefix is {}", match j { true => "enabled", false => "disabled" });
    let c: bool = match opts.get_one::<u8>("update") {   
        None    => { false },
        Some(0) => { false },
        Some(1) => { true },
        Some(_) => { true },
    };
    log::debug!("Cargo.toml will {}be overwritten", match c { true => "", false => "not " });



    log::debug!("Beginning execution");
    
    // Retrieve target crate metadata
    let package = extract_package_info()?;
    log::debug!("Package metadata found");

    // Extract metadata values
    use chrono::prelude::*;
    let year = Utc::now().year().to_string();   log::info!("Publish: {}", year);
    let name = &package.name;                   log::info!("Package: {}", name);
    let version = &package.version;             log::info!("Version: {}", version);
    let authors = &package.authors;             log::info!("Authors: {}", authors);
    
    // Generate the file name and path
    let crate_root = get_crate_root()?;
    let filename = match j { true => "JLICENSE.md", false => "LICENSE.md" };
    let filepath = format!("{}/{}", crate_root, &filename);
    
    // Overwrites the file at generated path
    let mut file = fresh_file(&filepath)?;      log::debug!("File '{}' initialized", &filename);
    
    // Populate the template with extracted metadata values
    let license = LICENSE.clone()
        .replace("{{name}}", &name)
        .replace("{{version}}", &version)
        .replace("{{year}}", &year)
        .replace("{{authors}}", &authors);      log::debug!("Template created");

    // Write template to file 
    use std::io::Write;
    writeln!(file, "{}", license)?;             log::debug!("Template written");
    
    // Change license in Cargo.toml
    match c {
        true => {
            if let Err(_) = update_license_info(filename) {
                log::warn!("License information within Cargo.toml has not been updated")
            } else {
                log::debug!("Package license information updated successfully")
            }
        },
        false => {
            log::warn!("License information within Cargo.toml has not been updated")
        }
    }


    let git_add = std::process::Command::new("git")
        .args(&["add", &filepath])
        .status();
    match git_add {
        Ok(_)  => log::info!("{} staged for git commit", &filename),
        Err(_) => log::warn!("File not tracked by git")
    }
        
    Ok(println!("\x1b[32m{} {}", filename, SUCCESS))
    
} // End of procedure


/// Result alias for readability
type Fallible<T> = Result<T, anyhow::Error>;

/// License template
const LICENSE: &str = include_str!("template.md");

/// Message to be generated upon success
const SUCCESS: &str =
"was generated successfully. 
Please consult with legal professionals to ensure that this document aligns 
with your specific requirements and complies with relevant laws.";

/// Initialize logging
pub fn init_logs(debug: bool) {
    let debug: &str = match debug { true => "debug", false => "warn" };
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(debug)
    ).init();
    log::debug!("Initialized logger");
}
    
/// Reinitializes file at the given path
fn fresh_file(filepath: &str) -> Fallible<std::fs::File> { 
    if let Ok(path_exists) = std::fs::metadata(&filepath) {
        if path_exists.is_file() {
            std::process::Command::new("rm")
                .args(&[&filepath])
                .status()?;
            log::warn!("Old file at {} was removed!", &filepath);
            return fresh_file(filepath)
        } else { return Err(anyhow::anyhow!(
            "Unable to write! {} already exists, but it is not a file.", filepath
        ))}
    } else { return Ok(std::fs::File::create(&filepath)?) }
}

/// Opens Cargo.toml for editing
fn get_cargo_manifest() -> Fallible<std::fs::File> {
    let mut current_dir = std::env::current_dir()?;
    loop {
        let cargo_path = current_dir.join("Cargo.toml");
        if cargo_path.exists() { return Ok(std::fs::File::open(cargo_path)?) }
        if !current_dir.pop() { return Err(anyhow::anyhow!("Cargo.toml not found in path")) }
    }
}

/// Finds the directory containing Cargo.toml
fn get_crate_root() -> Fallible<String> {
    let mut current_dir = std::env::current_dir()?;
    loop {
        let cargo_path = current_dir.join("Cargo.toml");
        if cargo_path.exists() { return Ok(current_dir.to_string_lossy().to_string()) }
        if !current_dir.pop() { return Err(anyhow::anyhow!("Cargo.toml not found in path")) }
    }
}

/// Reads the contents of file and attempts to parse to TOML
fn read_to_toml(mut file: std::fs::File) -> Fallible<toml::Value> {
    let mut contents = String::new();
    use std::io::Read;
    file.read_to_string(&mut contents)?;
    drop(file);
    if let Ok(value) = toml::from_str(&contents) { Ok(value) }
    else { Err(anyhow::anyhow!("Failed to parse TOML value from string")) }
}

/// Replace Cargo.toml values for "license" and "license-file" keys
fn update_license_info(filename: &str) -> Fallible<()> {
    use std::io::Write;
    let cargo = get_cargo_manifest()?;
    let mut toml = read_to_toml(cargo)?;

    if let Some(package) = toml.get_mut("package") {
        log::debug!("found pkg");
        if let Some(package) = package.as_table_mut() {
            log::debug!("got as mut");
            package.remove("license");            
            if let Some(license_file) = package.get_mut("license-file") {
                *license_file = toml::Value::String(filename.to_owned());
            } else {
                package.insert("license-file".to_string(), toml::Value::String(filename.to_owned()));
            }

            let cargo_path = format!("{}/{}", get_crate_root()?, "Cargo.toml");
            let mut cargo = fresh_file(&cargo_path)?;
            log::debug!("reopen manifest");
            writeln!(cargo, "{}", toml::ser::to_string_pretty(&toml)?)?;
            log::debug!("Written");

            return Ok(())
        }
    }
    
    Err(anyhow::anyhow!("Could not update Cargo.toml"))
}

/// Relevant package information
struct PackageInfo {
    name: String,
    version: String,
    authors: String,
    // TODO Other data?
}

/// Extracts relevant information from Cargo.toml
fn extract_package_info() -> Fallible<PackageInfo> {
    let cargo = get_cargo_manifest()?;
    let toml = read_to_toml(cargo)?;
    let package = toml.get("package").expect("Cargo.toml contains package metadata");
    Ok(PackageInfo {
        name:    match package.get("name")    { Some(val) => dequote_str(val.to_string()), None => "Unknown".to_string() }, 
        version: match package.get("version") { Some(val) => dequote_str(val.to_string()), None => "Unknown".to_string() }, 
        authors: format_authors(package.get("authors").cloned()),
    })
}

/// Trims double quotes from string values extracted from TOML 
fn dequote_str(str: String) -> String {
    str.trim_matches('"').to_owned()
}

/// Formats the array of crate authors into a single string
fn format_authors(author_vec: Option<toml::Value>) -> String {
    let mut authors = String::new();
    if let Some(val) = author_vec {
        if let Some(vec) = val.as_array() {
            if vec.len() == 1 { authors = format!("{}", vec[0]) }
            else {
                for (a, auth) in vec.iter().enumerate() {
                    if a < vec.len() - 1 {
                        authors = format!("{} {},", authors, dequote_str(auth.to_string()));
                    } else { 
                        authors = format!("{} and {}", authors, dequote_str(auth.to_string()));
                    }
                }
            }
            return authors
        }
    }
    return "Unknown".to_string();
}
