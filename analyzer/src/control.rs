use std::{
    error::Error, io::{ErrorKind, Read, Write}, net::TcpStream
};

const EV3_IP: &str = "172.20.10.3";
const PORT: u16 = 5000;

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
        println!("No EV3 tcp stream :/");
    }
}

pub fn connect() -> Result<TcpStream, std::io::Error> {
    return Err(std::io::Error::new(
            ErrorKind::Other,
            "Testing connection failure",
        ));

    println!("Connecting to EV3...");
    let stream = TcpStream::connect((EV3_IP, PORT))?;
    println!("Connected!");

    return Ok(stream);
}
