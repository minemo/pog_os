use std::env::set_var;

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    println!("UEFI-Kernel at: {}\nBIOS-Kernel at: {}", uefi_path, bios_path);
    
    match std::env::var_os("CI") {
        Some(v) => {
            println!("In CI environment, setting env-vars");
            set_var("BUILD_UEFI_PATH", uefi_path);
            set_var("BUILD_BIOS_PATH", bios_path);
        },
        _ => {
            println!("Not in CI environment, running qemu");
            let mut cmd = std::process::Command::new("qemu-system-x86_64");
            cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));
            
            // add ISA debug OS exit
            cmd.arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04");
            
            // run qemu
            let mut child = cmd.spawn().unwrap();
            child.wait().unwrap();
        }
    }
    
}