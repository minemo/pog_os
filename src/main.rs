fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    println!("UEFI-Kernel at: {}\nBIOS-Kernel at: {}", uefi_path, bios_path);
    
    match std::env::var_os("CI") {
        Some(_) => {
            println!("In CI environment, not running qemu");
        },
        _ => {
            println!("Not in CI environment, running qemu");
            let mut cmd = std::process::Command::new("qemu-system-x86_64");
            cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));
            
            // add ISA debug OS exit
            cmd.arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04");

            // add serial support
            cmd.arg("-serial").arg("stdio");
            
            cmd.arg("-d").arg("pic");

            // run qemu
            let mut child = cmd.spawn().unwrap();
            child.wait().unwrap();
        }
    }
    
}