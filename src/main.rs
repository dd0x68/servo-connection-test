extern crate hyper;
extern crate html5ever;
extern crate scoped_threadpool;
extern crate clap;
extern crate env_logger;
#[macro_use] extern crate string_cache;

use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::BufReader;
use std::path::Path;

use hyper::{Client, client};
use hyper::header::{Headers, Connection};

use std::string::String;

use scoped_threadpool::Pool;
use std::sync::Arc;
use clap::{Arg,App};

//TODO is there a way to do this with rust-url?
fn get_filename_from_url(url : &str) -> String {
    url.replace("/", "_")
}


fn fetch_resource(url: &str, client: &Client, io_flag: bool, conn_flag: bool){
    let mut headers = Headers::new();
    if conn_flag {
        headers.set(Connection::close());
    } else {
        headers.set(Connection::keep_alive());
    }
    
    match  client.get(url)
        .headers(headers)
        .send()
     {
            Ok(mut response) => if io_flag{
                write_resource(url, &mut response)},
            Err(why) => write_err(url, why),
    };

}

fn write_err(url: &str, error: hyper::Error) {
    println!("Error with {}: {}", url, error)
}

fn write_resource(url: &str, response: &mut client::Response){ 
    let filename = format!("./out/{}",get_filename_from_url(&url));
    let path = Path::new(&filename);
    let file =  File::create(&path).unwrap();
    let mut writer = BufWriter::new(file);

    let mut buf = Vec::new();
    if response.read_to_end(&mut buf).is_ok() {
        writer.write(buf.as_slice()).expect("IO Error");
    }
}


fn main() {
    env_logger::init().unwrap();
    let client = Arc::new(Client::new());

    let matches = App::new("servo-connection-test")
                            .bin_name("servo_connection_test")
                            .version("1.0")
                            .author("Diane Hosfelt dhosfelt@mozilla.com")
                            .about("models resource fetching with hyper")
                            .arg(Arg::with_name("threads")
                                .short("t")
                                .long("threads")
                                .help("number of threads in connection pool")
                                .takes_value(true))
                            .arg(Arg::with_name("no_io")
                                 .short("n")
                                 .long("no_io")
                                 .help("skips resource writing"))
                            .arg(Arg::with_name("close_connection")
                                 .short("c")
                                 .long("close_conn")
                                 .help("Closes all connections instead of keeping them alive"))
                            .get_matches();
    
    let threads = matches.value_of("threads").unwrap_or("8").parse::<u32>().unwrap(); 
    let io_flag = !matches.is_present("no_io");
    let conn_flag = matches.is_present("close_connection");

    if !Path::new("./out").is_dir() {
        fs::create_dir("./out").expect("Couldn't create ./out");
    }


    //open resources.txt and iterate through lines
    let path = Path::new("resources.txt");
    let exists = std::fs::metadata(path);
    if exists.is_err() {
       panic!("Please create resources.txt"); 
    }
    let file = File::open(&path).unwrap();
    let mut resources = BufReader::new(file);
   
    let mut base_url = String::new();
    resources.read_line(&mut base_url).unwrap();
    println!("{}", base_url);

    /*
     *  TODO not sure if this is the best way 
     *  http://seanmonstar.com/post/141495445652/async-hyper
     *
     *  hyper has it's own connection pooling--https://github.com/hyperium/hyper/blob/master/src/client/pool.rs  -- on by default
     * http://hyper.rs/hyper/0.8.0/hyper/client/struct.Client.html#method.with_pool_config
     *
     */
    let mut pool = Pool::new(threads);
    pool.scoped(|scope| {
        for l in resources.lines() {
            let c = client.clone();
            scope.execute(move || {
                let line = l.unwrap();
                fetch_resource(&line, &c, io_flag, conn_flag);
            });
        }
    });
}

