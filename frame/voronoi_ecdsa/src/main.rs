//// ECDSA Voronoi
//// by six@QRUCIAL
use p256::{ecdsa::{SigningKey, Signature, signature::Signer, VerifyingKey, signature::Verifier}};
use rand_core::OsRng; 


fn ecdsa_test(action: String, project: String) -> bool{
    
    // Verify if _action_ is allowed for _project_ signable
    // This needs to be related to consensus - all nodes need
    // to agree to move on with signing. + time limit? type? TBF
    let tobechecked = if action {
        "ok"
    } else {
        return false;
    };

    // action to sign
    //println!("action: {}",action);
    println!("action: {}",action);
    let mut msg=action.as_bytes();
    
    // Signing key from os random
    // To be replaced to PRIVATE NODE KEY
    // How many nodes need to sign? threshold ecdsa?
    // https://rustrepo.com/repo/KZen-networks-multi-party-ecdsa
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
    if rtn==true { println!("\naction '{0}' signature correct", action); }
    else { println!("\naction '{0}' signature incorrect",action);}
   
    // Test
    msg="Test eval to incorrect".as_bytes();
    let rtn=verify_key.verify(msg, &signature).is_ok();
    if rtn==true { println!("\naction signature is correct."); }
    else { println!("\naction signature is INcorrect.");}

}
