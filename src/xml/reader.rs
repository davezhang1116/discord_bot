use std::fs::File;
use std::io::BufReader;
use xml::common::Position;
use xml::reader::{ParserConfig, XmlEvent};

#[derive(Debug)]
pub struct XmlData{
    pub name: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub port: String,
    pub database: String,
    pub token: String,
}

pub fn get_data() -> XmlData{
    let file_path = "file.xml";
    let file = File::open(file_path).unwrap();

    let mut reader = ParserConfig::default()
        .ignore_root_level_whitespace(true)
        .create_reader(BufReader::new(file));

    let mut data_vec = Vec::new();
    let mut xml_data = XmlData{
        name: String::from(""),
        username: String::from(""),
        password: String::from(""),
        url: String::from(""),
        port: String::from(""),
        database: String::from(""),
        token: String::from("")
    };

    loop {
        match reader.next() {
            Ok(e) => {
                match e {
                    XmlEvent::EndDocument => {
                        break;
                    },
                    XmlEvent::StartElement { name, .. } => {
                        data_vec.push(format!("{name}"));
                    },
                    XmlEvent::Characters(data) => {
                        data_vec.push(format!("{}", data.trim().escape_debug()));
                    }
                    _ => {}
                }
            }
            Err(e) => {
                println!("Error at {}: {e}", reader.position());
                break;
            },
        }
    }

    if &data_vec[0] != &String::from("data"){
        println!("ERROR");
    }else{
        for i in 1..data_vec.len(){
            match data_vec[i].as_str(){
                "name" => {if xml_data.name == String::from(""){ xml_data.name = data_vec[i+1].clone()}},
                "username" => {if xml_data.username == String::from(""){ xml_data.username = data_vec[i+1].clone()}},
                "password" => {if xml_data.password == String::from(""){ xml_data.password = data_vec[i+1].clone()}},
                "url" => {if xml_data.url == String::from(""){ xml_data.url = data_vec[i+1].clone()}},
                "port" => {if xml_data.port == String::from(""){ xml_data.port = data_vec[i+1].clone()}},
                "database" => {if xml_data.database == String::from(""){ xml_data.database = data_vec[i+1].clone()}},
                "token" => {if xml_data.token == String::from(""){ xml_data.token = data_vec[i+1].clone()}},
                _ => {}
            }
        }
    }
    xml_data
}