struct Simple {
    regfile: [u16; 128],
}

impl Simple {
}

mod alu {
    // TODO: bitflags!?
    type Flags = u16;
    const ZF: u16 = 0b0001;
    const CF: u16 = 0b0010;
    const OF: u16 = 0b0100;
    const SF: u16 = 0b1000;
    const EF: u16 = 0b100_0000;

    type AluResult = (u16, Flags);
    type AluOp = fn(u16, u16, Flags) -> AluResult;

    pub fn alu(op: u16, a: u16, b: u16, flags: Flags) -> AluResult {
        if let Some(op) = dispatch_op(op) {
            op(a, b, flags)
        } else {
            (0, EF)
        }
    }

    fn dispatch_op(op: u16) -> Option<AluOp> {
        match op {
            1 => Some(add),
            2 => Some(sub),
            3 => Some(or),
            4 => Some(nor),
            5 => Some(and),
            6 => Some(nand),
            7 => Some(xor),
            8 => Some(xnor),
            9 => Some(adc),
            10 => Some(sbb),
            11 => Some(cmp),
            _ => None,
        }
    }

    fn flags(c: u16, cf: bool) -> Flags {
        let zf = c == 0;
        let sf = c & 0x8000 > 0;
        let of = !cf && sf;
        zf as u16 +
            ((cf as u16) << 1) +
            ((of as u16) << 2) +
            ((sf as u16) << 3)
    }

    fn add(a: u16, b: u16, f: Flags) -> AluResult {
        let (c, cf) = a.overflowing_add(b);
        (c, flags(c, cf))
    }

    fn sub(a: u16, b: u16, f: Flags) -> AluResult {
        let (c, cf) = a.overflowing_sub(b);
        (c, flags(c, cf))
    }

    fn or(a: u16, b: u16, f: Flags) -> AluResult {
        let c = a | b;
        (c, flags(c, false))
    }

    fn nor(a: u16, b: u16, f: Flags) -> AluResult {
        let c = !(a | b);
        (c, flags(c, false))
    }

    fn and(a: u16, b: u16, f: Flags) -> AluResult {
        let c = a & b;
        (c, flags(c, false))
    }

    fn nand(a: u16, b: u16, f: Flags) -> AluResult {
        let c = !(a & b);
        (c, flags(c, false))
    }

    fn xor(a: u16, b: u16, f: Flags) -> AluResult {
        let c = a ^ b;
        (c, flags(c, false))
    }

    fn xnor(a: u16, b: u16, f: Flags) -> AluResult {
        let c = !(a ^ b);
        (c, flags(c, false))
    }

    fn cf(f: Flags) -> u16 {
        if f & CF > 0 { 1 } else { 0 }
    }

    fn adc(a: u16, b: u16, f: Flags) -> AluResult {
        // TODO: are these flags right if a + CF overflows?
        let a = a.wrapping_add(cf(f));
        let (c, cf) = a.overflowing_add(b);
        (c, flags(c, cf))
    }

    fn sbb(a: u16, b: u16, f: Flags) -> AluResult {
        // TODO: are these flags right if a - CF overflows?
        let a = a.wrapping_sub(cf(f));
        let (c, cf) = a.overflowing_sub(b);
        (c, flags(c, cf))
    }

    fn cmp(a: u16, b: u16, f: Flags) -> AluResult {
        let (c, cf) = a.overflowing_sub(b);
        (a, flags(c, cf))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn simple_addition() {
            assert_eq!(add(5, 10, 0), (15, 0))
        }

        #[test]
        fn simple_subtraction() {
            assert_eq!(sub(10, 5, 0), (5, 0))
        }

        #[test]
        fn negative_subtraction() {
            assert_eq!(sub(5, 10, 0), (-5i16 as u16, SF | CF));
        }

        #[test]
        fn simple_or() {
            assert_eq!(or(5, 10, 0), (15, 0))
        }

        #[test]
        fn simple_nor() {
            assert_eq!(nor(3, 5, 0), (!7u16, SF | OF))
        }

        #[test]
        fn simple_and() {
            assert_eq!(and(3, 5, 0), (1, 0))
        }

        #[test]
        fn simple_nand() {
            assert_eq!(nand(3, 5, 0), (!1u16, SF | OF))
        }

        #[test]
        fn simple_xor() {
            assert_eq!(xor(3, 5, 0), (6, 0))
        }

        #[test]
        fn simple_xnor() {
            assert_eq!(xnor(3, 5, 0), (!6u16, SF | OF))
        }

        #[test]
        fn simple_adc() {
            assert_eq!(adc(3, 5, 0), (8, 0))
        }

        #[test]
        fn simple_sbb() {
            assert_eq!(sbb(5, 3, 0), (2, 0))
        }

        #[test]
        fn carrying_adc() {
            assert_eq!(adc(3, 5, 0b0010), (9, 0))
        }

        #[test]
        fn borrowing_sbb() {
            assert_eq!(sbb(5, 3, 0b0010), (1, 0))
        }

        #[test]
        fn overflowing_carrying_adc() {
            assert_eq!(adc(0xFFFE, 1, 0b0010), (0, CF | ZF))
        }

        #[test]
        fn overflowing_borrowing_sbb() {
            assert_eq!(sbb(0x8001, 1, 0b0010), (0x7FFF, 0))
        }

        #[test]
        fn simple_cmp_gt() {
            assert_eq!(cmp(5, 3, 0), (5, 0))
        }

        #[test]
        fn simple_cmp_eq() {
            assert_eq!(cmp(5, 5, 0), (5, ZF))
        }

        #[test]
        fn simple_cmp_lt() {
            assert_eq!(cmp(5, 8, 0), (5, SF | CF))
        }

        #[test]
        fn dispatch() {
            // 1 + 1 = 2
            assert_eq!(alu(1, 1, 1, 0), (2, 0))
        }
    }
}
