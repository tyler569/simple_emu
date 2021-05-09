pub struct Simple {
    regfile: [u16; Self::REGISTER_COUNT],
    ram: [u8; 65536],
}

impl Simple {
    const STACK_POINTER: usize = 15;
    const INSTRUCTION_POINTER: usize = 16;
    const FLAG_REGISTER: usize = 17;
    const REGISTER_COUNT: usize = 16 * 4;

    pub fn new() -> Self {
        Simple {
            regfile: [0; Self::REGISTER_COUNT],
            ram: [0; 65536],
        }
    }

    pub fn load_program(&mut self, program: Vec<u8>) {
        for (index, byte) in program.iter().enumerate() {
            self.ram[index] = *byte;
        }
    }

    fn ip(&self) -> usize {
        self.regfile[Self::INSTRUCTION_POINTER] as usize
    }

    fn advance_ip(&mut self, amount: usize) {
        self.regfile[Self::INSTRUCTION_POINTER] = 
            self.regfile[Self::INSTRUCTION_POINTER].wrapping_add(amount as u16);
    }

    fn flags(&self) -> u16 {
        self.regfile[Self::FLAG_REGISTER]
    }

    fn zf(&self) -> bool {
        self.flags() & alu::ZF > 0
    }

    fn cf(&self) -> bool {
        self.flags() & alu::CF > 0
    }

    fn of(&self) -> bool {
        self.flags() & alu::OF > 0
    }

    fn sf(&self) -> bool {
        self.flags() & alu::SF > 0
    }

    fn ef(&self) -> bool {
        self.flags() & alu::EF > 0
    }

    fn read_16(&self, address: usize) -> u16 {
        ((self.ram[address] as u16) << 8) +
            self.ram[address.wrapping_add(1)] as u16
    }

    fn write_16(&mut self, address: usize, value: u16) {
        match address {
            0xFF01 => {
                println!("{:#x}", value);
            }
            _ => {
                self.ram[address] = (value >> 8) as u8;
                self.ram[address.wrapping_add(1)] = value as u8;
            }
        }
    }

    fn push(&mut self, value: u16) {
        self.regfile[Self::STACK_POINTER] =
            self.regfile[Self::STACK_POINTER].wrapping_sub(2);
        self.write_16(self.regfile[Self::STACK_POINTER] as usize, value);
    }

    fn pop(&mut self) -> u16 {
        let value = self.read_16(self.regfile[Self::STACK_POINTER] as usize);
        self.regfile[Self::STACK_POINTER] =
            self.regfile[Self::STACK_POINTER].wrapping_add(2);
        value
    }

    fn should_jump(&self, cond: usize) -> bool {
        match cond {
            1 => !self.zf() && !self.cf(),
            2 => !self.cf(),
            3 => self.cf(),
            4 => self.cf() || self.zf(),
            5 => !self.zf() && self.sf() == self.of(),
            6 => self.sf() == self.of(),
            7 => self.sf() != self.of(),
            8 => !self.zf() || self.sf() != self.zf(),
            9 => self.zf(),
            10 => !self.zf(),
            11 => self.of(),
            12 => !self.of(),
            13 => true,
            _ => false,
        }
    }

    pub fn step(&mut self) -> bool {
        let instruction = self.read_16(self.ip()) as usize;
        eprintln!("{:>2}: {:0>16b}  {:>2x?}",
            self.ip(), instruction, &self.regfile[0..8]);
        if instruction == 0 {
            return false;
        }
        match instruction >> 12 {
            0b0000 if instruction >> 8 == 0 => { // 1op
                let rd = instruction & 0b1111;
                match instruction >> 4 {
                    1 => self.regfile[rd] = !self.regfile[rd],
                    2 => self.regfile[rd] = !self.regfile[rd].wrapping_add(1),
                    3 => self.push(self.regfile[rd]),
                    4 => self.regfile[rd] = self.pop(),
                    5 => self.regfile[rd] = self.regfile[rd].wrapping_add(1),
                    6 => self.regfile[rd] = self.regfile[rd].wrapping_sub(1),
                    _ => todo!(),
                };
                self.advance_ip(2);
                true
            }
            0b000 => { // 2op
                let op = (instruction >> 8) & 0b1111;
                let rd = (instruction >> 4) & 0b1111;
                let rs = instruction & 0b1111;
                let va = self.regfile[rd];
                let vb = self.regfile[rs];
                let (result, flags) = alu::alu(op, va, vb, self.flags());
                self.regfile[Self::FLAG_REGISTER] = flags;
                self.regfile[rd] = result;
                self.advance_ip(2);
                true
            }
            0b0001 => { // j? abs
                let cond = (instruction >> 8) & 0b1111;
                let rd = (instruction >> 4) & 0b1111;
                let typ = instruction & 0b1111;
                let has_immediate = typ == 2;
                let target;
                match typ {
                    0 => target = self.regfile[rd],
                    1 => target = self.read_16(self.regfile[rd] as usize),
                    2 => target = self.read_16(self.ip().wrapping_add(2)),
                    _ => todo!(),
                }
                self.advance_ip(2);
                if has_immediate {
                    self.advance_ip(2);
                }
                if self.should_jump(cond) {
                    self.regfile[Self::INSTRUCTION_POINTER] = target;
                }
                true
            }
            0b0010 => { // 2op immediate
                let op = (instruction >> 8) & 0b1111;
                let rd = (instruction >> 4) & 0b1111;
                let n = (instruction & 0b1111) as u16;
                let va = self.regfile[rd];
                let (result, flags) = alu::alu(op, va, n, self.flags());
                self.regfile[Self::FLAG_REGISTER] = flags;
                self.regfile[rd] = result;
                self.advance_ip(2);
                true
            }
            0b0011 => { // j? relative
                let cond = (instruction >> 8) & 0b1111;
                let target = (instruction & 0b1111_1111) as i8 as i16 as usize;
                self.advance_ip(2);
                if self.should_jump(cond) {
                    self.advance_ip(target);
                }
                true
            }
            0b0100 => { // mov rN, [rS + rO]
                let rd = (instruction >> 8) & 0b1111;
                let rs = (instruction >> 4) & 0b1111;
                let ro = instruction & 0b1111;
                let address = self.regfile[rs].wrapping_add(self.regfile[ro]);
                self.regfile[rd] = self.read_16(address as usize);
                self.advance_ip(2);
                true
            }
            0b0101 => { // mov [rN + rO], rS
                let rd = (instruction >> 8) & 0b1111;
                let rs = (instruction >> 4) & 0b1111;
                let ro = instruction & 0b1111;
                let address = self.regfile[rd].wrapping_add(self.regfile[ro]);
                self.write_16(address as usize, self.regfile[rs]);
                self.advance_ip(2);
                true
            }
            // 0b0110 empty
            // 0b0111 empty
            0b1000 => { // mov rN, i8
                let rd = (instruction >> 8) & 0b1111;
                let n = instruction & 0b1111_1111;
                self.regfile[rd] = n as u16;
                self.advance_ip(2);
                true
            }
            0b1001 => { // mov rN, i16
                let rd = (instruction >> 8) & 0b1111;
                let n = self.read_16(self.ip().wrapping_add(2));
                self.regfile[rd] = n;
                self.advance_ip(4);
                true
            }
            // 0b1010 empty
            0b1011 => { // mov rNpN, rNpN
                let rd = (instruction >> 8) & 0b1111;
                let rs = (instruction >> 4) & 0b1111;
                let pd = (instruction >> 2) & 0b11;
                let ps = instruction & 0b11;
                eprintln!("mov r{}, r{}", rd + pd * 16, rs + ps * 16);
                self.regfile[rd + pd * 16] = self.regfile[rs + ps * 16];
                self.advance_ip(2);
                true
            }
            // 0b1100 - 0b1111 empty
            _ => {
                todo!();
            }
        }
    }

    pub fn run(&mut self) {
        while self.step() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_program() {
        // mov r1, 10
        // mov r2, 11
        // add r2, r1
        let program = vec![0x81,0x0a,0x82,0x0b,0x01,0x21];
        let mut s = Simple::new();
        s.load_program(program);
        s.run();
        assert_eq!(s.regfile[2], 21);
    }

    #[test]
    fn jmp_program() {
        // mov r1, 1
        // jmp [ip + 2]
        // mov r2, 2
        let program = vec![0x81,0x01,0x3d,0x02,0x82,0x02];
        let mut s = Simple::new();
        s.load_program(program);
        s.run();
        assert_eq!(s.regfile[1], 1);
        assert_eq!(s.regfile[2], 0);
    }

    #[test]
    fn fib_program() {
        let program = vec![
            0x2b,0x10,0x39,0x12,0x82,0x00,0x83,0x01,
            0x22,0x11,0x39,0x0c,0x01,0x23,0x22,0x11,
            0x39,0x0a,0x01,0x32,0x3d,0xf2,0x00,0x00,
            0xb1,0x20,0x00,0x00,0xb1,0x30,0x00,0x00,
        ];
        let mut s = Simple::new();
        s.load_program(program);
        s.regfile[1] = 11;
        s.run();
        assert_eq!(s.regfile[1], 55);
    }

    #[test]
    fn inc_program() {
        let program = vec![0x00,0x51,0x00,0x51,0x00,0x51];
        let mut s = Simple::new();
        s.load_program(program);
        s.run();
        assert_eq!(s.regfile[1], 3);
    }

    #[test]
    fn stack_program() {
        let program = vec![
            0x81,0xff,0x00,0x31,0x00,0x31,0x00,0x31,
            0x00,0x42,0x00,0x43,0x00,0x44,0x00,0x45,
        ];
        let mut s = Simple::new();
        s.load_program(program);
        s.run();
        assert_eq!(s.regfile[1], 255);
        assert_eq!(s.regfile[2], 255);
        assert_eq!(s.regfile[3], 255);
        assert_eq!(s.regfile[4], 255);
        assert_ne!(s.regfile[5], 255);
        assert_eq!(s.regfile[15], 2);
    }
}

mod alu {
    // TODO: bitflags!?
    type Flags = u16;
    pub const ZF: u16 = 0b0001;
    pub const CF: u16 = 0b0010;
    pub const OF: u16 = 0b0100;
    pub const SF: u16 = 0b1000;
    pub const EF: u16 = 0b100_0000;

    type AluResult = (u16, Flags);
    type AluOp = fn(u16, u16, Flags) -> AluResult;

    pub fn alu(op: usize, a: u16, b: u16, flags: Flags) -> AluResult {
        if let Some(op) = dispatch_op(op) {
            op(a, b, flags)
        } else {
            (0, EF)
        }
    }

    fn dispatch_op(op: usize) -> Option<AluOp> {
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

    fn add(a: u16, b: u16, _f: Flags) -> AluResult {
        let (c, cf) = a.overflowing_add(b);
        (c, flags(c, cf))
    }

    fn sub(a: u16, b: u16, _f: Flags) -> AluResult {
        let (c, cf) = a.overflowing_sub(b);
        (c, flags(c, cf))
    }

    fn or(a: u16, b: u16, _f: Flags) -> AluResult {
        let c = a | b;
        (c, flags(c, false))
    }

    fn nor(a: u16, b: u16, _f: Flags) -> AluResult {
        let c = !(a | b);
        (c, flags(c, false))
    }

    fn and(a: u16, b: u16, _f: Flags) -> AluResult {
        let c = a & b;
        (c, flags(c, false))
    }

    fn nand(a: u16, b: u16, _f: Flags) -> AluResult {
        let c = !(a & b);
        (c, flags(c, false))
    }

    fn xor(a: u16, b: u16, _f: Flags) -> AluResult {
        let c = a ^ b;
        (c, flags(c, false))
    }

    fn xnor(a: u16, b: u16, _f: Flags) -> AluResult {
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

    fn cmp(a: u16, b: u16, _f: Flags) -> AluResult {
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

