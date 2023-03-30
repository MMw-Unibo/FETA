use std::{io, fs};
use std::net::{TcpStream};
use std::io::{Write, Read, Cursor};
use std::str::from_utf8;
use bstr::B;
use identity_iota::account::{Account, AccountBuilder};
use identity_iota::core::Timestamp;
use identity_iota::iota_core::IotaDID;
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use std::fs::{File, OpenOptions};
use std::time::Instant;
use std::env;
mod lib;

#[tokio::main]
async fn main() {

    
    let stdin = io::stdin();
    let mut user: Option<Account> = None;
    let mut issuer_did: Option<IotaDID> = None;

    let mut latency: Vec<u128> = Vec::new();


    //println!("Insert Stronghold password:");
    //println!("If the stronghold does not exists a new one will be created with the password of your choice");
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

    //println!("\nWhat do you want to do? (Insert the right number)\n1) Create my identity on the IOTA tangle\n2) I already have an identity\n");

    let now = Instant::now();
    user = Some(match lib::create_identity(&mut builder).await {
        Ok(identity) => {
            println!("Identity created! DID: {}", identity.did());
            latency.push(now.elapsed().as_nanos());

            identity
        },
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    });
    match lib::write_did(user.as_ref().unwrap().did()) {
        Ok(..) => println!("Did saved in did.txt"),
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    }
    
    let did: String = match lib::read_did() {
        Ok(did) => did,
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };
    let user_did: IotaDID = match IotaDID::parse(did.as_str()) {
        Ok(did) => did,
        Err(err) => {
            eprintln!("Error: {:?}", err);
            return
        },
    };

            

    match TcpStream::connect("192.168.10.205:3333") {
        Ok(mut stream) => {
            println!("\nSuccessfully connected to server in port 3333");
            let mut data = [0 as u8; 564]; //564 byte buffer
            
            let now = Instant::now();

            stream.write(b"vc").unwrap();

            match stream.read(&mut data) {
                Ok(..) => println!(),
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };

            stream.write(B(did.as_str())).unwrap();

            let vc: &str = match stream.read(&mut data) {
                Ok(size) => {
                    from_utf8(&data[0..size]).unwrap()
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };

            match lib::write_vc(vc) {
                Ok(..) => println!("VC created and saved in vc.txt"),
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            }
 
            latency.push(now.elapsed().as_nanos());
        
            let now = Instant::now();
            stream.write(b"vp").unwrap();

            let challenge: String = match stream.read(&mut data) {
                Ok(size) => {
                    from_utf8(&data[0..size]).unwrap().to_string()
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };

            stream.write(b"ack").unwrap();

            let timestr: &str = match stream.read(&mut data) {
                Ok(size) => from_utf8(&data[0..size]).unwrap(),
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };
            let timestamp: Timestamp = match Timestamp::parse(timestr) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };

            let vc: String = match lib::read_vc() {
                Ok(vc) => vc,
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };

            let vp: String = match lib::create_vp(&vc, user.as_mut().unwrap(), (challenge, timestamp)).await {
                Ok(vp) => {
                    println!("VP created!");
                    vp
                },
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            };
            latency.push(now.elapsed().as_nanos());


            stream.write(B(vp.as_str())).unwrap();

            match stream.read(&mut data) {
                Ok(size) => {
                    let did = from_utf8(&data[0..size]).unwrap();
                    issuer_did = Some(match IotaDID::parse(did) {
                        Ok(did) => did,
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            return
                        },
                    });
                },
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    return
                },
            }

            
            let client = IpfsClient::from_str("http://192.168.10.205:52004").unwrap();
  
                        
            let mut round = 0;

            
            let mut port = env::var("PORT").unwrap();

            let mut addr = String::from("tcp://*:");
            addr.push_str(&port);

            let ctx = zmq::Context::new();
            let socket = ctx.socket(zmq::REP).unwrap();
            assert!(socket.bind(&addr).is_ok());

            let mut clients_number_str = env::var("CLIENTS").unwrap();
            clients_number_str = clients_number_str.trim_end().to_owned();
            let mut clients_number = clients_number_str.parse::<usize>().unwrap();

            
            _ = socket.recv_string(0).unwrap();
            _ = socket.send("go",0).unwrap();
            let now = Instant::now();

            while round < 10 {
                println!("Round {} begins", round.to_string());
                _ = socket.recv_string(0).unwrap();

                match lib::create_ipfs_content(user.as_ref().unwrap()).await {
                    Ok(_) => {

                        let file = fs::read_to_string("/mnt/ipfs_content.txt").unwrap();
                        let data = Cursor::new(file);


                        let cid = match client.add(data).await {
                            Ok(res) => res.hash,
                            Err(e) => {
                                eprintln!("Error: {:?}", e);
                                return
                            },
                        };

                        println!("Model uploaded to IPFS! CID: {}", cid);

                        let vc: String = match lib::read_vc() {
                            Ok(vc) => vc,
                            Err(err) => {
                                eprintln!("Error: {:?}", err);
                                return
                            },
                        };
                        match lib::upload_to_tangle(user.as_mut().unwrap(), cid, vc, &round.to_string()).await {
                            Ok(_) => {
                                println!("Content uploaded to tangle!");
                                let models = match lib::get_models(&client, &round.to_string(), issuer_did.as_ref().unwrap(), &clients_number).await {
                                    Ok(models) => models,
                                    Err(err) => {
                                        eprintln!("Error: {:?}", err);
                                        return
                                    },
                                };
                                let serialized = serde_json::to_string(&models).unwrap();
                                let mut output = File::create("/mnt/models.json").unwrap();
                                write!(output, "{}", &serialized).unwrap();
                                println!("Retrieved and verified all models.");
                            },
                            Err(err) => {
                                eprintln!("Error: {:?}", err);
                                return
                            },
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {:?}", err);
                        return
                    },
                };
                round += 1;
                if round == 10 {
                    _ = socket.send("stop",0).unwrap();
                    latency.push(now.elapsed().as_nanos());

                } else {
                    _ = socket.send("0",0).unwrap();
                }
                
                
            };
            
            let mut f = OpenOptions::new().append(true).create(true).open(format!("/mnt/latency_{}.txt", clients_number_str)).expect("Unable to open file"); 
            for l in latency {
                write!(f, "{}", format!("{}\n", l));
            }
                             
            stream.write(b"shutdown").unwrap();
            println!("\nClient terminated.");
            return
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
}