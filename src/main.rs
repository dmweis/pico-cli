use std::{io, thread, time::Duration};

use clap::Parser;
use postcard::{to_allocvec_cobs, to_vec, to_vec_cobs};
use serde::{Deserialize, Serialize};
use serialport::{available_ports, SerialPortType};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port name
    #[arg(short, long)]
    port: Option<String>,

    /// list ports
    #[arg(long)]
    list_ports: bool,

    /// reset
    #[arg(long)]
    reset: bool,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct MotorCommand {
    a: i8,
    b: i8,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.list_ports {
        list_ports()?;
        return Ok(());
    }

    let port_name = if let Some(port_name) = args.port {
        port_name
    } else {
        find_port()?
    };

    let mut port = serialport::new(port_name, 115200).open()?;

    thread::spawn({
        let mut port = port.try_clone().unwrap();
        move || loop {
            let mut text = String::new();
            match port.read_to_string(&mut text) {
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }
            if !text.is_empty() {
                println!("{}", text);
            }
        }
    });

    if args.reset {
        println!("Resetting device");
        port.write_all(" reset ".as_bytes())?;
        thread::sleep(Duration::from_secs(1));
        return Ok(());
    }

    for i in 0..10 {
        let command = MotorCommand { a: i, b: 10 - i };
        let bytes = to_allocvec_cobs(&command)?;
        println!("Sending payload");
        port.write_all(&bytes)?;
        thread::sleep(Duration::from_millis(250));
    }

    Ok(())
}

fn list_ports() -> anyhow::Result<()> {
    let ports = available_ports()?;
    for port in ports {
        println!("  {}", port.port_name);
        match port.port_type {
            SerialPortType::UsbPort(info) => {
                println!("    Type: USB");
                println!("    VID:{:04x} PID:{:04x}", info.vid, info.pid);
                println!(
                    "     Serial Number: {}",
                    info.serial_number.as_ref().map_or("", String::as_str)
                );
                println!(
                    "      Manufacturer: {}",
                    info.manufacturer.as_ref().map_or("", String::as_str)
                );
                println!(
                    "           Product: {}",
                    info.product.as_ref().map_or("", String::as_str)
                );
                println!(
                    "         Interface: {}",
                    info.interface
                        .as_ref()
                        .map_or("".to_string(), |x| format!("{:02x}", *x))
                );
            }
            SerialPortType::BluetoothPort => {
                println!("    Type: Bluetooth");
            }
            SerialPortType::PciPort => {
                println!("    Type: PCI");
            }
            SerialPortType::Unknown => {
                println!("    Type: Unknown");
            }
        }
    }
    Ok(())
}

fn find_port() -> anyhow::Result<String> {
    let ports = available_ports()?;
    for port in ports {
        if let SerialPortType::UsbPort(info) = port.port_type {
            let serial_number = info.serial_number.as_ref().map_or("", String::as_str);
            if serial_number.eq_ignore_ascii_case("picoplayground") {
                return Ok(port.port_name);
            }
        }
    }
    anyhow::bail!("Failed to find port")
}
