use crate::command;
use crate::command::Command;
use env_logger::Builder;
#[test]
fn set_command_test(){
    let ucmd = String::from("set name mambisi");
    match command::parse(&ucmd){
        Ok(c) => {
            c.execute();
            info!("Hello World");
            assert!(true)
        },
        Err(e) => assert!(false,e.to_string())
    };
}

#[test]
fn set_command_with_type_test(){
    let ucmd = String::from("set name json mambisi");
    match command::parse(&ucmd){
        Ok(c) => {
            c.execute();
            info!("Hello World");
            assert!(true)
        },
        Err(e) => assert!(false,e.to_string())
    };
}


#[test]
fn set_command_test_valid_with_expiration(){
    let ucmd = String::from("set name mambisi ex 2");
    match command::parse(&ucmd){
        Ok(c) => {
            c.execute();
            assert!(true)
        },
        Err(e) => assert!(false,e.to_string())
    };
}

#[test]
fn set_command_test_invalid(){
    let ucmd = String::from("set name mambisi joke");
    match command::parse(&ucmd){
        Ok(c) => {
            c.execute();
            assert!(false)
        },
        Err(e) => assert!(true,e.to_string())
    };
}