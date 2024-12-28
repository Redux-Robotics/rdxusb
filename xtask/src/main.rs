use std::path::Path;

use maven_utils::{build_maven_zip, Target};

pub mod maven_utils;

const GROUP_ID: &str = "com.reduxrobotics.usb";
const ARTIFACT_ID: &str = "rdxusb";

fn main() {
    
    let task = std::env::args().nth(1);
    match task.as_deref() {
        Some("linuxathena") => build_maven(Target::LinuxAthena),
        Some("linuxx86-64") => build_maven(Target::LinuxX86_64),
        Some("linuxarm32") => build_maven(Target::LinuxArm32),
        Some("linuxarm64") => build_maven(Target::LinuxArm64),
        Some("windowsx86-64") => build_maven(Target::WindowsX86_64),
        Some("windowsarm64") => build_maven(Target::WindowsArm64),
        Some("osxuniversal") => build_maven(Target::OsxUniversal),
        Some("headers") => {
            build_maven_zip(Path::new("include"), GROUP_ID, ARTIFACT_ID, "headers").unwrap();
        }

        Some(..) | None => {
            eprintln!("specify a valid target: {{linuxathena, linuxx86-64, linuxarm32, linuxarm64, windowx86-64, windowsarm64, osxuniversal}}");
            std::process::exit(-1);
        }
    }
}

fn build_maven(target: Target) {
    maven_utils::build_maven(target, GROUP_ID, ARTIFACT_ID).unwrap();
    maven_utils::build_maven_pom(GROUP_ID, ARTIFACT_ID).unwrap();
    maven_utils::build_maven_metadata(GROUP_ID, ARTIFACT_ID).unwrap();

}