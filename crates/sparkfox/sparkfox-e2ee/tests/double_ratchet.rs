//! Double Ratchet 端到端加密测试
#![forbid(unsafe_code)]

use sparkfox_e2ee::{EncryptedPayload, Session, X25519KeyPair};

#[test]
fn encrypt_decrypt_roundtrip() {
    let alice = X25519KeyPair::generate();
    let bob = X25519KeyPair::generate();
    let mut alice_session = Session::init_alice(&alice, bob.public_key()).expect("Alice init");
    let mut bob_session = Session::init_bob(&bob, alice.public_key()).expect("Bob init");
    let plaintext = b"hello sparkfox e2ee";
    let payload: EncryptedPayload = alice_session.encrypt(plaintext).expect("Alice encrypt");
    let decrypted = bob_session.decrypt(&payload).expect("Bob decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn message_order_independence() {
    // 即使消息乱序到达，也应该能解密（out-of-order 解密能力）
    let alice = X25519KeyPair::generate();
    let bob = X25519KeyPair::generate();
    let mut a = Session::init_alice(&alice, bob.public_key()).unwrap();
    let mut b = Session::init_bob(&bob, alice.public_key()).unwrap();
    let p1 = a.encrypt(b"msg1").unwrap();
    let p2 = a.encrypt(b"msg2").unwrap();
    // 反向到达
    let d2 = b.decrypt(&p2).unwrap();
    let d1 = b.decrypt(&p1).unwrap();
    assert_eq!(d1, b"msg1");
    assert_eq!(d2, b"msg2");
}
