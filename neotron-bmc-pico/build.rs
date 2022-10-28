/// This is the build-script for the Neotron BMC.
///
/// It just copies the memory.x file somewhere Cargo can find it, then generates a version header.
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
	// Put the linker script somewhere the linker can find it
	let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
	File::create(out.join("memory.x"))
		.unwrap()
		.write_all(include_bytes!("memory.x"))
		.unwrap();
	println!("cargo:rustc-link-search={}", out.display());
	println!("cargo:rerun-if-changed=memory.x");

	// Generate a file containing the firmware version
	let version_output = std::process::Command::new("git")
		.current_dir(env::var_os("CARGO_MANIFEST_DIR").unwrap())
		.args(&["describe", "--tags", "--all", "--dirty"])
		.output()
		.expect("running git-describe");
	assert!(version_output.status.success());

	// Remove the trailing newline
	let mut output = version_output.stdout;
	output.pop();

	// Write the file
	std::fs::write(out.join("version.txt"), output).expect("writing version file");
}
