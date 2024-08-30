use std::{
    io, thread,
    time::{Duration, Instant},
};

use clap::Parser;
use postcard::{to_allocvec, to_allocvec_cobs};
use serde::{Deserialize, Serialize};
use serialport::{available_ports, SerialPort, SerialPortType};

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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
struct MotorCommand {
    a: i8,
    b: i8,
    c: i8,
    d: i8,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct LedCommand {
    status: bool,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
enum Command {
    ResetToUsbBoot,
    MotorCommand(MotorCommand),
    LedCommand(LedCommand),
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
                Err(e) => {
                    eprintln!("{:?}", e);
                    return;
                }
            }
            if !text.is_empty() {
                println!("{}", text);
            }
        }
    });

    if args.reset {
        println!("Resetting device");
        let command = Command::ResetToUsbBoot;
        let bytes = to_allocvec_cobs(&command)?;
        println!("Sending payload {:?}", &to_allocvec(&command)?);
        port.write_all(&bytes)?;

        thread::sleep(Duration::from_secs(1));
        return Ok(());
    }

    // led on
    port.write_all(&to_allocvec_cobs(&Command::LedCommand(LedCommand {
        status: true,
    }))?)?;

    println!("Starting loop");
    wind_up_motors(&mut port, 1)?;
    wind_up_motors(&mut port, -1)?;

    let command = Command::MotorCommand(MotorCommand::default());
    port.write_all(&to_allocvec_cobs(&command)?)?;

    // led off
    port.write_all(&to_allocvec_cobs(&Command::LedCommand(LedCommand {
        status: false,
    }))?)?;

    Ok(())
}

#[allow(unused)]
fn run_motors(port: &mut Box<dyn SerialPort>, drive: i8) -> anyhow::Result<()> {
    let now = Instant::now();
    loop {
        let command = Command::MotorCommand(MotorCommand {
            a: drive,
            b: drive,
            c: drive,
            d: drive,
        });
        port.write_all(&to_allocvec_cobs(&command)?)?;
        println!("Sending command {:?}", to_allocvec(&command)?);
        thread::sleep(Duration::from_millis(50));
        if now.elapsed() > Duration::from_secs(5) {
            break;
        }
    }
    Ok(())
}

fn wind_up_motors(port: &mut Box<dyn SerialPort>, drive: i8) -> anyhow::Result<()> {
    for i in 0..=100 {
        let command = Command::MotorCommand(MotorCommand {
            a: i * drive,
            b: i * drive,
            c: i * drive,
            d: i * drive,
        });
        port.write_all(&to_allocvec_cobs(&command)?)?;
        println!("Sending command {:?}", to_allocvec(&command)?);
        thread::sleep(Duration::from_millis(50));
    }
    for i in (0..=100).rev() {
        let command = Command::MotorCommand(MotorCommand {
            a: i * drive,
            b: i * drive,
            c: i * drive,
            d: i * drive,
        });
        port.write_all(&to_allocvec_cobs(&command)?)?;
        println!("Sending command {:?}", to_allocvec(&command)?);
        thread::sleep(Duration::from_millis(50));
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
