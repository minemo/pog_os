use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn get_or_create_disk(name: &str) -> PathBuf {
    if let Some(p) = fs::read_dir("./")
        .unwrap()
        .find(|x| x.as_ref().unwrap().file_name() == name)
    {
        println!("Using image ./{}", name);
        p.unwrap().path()
    } else {
        println!("Image {} not found. Creating a new one", name);
        // create file with test-pattern
        let mut f = File::create(name).unwrap();
        for i in 0..1024 * 1024 {
            f.write_all(&[(i % 2 == 0) as u8]).unwrap();
        }

        // create empty disk image using qemu-img
        // let mut create_cmd = std::process::Command::new("qemu-img");
        // create_cmd.args(["create", "-f", "raw", name, "1G"]);
        // let mut child = create_cmd.spawn().unwrap();
        // child.wait().unwrap();
        PathBuf::from(format!("./{name}"))
    }
}

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    let uefi_size: f64 = fs::metadata(uefi_path).unwrap().len() as f64;
    let bios_size: f64 = fs::metadata(bios_path).unwrap().len() as f64;

    println!(
        "UEFI-Kernel at: {} ({:.2} mb)\nBIOS-Kernel at: {} ({:.2} mb)",
        uefi_path,
        uefi_size / 1000000.0,
        bios_path,
        bios_size / 1000000.0
    );

    match std::env::var_os("CI") {
        Some(_) => {
            println!("In CI environment, not running qemu");
        }
        None => {
            println!("Not in CI environment, running qemu");
            let mut cmd = std::process::Command::new("qemu-system-x86_64");

            // use UEFI unless specified otherwise
            match std::env::var_os("POG_USE_BIOS") {
                Some(_) => {
                    println!("using BIOS instead of UEFI");
                    cmd.arg("-drive")
                        .arg(format!("format=raw,file={bios_path}"));
                }
                None => {
                    cmd.arg("-bios").arg("./ovmf/OVMF-pure-efi.fd");
                    cmd.arg("-drive")
                        .arg(format!("format=raw,file={uefi_path}"));
                }
            }

            // add a disk for the os to use
            cmd.arg("-drive").arg(format!(
                "format=raw,if=ide,bus=0,index=1,file={}",
                get_or_create_disk("drive.img").to_str().unwrap()
            ));

            // set device specs
            cmd.arg("-cpu").arg("max");
            cmd.arg("-smp").arg("4");
            cmd.arg("-m").arg("4G");

            // add ISA debug OS exit
            cmd.arg("-device")
                .arg("isa-debug-exit,iobase=0xf4,iosize=0x04");

            // add serial support
            cmd.arg("-serial").arg("stdio");

            // cmd.arg("-s").arg("-S");

            // cmd.arg("-d").arg("int");

            // run qemu
            let mut child = cmd.spawn().unwrap();
            child.wait().unwrap();
        }
    }
}
