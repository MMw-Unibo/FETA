use std::{io, thread};
use std::io::{Read, Write};
use std::io::prelude::*;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::str::from_utf8;
use identity_iota::account::{Account, AccountBuilder};
use identity_iota::iota_core::IotaDID;
use bstr::B;
use identity_iota::did::DID;
use tokio::runtime::Runtime;

mod lib;

fn handle_client(mut stream: TcpStream, issuer: Account) {
    let rt  = Runtime::new().unwrap();
    let mut data = [0 as u8; 1047]; //1047 byte buffer
    let mut iteration = 0;
    'foo: while match stream.read(&mut data) {
        Ok(size) => {
            let msg = from_utf8(&data[0..size]).unwrap();
            println!("\nReceived instruction: {}", msg);

            match msg {
                "vc" => {
                    stream.write(b"ack").unwrap();
                    match stream.read(&mut data) {
                        Ok(size) => {
                            let did = from_utf8(&data[0..size]).unwrap();

                            let user_did: IotaDID = match IotaDID::parse(did) {
                                Ok(did) => did,
                                Err(err) => {
                                    eprintln!("Error: {:?}", err);
                                    return
                                },
                            };

                            rt.block_on(async {
                                let vc: String = match lib::crea_vc(&issuer, &user_did).await {
                                    Ok(vc) => vc,
                                    Err(err) => {
                                        eprintln!("Error: {:?}", err);
                                        return
                                    }
                                };
                                println!("VC created!");
                                stream.write(B(vc.as_str())).unwrap();
                            });
                        }
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            return
                        }
                    }
                },
                "vp" => {
                    let challenge = lib::create_challenge();
                    println!("Challenge created!");
                    stream.write(B(challenge.0.as_str())).unwrap();

                    match stream.read(&mut data) {
                        Ok(..) => println!(),
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            return
                        },
                    };

                    stream.write(B(challenge.1.to_rfc3339().as_str())).unwrap();

                    let vp: &str = match stream.read(&mut data) {
                        Ok(size) => from_utf8(&data[0..size]).unwrap(),
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            return
                        },
                    };
                    println!("Received VP from client.");

                    rt.block_on(async {
                        match lib::verify_vp(&String::from(vp), &issuer, challenge).await {
                            Ok(..) => {
                                println!("VP verified!");
                                stream.write(B(&issuer.did().as_str())).unwrap();
                            },
                            Err(err) => {
                                eprintln!("Vp not validated: {:?}", err);
                                return
                            },
                        };
                    });
                },
                "shutdown" => {
                    println!("Terminating connection with {}", stream.peer_addr().unwrap());
                    break 'foo
                },
                _ => {
                    println!("\nUnknown input received");
                    iteration += 1;
                    if iteration == 10 {
                        return
                    }
                },
            }
            true
        },
        Err(_) => {
            println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            stream.shutdown(Shutdown::Both).unwrap();
            false
        }
    } {}
    stream.shutdown(Shutdown::Both).unwrap();
}

#[tokio::main]
async fn main() {
    // println!("Insert Stronghold password:");
    // println!("If the stronghold does not exists a new one will be created with the password of your choice");
    // let stdin = io::stdin();
    let password = String::from("");


    let mut builder: AccountBuilder = match lib::create_builder(password, String::from("dev"), String::from("http://192.168.10.203:14265")).await {
        Ok(res) => {
            println!("\nBuilder created!");
            res
        },
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };

    //SERVER IDENTITY GENERATION, UNCOMMENT TO USE
    //---------------------------------------------------------------------------------------------
    
    let issuer: Account = match lib::create_identity(&mut builder).await {
        Ok(identity) => {
            println!("Identity created! DID: {}", identity.did());
            identity
        },
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };
    match lib::write_did(issuer.did()) {
        Ok(..) => println!("Did saved in did.txt"),
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    }
    
    //---------------------------------------------------------------------------------------------

    let did: String = match lib::read_did() {
        Ok(did) => did,
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };
    let issuer_did: IotaDID = match IotaDID::parse(did) {
        Ok(did) => did,
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };

    let listener = TcpListener::bind("0.0.0.0:3333").unwrap();
    println!("\nServer listening on port 3333");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());

                let issuer: Account = match lib::load_identity(&mut builder, issuer_did.clone()).await {
                    Ok(identity) => {
                        println!("Identity loaded! DID: {}", identity.did());
                        identity
                    },
                    Err(err) => {
                        eprintln!("Error: {:?}", err);
                        return
                    },
                };

                thread::spawn(move|| {
                    //connection succeeded
                    handle_client(stream, issuer)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
                //connection failed
            }
        }
    }
    // close the socket server
    drop(listener);
}