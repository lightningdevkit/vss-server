#[cfg(genproto)]
extern crate prost_build;
#[cfg(genproto)]
use std::{env, fs, fs::File, path::Path};

/// To generate updated proto objects, run `RUSTFLAGS="--cfg genproto" cargo build`
fn main() {
	#[cfg(genproto)]
	generate_protos();
}

#[cfg(genproto)]
fn generate_protos() {
	download_file(
		"https://raw.githubusercontent.com/lightningdevkit/vss-server/7f492fcac0c561b212f49ca40f7d16075822440f/app/src/main/proto/vss.proto",
		"src/proto/vss.proto",
	).unwrap();

	prost_build::Config::new()
		.bytes(&["."])
		.compile_protos(&["src/proto/vss.proto"], &["src/"])
		.expect("protobuf compilation failed");
	println!("OUT_DIR: {}", &env::var("OUT_DIR").unwrap());
	let from_path = Path::new(&env::var("OUT_DIR").unwrap()).join("vss.rs");
	fs::copy(from_path, "src/types.rs").unwrap();
}

#[cfg(genproto)]
fn download_file(url: &str, save_to: &str) -> Result<(), Box<dyn std::error::Error>> {
	let mut response = reqwest::blocking::get(url)?;
	fs::create_dir_all(Path::new(save_to).parent().unwrap())?;
	let mut out_file = File::create(save_to)?;
	response.copy_to(&mut out_file)?;
	Ok(())
}
