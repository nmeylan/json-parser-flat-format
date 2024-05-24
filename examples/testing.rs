fn main() {
    let str = "{  \n \"b\"}".as_bytes();
    // let mask = 0x7b7b7b7b7b7b7b7b;
    let mask = 0x0101010101010101 * b'\n' as u64;
    let bytes = u64::from_le_bytes([str[0], str[1], str[2], str[3], str[4], str[5], str[6], str[7]]);
    let comparison = mask ^ bytes;
    let high_bit_mask1 = (((comparison >> 1) | 0x8080808080808080) - comparison) & 0x8080808080808080;
    println!("{}", comparison);
    println!("{}", high_bit_mask1);
    let position = if high_bit_mask1 == 0 {
        None
    } else {
        Some(high_bit_mask1.trailing_zeros() >> 3)
    };
    println!("Position of {{ {:?}", position);

    let i = high_bit_mask1 == 0 && str[0] & 0x01 * b'{' != 0x01 * b'{';
    println!("{}", i);

}