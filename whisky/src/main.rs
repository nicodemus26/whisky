use std::{thread};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::string::String;
use std::collections::HashMap;

fn main () {
    let setup_server: Whisky = {
        let mut init_server: Whisky = Whisky::new("9080");
        init_server.get("/ping", ping_handler);
        init_server
    };
    setup_server.run()
    
}

fn ping_handler(c: Context) {
    println!("I have been routed and now I just need to be handled");
    let mut output = c.output;
    match output.write(b"{\"value\",\"pong\"}\n") {
        Ok(_) => (),
        Err(e) => println!("There was some issue writing a result: {}", e)
    }
}

struct Whisky {
    server: TcpListener,
    port: String,
    handlers: HashMap<String, WhiskyHandler >
}

type WhiskyHandler = fn(Context) -> ();

impl Whisky {
    fn new(port: &str) -> Whisky {
        let listener = match TcpListener::bind("127.0.0.1:".to_string() + port){
            Ok(listener) => listener,
            Err(e) => panic!("There was an issue {}", e)
        };
        Whisky{server: listener, port: String::from(port), handlers: HashMap::new()}
    }

    fn run(&self) {
        println!("listening on port {}", self.port);
        for stream in self.server.incoming() {
            match stream {
                Ok(stream) => {
                    let handlers = self.handlers.clone();
                    thread::spawn(move || {
                        handle_client(stream, handlers);
                    });
                }
                Err(e) => panic!("an error has occured {}", e)
            }
        }
    }

    fn get(&mut self,route: &str, f: WhiskyHandler) {
        println!("install a handler");
        self.handlers.insert(String::from(route), f);
    }
}

#[derive(Debug)]
struct Context {
    headers: std::collections::HashMap<String,String>,
    method: String,
    url: String,
    protocol: String,
    output: TcpStream,
}

impl Context {
    fn new(request_string: String, output_stream: TcpStream) -> Context {
        
        println!("Here is a request\n{}\nEND", request_string);
        let first_line = request_string.lines().nth(0);
        let mut method = String::from("");
        let mut url = String::from("");
        let mut proto = String::from("");
        let mut headers = std::collections::HashMap::new();
        match first_line {
            Some(fl) => {
                let mut parts = fl.split_whitespace();
                method = match parts.nth(0) {
                    Some(m) => String::from(m),
                    None => { println!("Request missing method"); String::from("") }
                };
                url = match parts.nth(0) {
                    Some(u) => String::from(u),
                    None => { println!("Request missing url"); String::from("") }
                };
                proto = match parts.nth(0) {
                    Some(p) => { String::from(p) },
                    None => { println!("Request missing protocol"); String::from("") }
                };
            },
            None => ()
        }
        for l in request_string.lines().skip(1) {
            if l.is_empty() {
                continue
            }
            let colon_index = l.find(':').unwrap_or(l.len()-1);
            let name = String::from(&l[..colon_index]);
            let value = String::from(&l[colon_index+2..]);
            headers.insert(name, value);
        }
        Context {
            headers: headers,
            method: method,
            url: url,
            protocol: proto,
            output: output_stream,
        }
    }
}

fn parse_header(stream: &TcpStream) -> String {
    let mut found_crlf = 0;
    // goes "even" "odd" depending on wether CR was the last found true (odd) or LF was found false (even)
    let mut last_found_cr = false;
    let mut header = Vec::new();
        
    for res in stream.bytes() {
        match res {
            Ok(b) => {
                header.push(b);
                if b == 13 {
                    last_found_cr = true;
                } else if b == 10 {
                    if last_found_cr {
                       found_crlf += 1;
                    }
                    last_found_cr = false;
                } else {
                    // the number of found crlfs need to be the number of consecutive crlf's
                    found_crlf = 0;
                    println!("Have a byte {:?}", b)
                }
                if found_crlf == 2 {
                    println!("End of Header");
                    break
                }
            }
            Err(e) => println!("There was an erro matchin on a byte: {}", e)
        }
    }
    let string_header = match String::from_utf8(header){
        Ok(sr) => sr,
        Err(e) => { println!("The request is not in utf8: {}", e); String::from("")}
    };
    string_header

}

fn handle_client(stream: TcpStream, handlers: HashMap<String, WhiskyHandler>) {
    // println!("handling request");
    let string_request = parse_header(&stream);


    let context = Context::new(string_request, stream);
    println!("Built context: {:?}", context);

    if handlers.contains_key(&context.url) {
        match handlers.get(&context.url) {
            Some(handler) => handler(context),
            None => println!("Something went terribly wrong getting a handler for {}", context.url)
        }
    } else {
        // println!("key not found in:\n {:?}", handlers);
        let mut output = context.output;
        match output.write(b"404 page not found") {
            Ok(_/*bytes_written*/) => (),
            Err(e) => println!("Error while writing result: {}", e)
        };
    }

}
