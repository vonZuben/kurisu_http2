/// 5.1 Integer Representation
/// Integers are used to represent name indexes, header field indexes, or string lengths. An integer representation can start anywhere within an octet. To allow for optimized processing, an integer representation always finishes at the end of an octet.
///
/// An integer is represented in two parts: a prefix that fills the current octet and an optional list of octets that are used if the integer value does not fit within the prefix. The number of bits of the prefix (called N) is a parameter of the integer representation.
///
/// If the integer value is small enough, i.e., strictly less than 2N-1, it is encoded within the N-bit prefix.
///
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | ? | ? | ? |       Value       |
/// +---+---+---+-------------------+
/// Figure 2: Integer Value Encoded within the Prefix (Shown for N = 5)
///
/// Otherwise, all the bits of the prefix are set to 1, and the value, decreased by 2N-1, is encoded using a list of one or more octets. The most significant bit of each octet is used as a continuation flag: its value is set to 1 except for the last octet in the list. The remaining bits of the octets are used to encode the decreased value.
///
///   0   1   2   3   4   5   6   7
/// +---+---+---+---+---+---+---+---+
/// | ? | ? | ? | 1   1   1   1   1 |
/// +---+---+---+-------------------+
/// | 1 |    Value-(2^N-1) LSB      |
/// +---+---------------------------+
///                ...
/// +---+---------------------------+
/// | 0 |    Value-(2^N-1) MSB      |
/// +---+---------------------------+
/// Figure 3: Integer Value Encoded after the Prefix (Shown for N = 5)
///
/// Decoding the integer value from the list of octets starts by reversing the order of the octets in the list. Then, for each octet, its most significant bit is removed. The remaining bits of the octets are concatenated, and the resulting value is increased by 2N-1 to obtain the integer value.
///
/// The prefix size, N, is always between 1 and 8 bits. An integer starting at an octet boundary will have an 8-bit prefix.
///

// pub fn decode_integer<'a, B: IntoIterator<Item=&'a u8>>(bts: B, prefix_size: u8) -> Result<u32, &'static str> {
pub fn decode_integer<'a, 'b, I: Iterator<Item=&'b u8>>(bts: &'a mut I, prefix_size: u8) -> Result<u32, &'static str> {
    use std::num::Wrapping;

    if prefix_size < 1 || prefix_size > 8 {
        return Err("hpack integer: invalid prefix");
    }
    // if bts.peek().is_none() {
    //     return Err("hpack integer: not enough octets (0)");
    // }

    // Make sure there's no overflow in the shift operation
    let Wrapping(mask) = if prefix_size == 8 {
        Wrapping(0xFFu8)
    } else {
        Wrapping(1u8 << prefix_size) - Wrapping(1)
    };

    let tv = bts.next();

    if tv.is_none() { return Err("hpack integer: not enough octets (0)"); }

    let mut value = (tv.unwrap() & mask) as u32;

    // if there is only one octet in the encodeing
    if value < mask as u32 {
        // Value fits in the prefix bits.
        return Ok(value);
    }

    // The value does not fit into the prefix bits, so we read as many following
    // bytes as necessary to decode the integer.
    // Already one byte used (the prefix)
    let mut m = 0;
    // The octet limit is chosen such that the maximum allowed *value* can
    // never overflow an unsigned 32-bit integer. The maximum value of any
    // integer that can be encoded with 5 octets is ~2^28
    let octet_limit = 5;

    for (i, b) in bts.enumerate() {
        value += ((b & 127) as u32) * (1 << m);
        m += 7;

        if b & 128 != 128 {
            // Most significant bit is not set => no more continuation bytes
            return Ok(value);
        }

        if i == octet_limit {
            // The spec tells us that we MUST treat situations where the
            // encoded representation is too long (in octets) as an error.
            return Err("hpack integer: to many octets");
        }
    }

    // If we have reached here, it means the buffer has been exhausted without
    // hitting the termination condition.
    Err("hpack integer: not enough octets")
}

// encode n into bst
pub fn encode_integer<'a, 'b, I: Iterator<Item=&'b mut u8>>(n: u32, bts: &'a mut I, prefix_size: u8) {
    let mut n = n;
    let check = ( 1 << prefix_size ) - 1;

    let first_byte = bts.next().unwrap();

    *first_byte = 0;

    if n < check {
        *first_byte |= n as u8;
        return;
    }

    *first_byte |= check as u8;
    n -= check;

    loop {
        let br = bts.next().unwrap();

        if n < 128 {
            *br = n as u8;
            break;
        }

        *br = 0x80 | ( n as u8 & 0x7f );
        n >>= 7;

        if n == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_integer, encode_integer};

    #[test]
    fn decode_test() {
        // simple tst
        let tst_num = vec![0x41u8];
        let num = decode_integer(&mut tst_num.iter(), 8).unwrap();
        assert_eq!(num, 65);

        // complex number
        let tst_num = vec![0xFF, 0x05];
        let num = decode_integer(&mut tst_num.iter(), 8).unwrap();
        assert_eq!(num, 260);

        // more complex number
        let tst_num = vec![0x1F, 0x9A, 0x0A];
        let num = decode_integer(&mut tst_num.iter(), 5).unwrap();
        assert_eq!(num, 1337);
    }

    // this test relise on decodeing to work
    #[test]
    fn encode_test() {
        let mut vec = vec![0; 10];

        // simple
        let tst_code = vec![0x4];
        encode_integer(4, &mut vec.iter_mut(), 8);
        let num = decode_integer(&mut vec.iter(), 8).unwrap();
        assert_eq!(num, 4);

        // little less simple
        let tst_code = vec![0x03, 0x01];
        encode_integer(4, &mut vec.iter_mut(), 2);
        let num = decode_integer(&mut vec.iter(), 2).unwrap();
        assert_eq!(num, 4);

        // more complex
        let tst_code = vec![0x1F, 0x9A, 0x0A];
        encode_integer(1337, &mut vec.iter_mut(), 5);
        let num = decode_integer(&mut vec.iter(), 5).unwrap();
        assert_eq!(num, 1337);
    }
}
