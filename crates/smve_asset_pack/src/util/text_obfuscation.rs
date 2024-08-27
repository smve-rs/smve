//! Functions to toggle obfuscation used by the asset uncookers

/// Obfuscates/deobfuscates the input data.
///
/// Obfuscation and deobfuscation is the same function because it is reversible.
pub fn toggle_obfuscation(input: &[u8]) -> Vec<u8> {
    // Power of 2 so that compiler can optimize modulo
    const PASSWORD_LENGTH: usize = 16;
    const PASSWORD: &[u8] = b"invalid pointer\0";
    // Output = in XOR NOT(password)
    let mut result = input.to_vec();
    for i in 0..input.len() {
        result[i] ^= !PASSWORD[i % PASSWORD_LENGTH];
    }
    result
}
