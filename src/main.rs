use std::fs;

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    let uefi_size: f64 = fs::metadata(uefi_path).unwrap().len() as f64;
    let bios_size: f64 = fs::metadata(bios_path).unwrap().len() as f64;


    println!("UEFI-Kernel at: {} ({:.2} mb)\nBIOS-Kernel at: {} ({:.2} mb)", uefi_path, uefi_size/1000000.0, bios_path, bios_size/1000000.0);
    
    match std::env::var_os("CI") {
        Some(_) => {
            println!("In CI environment, not running qemu");
        },
        _ => {
            println!("Not in CI environment, running qemu");
            let mut cmd = std::process::Command::new("qemu-system-x86_64");
            cmd.arg("-bios").arg("./ovmf/OVMF-pure-efi.fd");
            cmd.arg("-drive").arg(format!("format=raw,file={uefi_path}"));
            // cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));

            // set device specs
            cmd.arg("-m").arg("4G");            
            cmd.arg("-mem-prealloc");            
            
            // add ISA debug OS exit
            cmd.arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04");

            // add serial support
            cmd.arg("-serial").arg("stdio");
            
            // cmd.arg("-d").arg("int").arg("-D").arg("debug.txt");

            // run qemu
            let mut child = cmd.spawn().unwrap();
            child.wait().unwrap();
        }
    }
    
}