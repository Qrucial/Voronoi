//// ECDSA
use p256::{ecdsa::{SigningKey, Signature, signature::Signer, VerifyingKey, signature::Verifier}};
use rand_core::OsRng; 

//// Rocket.rs
#[macro_use] extern crate rocket;

#[get("/v")]
fn v() -> &'static str {
    ecdsa_test("testme".to_string());
    "test ok, chk terminal"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![v])
    //rocket::build().mount("/v", routes![v]);
}

//#[get("/v/<msg>")]
//fn v(msg: String) -> &'static str {
//        format!("See this in terminal now: {}!", msg)
//}


//// Main with ecdsa+rocket
fn ecdsa_test(message: String) {

    // Message to sign
    //let mut message = String::from(message);
    //println!("Message: {}",message);
    println!("Message: {}",message);
    let mut msg=message.as_bytes();
    
    // Signing key from os random
    let signing_key = SigningKey::random(&mut OsRng); 
    let sk=signing_key.to_bytes();
    println!("\nSigning key: {:x?}",hex::encode(sk));
    
    // Verification key from signing key
    let verify_key = VerifyingKey::from(&signing_key); 
    let vk=verify_key.to_encoded_point(false);
    println!("\nVerifying key: {:x?}",hex::encode(vk));
    
    // Signing process
    let signature: Signature = signing_key.sign(msg);
    println!("\nSignature: {:x?}",hex::encode(signature));
    
    // Verification process
    let rtn=verify_key.verify(msg, &signature).is_ok();
    
    // Evaluation of verification
    if rtn==true { println!("\nMessage '{0}' signature correct", message); }
    else { println!("\nMessage '{0}' signature incorrect",message);}
   
    // Test
    msg="Test eval to incorrect".as_bytes();
    let rtn=verify_key.verify(msg, &signature).is_ok();
    if rtn==true { println!("\nMessage signature is correct."); }
    else { println!("\nMessage signature is INcorrect.");}

}
