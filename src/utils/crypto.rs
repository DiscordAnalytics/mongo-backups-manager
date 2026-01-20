use chacha20poly1305::{
  ChaCha20Poly1305, Key,
  aead::{KeyInit, OsRng},
};

pub fn generate_key() -> String {
  let key = ChaCha20Poly1305::generate_key(&mut OsRng);

  key.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn key_to_cipher(s: &str) -> ChaCha20Poly1305 {
  let bytes: Vec<u8> = (0..s.len())
    .step_by(2)
    .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("Invalid hex"))
    .collect();

  ChaCha20Poly1305::new(Key::from_slice(&bytes))
}
