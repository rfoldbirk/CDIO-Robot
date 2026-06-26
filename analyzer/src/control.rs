use std::{
    error::Error, io::{ErrorKind, Read, Write}, net::TcpStream
};

const PI_IP: &str = "172.20.10.11";
const PI_PORT: u16 = 8000;


const EV3_IP: &str = "172.20.10.3";
const EV3_PORT: u16 = 5000;

pub fn send_command(stream: &mut Option<TcpStream>, cmd: &str, last_command: &mut String) {
    if let Some(stream) = stream {
        if *last_command == cmd {
            return;
        }

        let _ = stream.write_all(format!("{cmd}\n").as_bytes());

        let mut buf = [0u8; 1024];

        match stream.read(&mut buf) {
            Ok(n) if n > 0 => {
                println!("EV3: {}", String::from_utf8_lossy(&buf[..n]).trim());
            }
            _ => {}
        }

        *last_command = cmd.to_string();
    }
    else {
        println!("No TCP stream :/ - could not send {cmd}");
    }
}

pub fn connect_ev3() -> Result<TcpStream, std::io::Error> {
    println!("Connecting to EV3...");
    let stream = TcpStream::connect((EV3_IP, EV3_PORT))?;
    println!("Connected!");

    return Ok(stream);
}


pub fn connect_suck() -> Result<TcpStream, std::io::Error> {
    // return Err(std::io::Error::new(
    //     ErrorKind::Other,
    //     "Testing connection failure",
    // ));

    println!("Connecting to Raspberry Pi...");
    let stream = TcpStream::connect((PI_IP, PI_PORT))?;

    println!("Connected! :)");

    return Ok(stream);
}
