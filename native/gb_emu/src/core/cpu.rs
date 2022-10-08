use std::cell::RefCell;
use std::rc::Rc;
use crate::core::convention::Term;

use crate::core::memory::Memory;
use crate::core::register::{Flag, Register};

// 每条指令所花费的机器周期，1机器周期 = 4时钟周期
const OP_CYCLES: [u32; 256] = [
//  0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
    1, 3, 2, 2, 1, 1, 2, 1, 5, 2, 2, 2, 1, 1, 2, 1, // 0
    0, 3, 2, 2, 1, 1, 2, 1, 3, 2, 2, 2, 1, 1, 2, 1, // 1
    2, 3, 2, 2, 1, 1, 2, 1, 2, 2, 2, 2, 1, 1, 2, 1, // 2
    2, 3, 2, 2, 3, 3, 3, 1, 2, 2, 2, 2, 1, 1, 2, 1, // 3
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // 4
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // 5
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // 6
    2, 2, 2, 2, 2, 2, 0, 2, 1, 1, 1, 1, 1, 1, 2, 1, // 7
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // 8
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // 9
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // a
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, // b
    2, 3, 3, 4, 3, 4, 2, 4, 2, 4, 3, 0, 3, 6, 2, 4, // c
    2, 3, 3, 0, 3, 4, 2, 4, 2, 4, 3, 0, 3, 0, 2, 4, // d
    3, 3, 2, 0, 0, 4, 2, 4, 4, 1, 4, 0, 0, 0, 2, 4, // e
    3, 3, 2, 1, 0, 4, 2, 4, 3, 2, 4, 1, 0, 0, 2, 4, // f
];

// 每条扩展指令所花费的机器周期
const EXT_OP_CYCLES: [u32; 256] = [
//  0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 0
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 1
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 2
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 3
    2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, // 4
    2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, // 5
    2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, // 6
    2, 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 2, 3, 2, // 7
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 8
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // 9
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // a
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // b
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // c
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // d
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // e
    2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, // f
];

pub struct Cpu {
    pub reg: Register,
    pub mem: Rc<RefCell<dyn Memory>>,
    pub halted: bool,
    pub ei: bool,
}

impl Cpu {
    #[inline(always)]
    fn get_mem(&self, a: u16) -> u8 {
        (*self.mem).borrow().get(a)
    }

    #[inline(always)]
    fn get_mem_word(&self, a: u16) -> u16 {
        (*self.mem).borrow().get_word(a)
    }

    #[inline(always)]
    fn set_mem(&mut self, a: u16, v: u8) {
        self.mem.borrow_mut().set(a, v);
    }

    #[inline(always)]
    fn set_mem_word(&mut self, a: u16, v: u16) {
        self.mem.borrow_mut().set_word(a, v);
    }

    // 从内存地址（保存在寄存器hl）中取出值
    #[inline(always)]
    fn get_mem_hl(&self) -> u8 {
        self.get_mem(self.reg.get_hl())
    }

    // 根据a+b的计算结果r，判断加法的计算过程中，第3位是否发生进位
    #[inline(always)]
    fn is_half_carry(a: u8, b: u8, r: u8) -> bool {
        // 异或操作^可以表示两个一位数相加或相减的结果，例如：
        // 0+0 == 0^0 == 0, 1+1 == 1^1 == 0,  0+1 == 0^1 == 1
        // 0-0 == 0^0 == 0, 1-1 == 1^1 == 0,  0-1 == 0^1 == 1
        // 我们可以用 (a^b) & 0x10 得到不考虑进位时，a+b 结果的第4位，如果它和r的第4位相同，
        // 则表示第3位没有发生进位（此时(a^b^r)的第4位为0），否则第3位发生进位（此时(a^b^r)的第4位为1）
        return (a ^ b ^ r) & 0x10 > 0;
    }

    // 根据a-b的计算结果r，判断减法的计算过程中，第3位是否发生借位
    #[inline(always)]
    fn is_half_borrow(a: u8, b: u8, r: u8) -> bool {
        return Self::is_half_carry(a, b, r);
    }

    // 根据a+b+carry的计算结果r，判断加法计算过程中，第7位是否发生进位
    // carry表示进位，0或1
    #[inline(always)]
    fn is_carry(a: u8, b: u8, carry: u8) -> bool {
        // 如果a+b大于8为数的最大值0xff，则认为第7位发生进位
        return u16::from(a) + u16::from(b) + u16::from(carry) > 0xff;
    }

    // 根据或a-b-borrow的计算结果r，判断减法计算过程中，第7位是否发生借位
    // borrow表示借位，0或1
    #[inline(always)]
    fn is_borrow(a: u8, b: u8, borrow: u8) -> bool {
        return u16::from(a) < u16::from(b) + u16::from(borrow);
    }

    // 取出8位立即数
    fn imm(&mut self) -> u8 {
        let v = self.get_mem(self.reg.pc);
        self.reg.pc += 1;
        v
    }

    // 取出16位立即数
    fn imm_word(&mut self) -> u16 {
        let v = self.get_mem_word(self.reg.pc);
        self.reg.pc += 2;
        v
    }

    // 将16位数据放入栈顶
    fn stack_push(&mut self, v: u16) {
        // 将栈指针向上移动
        self.reg.sp -= 2;
        self.set_mem_word(self.reg.sp, v);
    }

    // 弹出栈顶的16位数据
    fn stack_pop(&mut self) -> u16 {
        let v = self.get_mem_word(self.reg.sp);
        // 将栈指针乡下移动
        self.reg.sp += 2;
        v
    }

    // 处理指令 ADD A,v
    // 将给定的8位数v加上寄存器A中的值，再把结果写回寄存器A
    // 修改标志位：
    // Z - 计算结果为0, 则置1
    // N - 置0
    // H - 第3位进位时, 则置1
    // C - 第7位进位时, 则置1
    fn add(&mut self, v: u8) {
        // 寄存器A中的原生数值
        let o = self.reg.a;
        // 计算结果
        let r = o.wrapping_add(v);

        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, Self::is_half_carry(o, v, r));
        self.reg.set_flag(Flag::C, Self::is_carry(o, v, 0));

        // 将计算结果写回寄存器A
        self.reg.a = r;
    }

    // 处理指令ADC A,v  也就是带进位的加法
    // 将给定8位数v加上寄存器A中的值，再加上标志位C代表的进位（0或1），再把结果写回寄存器A
    // 修改标志位：
    // Z - 计算结果为0, 则置1
    // N - 置0
    // H - 第3位进位时, 则置1
    // C - 第7位进位时, 则置1
    fn adc(&mut self, v: u8) {
        // 寄存器A中原始的值
        let o = self.reg.a;
        // 是否有进位
        let c = u8::from(self.reg.get_flag(Flag::C));
        // 计算结果
        let r = o.wrapping_add(v).wrapping_add(c);

        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, Self::is_half_carry(o, v, r));
        self.reg.set_flag(Flag::C, Self::is_carry(o, v, c));

        // 将计算结果写回寄存器A
        self.reg.a = r
    }

    // 处理指令SUB A,v
    // 将寄存器A中的值减去给定的8位数，再把结果写回寄存器A
    // 修改标志位：
    // Z - 计算结果为0, 则置1
    // N - 置1
    // H - 第4位借位时, 则置1
    // C - 没有发生借位, 则置1
    fn sub(&mut self, v: u8) {
        // 寄存器A中的原始数值
        let o = self.reg.a;
        // 计算结果
        let r = o.wrapping_sub(v);
        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, true);
        self.reg.set_flag(Flag::H, Self::is_half_borrow(o, v, r));
        // TODO: 文档描述的是没有发生借位，将C置1，实际中确实发生借位将C置1
        self.reg.set_flag(Flag::C, Self::is_borrow(o, v, 0));

        // 将计算结果写回寄存器A
        self.reg.a = r;
    }

    // 处理指令SBC A,v  也就是带借位的算数减法运算
    // 将寄存器A中的值减去给定的8位数v，再减去进位（0或1），最后把结果写回寄存器A
    // 修改标志位：
    // Z - 计算结果为0, 则置1
    // N - 置1
    // H - 第4位借位时, 则置1
    // C - 没有发生借位, 则置1
    fn sbc(&mut self, v: u8) {
        // 寄存器A中原始数值
        let o = self.reg.a;
        // 进位，0或1
        let c = u8::from(self.reg.get_flag(Flag::C));
        // 计算结果
        let r = o.wrapping_sub(v).wrapping_sub(c);

        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, true);
        self.reg.set_flag(Flag::H, Self::is_half_borrow(o, v, r));
        self.reg.set_flag(Flag::C, Self::is_borrow(o, v, c));

        // 将计算结果写回寄存器A
        self.reg.a = r;
    }

    // 处理指令AND A,v  逻辑和运算
    // 修改标志位
    // Z - 计算结果为0, 则置1
    // N - 置0
    // H - 置1
    // C - 置0
    fn and(&mut self, v: u8) {
        self.reg.a &= v;

        // 设置标志位
        self.reg.set_flag(Flag::Z, self.reg.a == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, true);
        self.reg.set_flag(Flag::C, false);
    }

    // 处理指令OR A,v  逻辑或运算
    // 修改标志位
    // Z - 计算结果为0, 则置1
    // N - 置0
    // H - 置0
    // C - 置0
    fn or(&mut self, v: u8) {
        self.reg.a |= v;

        // 设置标志位
        self.reg.set_flag(Flag::Z, self.reg.a == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, false);
    }

    // 处理指令XOR A,v  逻辑异或运算
    // 修改标志位
    // Z - 计算结果为0, 则置1
    // N - 置0
    // H - 置0
    // C - 置0
    fn xor(&mut self, v: u8) {
        self.reg.a ^= v;

        // 设置标志位
        self.reg.set_flag(Flag::Z, self.reg.a == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, false);
    }

    // 处理指令CP A,v  将寄存器A与8位数v进行比较，效果等同于A-v但是丢弃计算结果，只关注是否发生借位
    // 修改标志位
    // Z - 计算结果为0, 则置1
    // N - 置1
    // H - 当计算结果的第4位发生借位时，置1
    // C - 没有发生借位时，置1
    fn cp(&mut self, v: u8) {
        let a = self.reg.a;
        self.sub(v);
        self.reg.a = a;
    }

    // 处理指令INC r8 将寄存器中的值自增
    // 修改标志位
    // Z - 计算结果为0，则置1
    // N - 置0
    // H - 当第3位进位时，置1
    // C - 保持不变
    fn inc(&mut self, v: u8) -> u8 {
        let r = v.wrapping_add(1);

        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, Self::is_half_carry(v, 1, r));
        r
    }

    // 处理指令DEC r8  将8位寄存器中的值自减
    // 修改标志位
    // Z - 计算结果为0，则置1
    // N - 置1
    // H - 当第4位借位时，置1
    // C - 保持不变
    fn dec(&mut self, v: u8) -> u8 {
        let r = v.wrapping_sub(1);

        // 设置标志位
        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, true);
        self.reg.set_flag(Flag::H, Self::is_half_borrow(v, 1, r));
        r
    }

    // ADD HL,v  将寄存器HL中的值和16位数据v相加，在把结果写回寄存器HL
    // 修改标志位
    // Z - 保持不变
    // N - 置0
    // H - 当第11位进位时，置1
    // C - 当第15位进位时，置1
    fn add_hl(&mut self, v: u16) {
        let o = self.reg.get_hl();
        let r = o.wrapping_add(v);

        // 设置标志位
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, (o ^ v ^ r) & 0x1000 > 0);
        self.reg.set_flag(Flag::C, o > 0xffff - v);

        // 将计算结果写回寄存器
        self.reg.set_hl(r);
    }


    // 将寄存器SP中的值与8位立即数（以有符号8位数表示）相加，返回计算结果
    // 修改标志位
    // Z - 置0
    // N - 置0
    // H - 当第4位进位时，置1
    // C - 当第7位进位时，置1
    fn add_sp(&mut self) -> u16 {
        let o = self.reg.sp;
        // 将取到的立即数转为无符号16位数
        let d = i16::from(self.imm() as i8) as u16;
        let r = o.wrapping_add(d);

        // 设置标志位寄存器
        // Z和N位置0
        self.reg.set_flag(Flag::Z, false);
        self.reg.set_flag(Flag::N, false);
        let r_noc = o ^ d ^ r;
        // 设置H位，当SP+d8的第3位进位时置1
        self.reg.set_flag(Flag::H, r_noc & 0x0010 > 0);
        // 设置C位，当SP+d8的第7位进位时置1
        self.reg.set_flag(Flag::C, r_noc & 0x0100 > 0);

        r
    }

    // 处理指令DAA  调整上一步执行的算数加法或算数减法指令所产生的结果（保存在寄存器A中），使其符合BCD编码
    // 这里提到的上一步执行的算数加法或算数减法指令，其操作数都符合BCD编码，也就是：0x0 <= 操作数 <= 0x9
    // 修改标志位
    // Z - 当计算结果为0，置0
    // N - 保持不变
    // H - 置0
    // C - 当高4位表示的十进制数发生进位，置1
    fn daa(&mut self) {
        // 这里以加法为例，假设上一步计算了：15+15，其BCM表示的二进制表示为：0001 0101 + 0001 0101
        // 执行二进制加法后的结果为：0010 1010，低4位是0xa，无法用来表示一位十进制数，我们需要修正它
        // 四位二进制数是满16进1，而一位十进制数是满10进1，他们之间相差6，如果我们想让后四位二进制数进位，需要在额外加上0x06
        // 开始修正后四位：0010 1010 + 0000 0110 = 0011 0000，刚好符合十进制数30的BCM编码
        // 有两种情况我们需要考虑修正BCM编码（修正方法就是让4位二进制数+0x6）：
        // 1. 发生进位（低4位满16，向高4位进1），让其加上0x6，填补其向高位多进的'6'
        // 2. 4位二进制数 > 0x9（最大的1位十进制数），通过让其加上0x6迫使向高位进1，剩下的值<0x9，符合BCM编码
        //
        // 减法的情况类似，如果发生借位，则低4位会向高4位借来16，我们要减去多借来的'6'，让其符合BCM编码
        // 减法不需要考虑4位二进制数>0x9的情况，因为两个BCM编码的数相减，其结果不可能大于9
        let mut a = self.reg.a;
        // 要修正的值，先检测高4位是否发生进位，如果有则需要修正
        let mut adjust = if self.reg.get_flag(Flag::C) { 0x60 } else { 0x00 };
        if self.reg.get_flag(Flag::H) {
            // 低4位发生进位，需要修正
            adjust |= 0x06;
        };
        if !self.reg.get_flag(Flag::N) {
            // 上一条执行的是算数加法运算
            if a & 0x0f > 0x09 {
                // 低4位超出0x9，需要修正
                adjust |= 0x06;
            };
            if a > 0x99 {
                // 高4位超出0x9，需要修正
                // 0x99是高4位即将进位的临界条件，如果a的低4位大于9，那么修正后必定会向高4位进位，此时如果a的高4位是9，则需要发生进位
                adjust |= 0x60;
            };
            // 修正上一步算数加法的结果
            a = a.wrapping_add(adjust);
        } else {
            // 修正上一步算数减法的结果
            a = a.wrapping_sub(adjust);
        }
        // 如果修正后高4位发生进位/借位，修正值至少是0x60
        self.reg.set_flag(Flag::C, adjust >= 0x60);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::Z, a == 0x00);

        // 将修正后的值写回寄存器
        self.reg.a = a;
    }

    // 处理指令CPL  对寄存器A中的值取反
    // 设置标志位
    // Z - 保持不变
    // N - 置1
    // H - 置1
    // C - 保持不变
    fn cpl(&mut self) {
        self.reg.a = !self.reg.a;
        self.reg.set_flag(Flag::N, true);
        self.reg.set_flag(Flag::H, true);
    }

    // 处理指令CCF  修改标志位
    // Z - 保持不变
    // N - 置0
    // H - 置0
    // C - 取反
    fn ccf(&mut self) {
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, !self.reg.get_flag(Flag::C));
    }

    // 处理指令SCF  修改标志位
    // Z - 保持不变
    // N - 置0
    // H - 置0
    // C - 置1
    fn scf(&mut self) {
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, true);
    }

    // 处理指令RLC  将寄存器中的值循环左移1位，也就是左移1位，移出的最高位补充到最低位，同时把其保存到溢出标志位C
    // 设置标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始值的最高位
    fn rlc(&mut self, v: u8) -> u8 {
        // 取出最高位
        let c = v >> 7;
        // 循环左移1位后的结果
        let r = (v << 1) | c;

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        // 将原始值的最高位保存到标志位C
        self.reg.set_flag(Flag::C, c == 0x01);

        r
    }

    // 处理指令RL  将寄存器中的值左移1位，把溢出标志位C补充到最低位，再将被移走的最高位保存到溢出标志位
    // 设置标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始值的最高位
    fn rl(&mut self, v: u8) -> u8 {
        // 取出溢出标志位的值：0x01或0x00
        let c = u8::from(self.reg.get_flag(Flag::C));
        // a左移1位，将溢出标志位的值补充到最低位
        let r = (v << 1) | c;

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        // 将原始值的最高位保存到标志位C
        self.reg.set_flag(Flag::C, v >> 7 == 0x01);

        r
    }

    // 处理指令RRC  将寄存器中的值循环右移1位，也就是右移1位，移出的最低位补充到最高位，同时把其保存到溢出标志位C
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始值的最低位
    fn rrc(&mut self, v: u8) -> u8 {
        // 取出最低位
        let c = v & 0x01;
        // a右移1位，将原最低位补充到最高位
        let r = (v >> 1) | (c << 7);

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        // 将原始值的最低位保存到标志位C
        self.reg.set_flag(Flag::C, c == 0x01);

        r
    }

    // 处理指令RR  将寄存器中的值右移1位，把溢出标志位C补充到最高位，再将被移走的最低位保存到溢出标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始值的最低位
    fn rr(&mut self, v: u8) -> u8 {
        // 溢出标志位中的值：0x01或0x00
        let c = u8::from(self.reg.get_flag(Flag::C));
        let r = (v >> 1) | (c << 7);

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        // 将原始值的最低位保存到标志位C
        self.reg.set_flag(Flag::C, v & 0x01 == 0x01);

        r
    }

    // 处理条件跳转指令，当满足条件con后，跳转到立即数指定的内存地址
    fn jp_if(&mut self, con: bool) {
        let addr = self.imm_word();
        if con {
            self.reg.pc = addr;
        }
    }

    // 处理跳转指令JR  读取一个8位有符号立即数，跳转到pc+n的位置
    fn jr(&mut self, n: u8) {
        self.reg.pc = (u32::from(self.reg.pc) as i32 + i32::from(n as i8)) as u16;
    }

    // 处理条件跳转指令JR IF  如满足条件con，读取一个8位有符号立即数，跳转到pc+n的位置
    fn jr_if(&mut self, con: bool) {
        let n = self.imm();
        if con {
            self.jr(n);
        }
    }

    // 处理指令CALL  将下一条指令的地址压入栈，并跳转到由16位立即数指定的地址
    fn call(&mut self, addr: u16) {
        self.stack_push(self.reg.pc);
        self.reg.pc = addr;
    }

    // 处理指令CALL IF  待条件的CALL指令，当条件con满足时才发生跳转
    fn call_if(&mut self, con: bool) {
        let addr = self.imm_word();
        if con {
            self.call(addr);
        }
    }

    // 处理RST指令  将当前地址压入栈，并跳转到新指定的地址
    fn rst(&mut self, a: u16) {
        self.stack_push(self.reg.pc);
        self.reg.pc = a;
    }

    // 处理RET指令  从栈中弹出一个16位地址，并跳转到该地址
    fn ret(&mut self) {
        self.reg.pc = self.stack_pop();
    }

    // 带条件的RET指令
    fn ret_if(&mut self, con: bool) {
        if con {
            self.ret();
        }
    }

    // 处理指令SLA r8  目标寄存器左移一位，最高位移至溢出标志位C，最低位设置为0
    // 修改标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始数据最高位
    fn sla(&mut self, v: u8) -> u8 {
        let c = v >> 7 == 0x01;
        let r = v << 1;

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        // 将移出的最高位保存到溢出标志位C
        self.reg.set_flag(Flag::C, c);

        r
    }

    // 处理指令SRA r8  目标寄存器右移1位，把最低位移动到溢出标志位C，最高位保持不变
    // 修改标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始数据最低位
    fn sra(&mut self, v: u8) -> u8 {
        let c = v & 0x01 == 0x01;
        // 将原始值右移1位，同时保持最高位不变
        let r = (v >> 1) | (v & 0x80);

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, c);

        r
    }

    // 处理指令SRL r8  目标寄存器右移1位，最低位移动到溢出标志位C，最高位置0
    // 修改标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 原始数据最低位
    fn srl(&mut self, v: u8) -> u8 {
        let c = v & 0x01 == 0x01;
        let r = v >> 1;

        self.reg.set_flag(Flag::Z, r == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, c);

        r
    }

    // 处理指令SWAP r8  交换目标寄存器的高4位和低4位
    // 修改标志位
    // Z - 当计算结果为0，置1
    // N - 置0
    // H - 置0
    // C - 置0
    fn swap(&mut self, v: u8) -> u8 {
        self.reg.set_flag(Flag::Z, v == 0x00);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, false);
        self.reg.set_flag(Flag::C, false);

        // 交换传入8位的数高4位和低4位
        (v >> 4) | (v << 4)
    }

    // 处理指令BIT  取指定寄存器中指定的bit位
    // 修改标志位
    // Z - 当指定的bit位为0，置1
    // N - 置0
    // H - 置1
    // C - 保持不变
    fn bit(&mut self, r: u8, p: u8) {
        // 指定的bit位是否为0
        let is_z = r & (1 << p) == 0x00;

        self.reg.set_flag(Flag::Z, is_z);
        self.reg.set_flag(Flag::N, false);
        self.reg.set_flag(Flag::H, true);
    }

    // 处理指令RES  置0目标寄存器中指定的bit位
    fn res(&mut self, v: u8, p: u8) -> u8 {
        v & !(1 << p)
    }

    // 处理指令SET  置1目标寄存器中指定的bit位
    fn set(&mut self, v: u8, p: u8) -> u8 {
        v | (1 << p)
    }
}

impl Cpu {
    pub fn power_up(term: Term, mem: Rc<RefCell<dyn Memory>>) -> Self {
        Self {
            reg: Register::power_up(term),
            mem,
            halted: false,
            ei: true,
        }
    }

    // 处理中断，返回处理中断消耗的机器周期
    fn hi(&mut self) -> u32 {
        if !self.halted && !self.ei {
            // 当cup正在运行且不允许处理中断
            return 0;
        }
        // 获取IF寄存器的值
        // IF寄存器中低5位记录是否发生以下中断：
        // VBlank (第0位为1则表示发生此中断)
        // LCDStat (第1位为1则表示发生此中断)
        // Timer (第2位为1则表示发生此中断)
        // Serial (第3为1则表示发生此中断)
        // Joypad (第4为1则表示发生此中断)
        let intf = self.get_mem(0xff0f);
        // 获取IE寄存器中断值
        // IE寄存器的低5位表示是否允许处理以下中断:
        // VBlank (第0位为1则表示允许处理)
        // LCDStat (第1位为1则表示允许处理)
        // Timer (第2位为1则表示发生允许处理)
        // Serial (第3为1则表示发生允许处理)
        // Joypad (第4为1则表示发生允许处理)
        let inte = self.get_mem(0xffff);
        // 计算是否有能处理的中断，如果有多个中断，优先处理最低位的中断
        let ii = inte & intf;
        if ii == 0x00 {
            // 没有能处理的中断
            return 0;
        }

        // 唤起cpu
        self.halted = false;
        if !self.ei {
            // 不允许中断
            return 0;
        }
        // 决定处理中断后，将ei置为false
        self.ei = false;

        let n = intf.trailing_zeros();
        // 将IF寄存器中处理过的中断置0
        self.set_mem(0xff0f, intf & !(1 << n));

        // 保存当前将要执行的指令，以便处理完中断后继续回来执行
        self.stack_push(self.reg.pc);
        // 将pc设置为相应的中断处理程序地址
        // V-Blank: 0x40
        // LCDStat: 0x48
        // Timer: 0x50
        // Serial: 0x58
        // Joypad: 0x60
        self.reg.pc = 0x0040 | (n as u16) << 3;
        4
    }

    // 执行指令，并返回每次执行指令所花费的机器周期
    fn ex(&mut self) -> u32 {
        // 指令的8位数编码
        let mut opcode = self.imm();
        // 是否是扩展指令
        let mut is_ext = false;
        // 分支跳转指令所消耗的额外机器周期
        let mut extra_cycles = 0;

        match opcode {
            // LD r8, d8，将立即数写入8位寄存器
            0x06 => self.reg.b = self.imm(),
            0x0e => self.reg.c = self.imm(),
            0x16 => self.reg.d = self.imm(),
            0x1e => self.reg.e = self.imm(),
            0x26 => self.reg.h = self.imm(),
            0x2e => self.reg.l = self.imm(),
            0x36 => {
                let v = self.imm();
                self.set_mem(self.reg.get_hl(), v);
            }
            0x3e => self.reg.a = self.imm(),
            // LD r8, r8，将8位寄存器的值写入另一个8位寄存器
            0x40 => {}
            0x41 => self.reg.b = self.reg.c,
            0x42 => self.reg.b = self.reg.d,
            0x43 => self.reg.b = self.reg.e,
            0x44 => self.reg.b = self.reg.h,
            0x45 => self.reg.b = self.reg.l,
            0x46 => self.reg.b = self.get_mem_hl(),
            0x47 => self.reg.b = self.reg.a,
            0x48 => self.reg.c = self.reg.b,
            0x49 => {}
            0x4a => self.reg.c = self.reg.d,
            0x4b => self.reg.c = self.reg.e,
            0x4c => self.reg.c = self.reg.h,
            0x4d => self.reg.c = self.reg.l,
            0x4e => self.reg.c = self.get_mem_hl(),
            0x4f => self.reg.c = self.reg.a,
            0x50 => self.reg.d = self.reg.b,
            0x51 => self.reg.d = self.reg.c,
            0x52 => {}
            0x53 => self.reg.d = self.reg.e,
            0x54 => self.reg.d = self.reg.h,
            0x55 => self.reg.d = self.reg.l,
            0x56 => self.reg.d = self.get_mem_hl(),
            0x57 => self.reg.d = self.reg.a,
            0x58 => self.reg.e = self.reg.b,
            0x59 => self.reg.e = self.reg.c,
            0x5a => self.reg.e = self.reg.d,
            0x5b => {}
            0x5c => self.reg.e = self.reg.h,
            0x5d => self.reg.e = self.reg.l,
            0x5e => self.reg.e = self.get_mem_hl(),
            0x5f => self.reg.e = self.reg.a,
            0x60 => self.reg.h = self.reg.b,
            0x61 => self.reg.h = self.reg.c,
            0x62 => self.reg.h = self.reg.d,
            0x63 => self.reg.h = self.reg.e,
            0x64 => {}
            0x65 => self.reg.h = self.reg.l,
            0x66 => self.reg.h = self.get_mem_hl(),
            0x67 => self.reg.h = self.reg.a,
            0x68 => self.reg.l = self.reg.b,
            0x69 => self.reg.l = self.reg.c,
            0x6a => self.reg.l = self.reg.d,
            0x6b => self.reg.l = self.reg.e,
            0x6c => self.reg.l = self.reg.h,
            0x6d => {}
            0x6e => self.reg.l = self.get_mem_hl(),
            0x6f => self.reg.l = self.reg.a,
            0x70 => self.set_mem(self.reg.get_hl(), self.reg.b),
            0x71 => self.set_mem(self.reg.get_hl(), self.reg.c),
            0x72 => self.set_mem(self.reg.get_hl(), self.reg.d),
            0x73 => self.set_mem(self.reg.get_hl(), self.reg.e),
            0x74 => self.set_mem(self.reg.get_hl(), self.reg.h),
            0x75 => self.set_mem(self.reg.get_hl(), self.reg.l),
            0x77 => self.set_mem(self.reg.get_hl(), self.reg.a),
            0x78 => self.reg.a = self.reg.b,
            0x79 => self.reg.a = self.reg.c,
            0x7a => self.reg.a = self.reg.d,
            0x7b => self.reg.a = self.reg.e,
            0x7c => self.reg.a = self.reg.h,
            0x7d => self.reg.a = self.reg.l,
            0x7e => self.reg.a = self.get_mem_hl(),
            0x7f => {}
            0x02 => self.set_mem(self.reg.get_bc(), self.reg.a),
            0x12 => self.set_mem(self.reg.get_de(), self.reg.a),
            0x0a => self.reg.a = self.get_mem(self.reg.get_bc()),
            0x1a => self.reg.a = self.get_mem(self.reg.get_de()),
            // LD (C),A  将寄存器A中的值写入寄存器C中保存的地址
            0xe2 => {
                // 低地址保存在寄存器C，高地址全部为1
                let a = 0xff00 | u16::from(self.reg.c);
                self.set_mem(a, self.reg.a);
            }
            // LD A,(C)  将内存地址（寄存器C保存）中的值写入寄存器A
            0xf2 => {
                // 低地址保存在寄存器C，高地址全部为1
                let v = self.get_mem(u16::from(self.reg.c) | 0xff00);
                self.reg.a = v;
            }
            // LD (HL+),A  将寄存器A中的值写入内存地址（由HL寄存器指定），同时HL自增
            0x22 => {
                let a = self.reg.get_hl();
                self.set_mem(a, self.reg.a);
                self.reg.set_hl(a + 1);
            }
            // LD (HL-),A  将寄存器A中的值写入内存地址（由寄存器HL指定），同时HL自减
            0x32 => {
                let a = self.reg.get_hl();
                self.set_mem(a, self.reg.a);
                self.reg.set_hl(a - 1);
            }
            // LD A,(HL+)  将内存地址（由寄存器HL指定）中的值写入寄存器A，同时HL自增
            0x2a => {
                let a = self.reg.get_hl();
                self.reg.a = self.get_mem(a);
                self.reg.set_hl(a + 1);
            }
            // LD A,(HL-)  将内存地址（由寄存器HL指定）中的值写入寄存器A，同时HL自减
            0x3a => {
                let a = self.reg.get_hl();
                self.reg.a = self.get_mem(a);
                self.reg.set_hl(a - 1);
            }
            // LD (d8),A 将寄存器A的值写入内存地址（由立即数指定）
            0xe0 => {
                // 8位立即数保存内存地址的低8位，高8位全部为1
                let a = 0xff00 | u16::from(self.imm());
                self.set_mem(a, self.reg.a);
            }
            // LD A,(a8)  将内存地址（由立即数指定）中的值写入寄存器A
            0xf0 => {
                // 8位立即数保存内存地址的低8位，高8位全部为1
                let a = 0xff00 | u16::from(self.imm());
                self.reg.a = self.get_mem(a);
            }
            // LD (d16),A  将寄存器A中的值写入内存地址（由立即数指定）
            0xea => {
                let a = self.imm_word();
                self.set_mem(a, self.reg.a);
            }
            // LD A,(d16) 将内存地址（由立即数指定）中的值写入寄存器A
            0xfa => {
                let a = self.imm_word();
                self.reg.a = self.get_mem(a);
            }
            // LD r16,d16  将16位立即数写入16位寄存器中
            0x01 => {
                let v = self.imm_word();
                self.reg.set_bc(v);
            }
            0x11 => {
                let v = self.imm_word();
                self.reg.set_de(v);
            }
            0x21 => {
                let v = self.imm_word();
                self.reg.set_hl(v);
            }
            0x31 => self.reg.sp = self.imm_word(),
            // LD SP,HL  将寄存器HL中的值写入寄存器SP
            0xf9 => self.reg.sp = self.reg.get_hl(),
            // LD HL,SP+d8  将SP寄存器中的值加上8位有符号立即数，将结果写入寄存器HL
            0xf8 => {
                let v = self.add_sp();
                self.reg.set_hl(v);
            }

            // LD (d16),SP  将SP寄存器中的值写入内存（由立即数指定）
            0x08 => {
                let a = self.imm_word();
                self.set_mem_word(a, self.reg.sp);
            }

            // PUSH 将16位寄存器的值入栈
            0xc5 => self.stack_push(self.reg.get_bc()),
            0xd5 => self.stack_push(self.reg.get_de()),
            0xe5 => self.stack_push(self.reg.get_hl()),
            0xf5 => self.stack_push(self.reg.get_af()),

            // POP 出栈并将数据写入16位寄存器
            0xc1 => {
                let v = self.stack_pop();
                self.reg.set_bc(v);
            }
            0xd1 => {
                let v = self.stack_pop();
                self.reg.set_de(v);
            }
            0xe1 => {
                let v = self.stack_pop();
                self.reg.set_hl(v);
            }
            0xf1 => {
                let v = self.stack_pop();
                self.reg.set_af(v);
            }

            // ADD A, r8/d8  算数加法运算，将指定的8位数和寄存器A中的数相加，再写回寄存器A
            0x80 => self.add(self.reg.b),
            0x81 => self.add(self.reg.c),
            0x82 => self.add(self.reg.d),
            0x83 => self.add(self.reg.e),
            0x84 => self.add(self.reg.h),
            0x85 => self.add(self.reg.l),
            0x86 => self.add(self.get_mem_hl()),
            0x87 => self.add(self.reg.a),
            0xc6 => {
                let v = self.imm();
                self.add(v);
            }

            // ADC A,r8/d8  带进位的算数加法运算
            0x88 => self.adc(self.reg.b),
            0x89 => self.adc(self.reg.c),
            0x8a => self.adc(self.reg.d),
            0x8b => self.adc(self.reg.e),
            0x8c => self.adc(self.reg.h),
            0x8d => self.adc(self.reg.l),
            0x8e => self.adc(self.get_mem_hl()),
            0x8f => self.adc(self.reg.a),
            0xce => {
                let v = self.imm();
                self.adc(v);
            }

            // SUB A,r8/d8  算数减法运算
            0x90 => self.sub(self.reg.b),
            0x91 => self.sub(self.reg.c),
            0x92 => self.sub(self.reg.d),
            0x93 => self.sub(self.reg.e),
            0x94 => self.sub(self.reg.h),
            0x95 => self.sub(self.reg.l),
            0x96 => self.sub(self.get_mem_hl()),
            0x97 => self.sub(self.reg.a),
            0xd6 => {
                let v = self.imm();
                self.sub(v);
            }

            //SBC A,r8/d8
            0x98 => self.sbc(self.reg.b),
            0x99 => self.sbc(self.reg.c),
            0x9a => self.sbc(self.reg.d),
            0x9b => self.sbc(self.reg.e),
            0x9c => self.sbc(self.reg.h),
            0x9d => self.sbc(self.reg.l),
            0x9e => self.sbc(self.get_mem_hl()),
            0x9f => self.sbc(self.reg.a),
            0xde => {
                let v = self.imm();
                self.sbc(v);
            }

            // AND A,r8/d8
            0xa0 => self.and(self.reg.b),
            0xa1 => self.and(self.reg.c),
            0xa2 => self.and(self.reg.d),
            0xa3 => self.and(self.reg.e),
            0xa4 => self.and(self.reg.h),
            0xa5 => self.and(self.reg.l),
            0xa6 => self.and(self.get_mem_hl()),
            0xa7 => self.and(self.reg.a),
            0xe6 => {
                let v = self.imm();
                self.and(v);
            }

            // OR A,r8/d8
            0xb0 => self.or(self.reg.b),
            0xb1 => self.or(self.reg.c),
            0xb2 => self.or(self.reg.d),
            0xb3 => self.or(self.reg.e),
            0xb4 => self.or(self.reg.h),
            0xb5 => self.or(self.reg.l),
            0xb6 => self.or(self.get_mem_hl()),
            0xb7 => self.or(self.reg.a),
            0xf6 => {
                let v = self.imm();
                self.or(v);
            }

            // XOR A,r8/d8
            0xa8 => self.xor(self.reg.b),
            0xa9 => self.xor(self.reg.c),
            0xaa => self.xor(self.reg.d),
            0xab => self.xor(self.reg.e),
            0xac => self.xor(self.reg.h),
            0xad => self.xor(self.reg.l),
            0xae => self.xor(self.get_mem_hl()),
            0xaf => self.xor(self.reg.a),
            0xee => {
                let v = self.imm();
                self.xor(v);
            }

            // CP A,r8/d8
            0xb8 => self.cp(self.reg.b),
            0xb9 => self.cp(self.reg.c),
            0xba => self.cp(self.reg.d),
            0xbb => self.cp(self.reg.e),
            0xbc => self.cp(self.reg.h),
            0xbd => self.cp(self.reg.l),
            0xbe => self.cp(self.get_mem_hl()),
            0xbf => self.cp(self.reg.a),
            0xfe => {
                let v = self.imm();
                self.cp(v);
            }

            // INC r8
            0x04 => self.reg.b = self.inc(self.reg.b),
            0x0c => self.reg.c = self.inc(self.reg.c),
            0x14 => self.reg.d = self.inc(self.reg.d),
            0x1c => self.reg.e = self.inc(self.reg.e),
            0x24 => self.reg.h = self.inc(self.reg.h),
            0x2c => self.reg.l = self.inc(self.reg.l),
            0x34 => {
                let v = self.inc(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x3c => self.reg.a = self.inc(self.reg.a),

            // DEC r8
            0x05 => self.reg.b = self.dec(self.reg.b),
            0x0d => self.reg.c = self.dec(self.reg.c),
            0x15 => self.reg.d = self.dec(self.reg.d),
            0x1d => self.reg.e = self.dec(self.reg.e),
            0x25 => self.reg.h = self.dec(self.reg.h),
            0x2d => self.reg.l = self.dec(self.reg.l),
            0x35 => {
                let v = self.dec(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x3d => self.reg.a = self.dec(self.reg.a),

            // ADD HL,r16
            0x09 => self.add_hl(self.reg.get_bc()),
            0x19 => self.add_hl(self.reg.get_de()),
            0x29 => self.add_hl(self.reg.get_hl()),
            0x39 => self.add_hl(self.reg.sp),

            // ADD SP,d8  将寄存器SP中的值加上8位立即数（有符号），再把计算结果写回寄存器SP
            0xe8 => self.reg.sp = self.add_sp(),

            // INC r16  将16位寄存器自增
            0x03 => self.reg.set_bc(self.reg.get_bc().wrapping_add(1)),
            0x13 => self.reg.set_de(self.reg.get_de().wrapping_add(1)),
            0x23 => self.reg.set_hl(self.reg.get_hl().wrapping_add(1)),
            0x33 => self.reg.sp = self.reg.sp.wrapping_add(1),

            // DEC r16  将16位寄存器自减
            0x0b => self.reg.set_bc(self.reg.get_bc().wrapping_sub(1)),
            0x1b => self.reg.set_de(self.reg.get_de().wrapping_sub(1)),
            0x2b => self.reg.set_hl(self.reg.get_hl().wrapping_sub(1)),
            0x3b => self.reg.sp = self.reg.sp.wrapping_sub(1),

            // DAA
            0x27 => self.daa(),

            // CPL
            0x2f => self.cpl(),
            // CCF
            0x3f => self.ccf(),
            // SCF
            0x37 => self.scf(),

            // NOP  不做操作
            0x00 => {}
            // HALT  关闭CPU，直到发生新的中断事件，竟可能使用此指令来降低能耗
            0x76 => self.halted = true,
            // STOP  按下按钮前暂停CPU和LCD显示，模拟器实现不用做特殊处理
            0x10 => {}

            // DI  禁用中断，但不是立即禁用，在下一条指令执行时禁用
            0xf3 => self.ei = false,
            // EI  启用中断，但不是立即启用，在下一条指令执行时启用
            0xfb => self.ei = true,

            // RLCA
            0x07 => {
                self.reg.a = self.rlc(self.reg.a);
                self.reg.set_flag(Flag::Z, false);
            }
            // RLA
            0x17 => {
                self.reg.a = self.rl(self.reg.a);
                self.reg.set_flag(Flag::Z, false);
            }
            // RRCA
            0x0f => {
                self.reg.a = self.rrc(self.reg.a);
                self.reg.set_flag(Flag::Z, false);
            }
            // RRA
            0x1f => {
                self.reg.a = self.rr(self.reg.a);
                // TODO: 为什么强制将标志位Z置0
                self.reg.set_flag(Flag::Z, false);
            }

            // JUMP  无条件跳转
            0xc3 => self.reg.pc = self.imm_word(),
            0xe9 => self.reg.pc = self.reg.get_hl(),
            // JUMP IF  有条件跳转
            0xc2 => {
                self.jp_if(!self.reg.get_flag(Flag::Z));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0xca => {
                self.jp_if(self.reg.get_flag(Flag::Z));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0xd2 => {
                self.jp_if(!self.reg.get_flag(Flag::C));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0xda => {
                self.jp_if(self.reg.get_flag(Flag::C));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            // JR
            0x18 => {
                let n = self.imm();
                self.jr(n);
            }
            0x20 => {
                self.jr_if(!self.reg.get_flag(Flag::Z));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0x28 => {
                self.jr_if(self.reg.get_flag(Flag::Z));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0x30 => {
                self.jr_if(!self.reg.get_flag(Flag::C));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }
            0x38 => {
                self.jr_if(self.reg.get_flag(Flag::C));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 1 };
            }

            // CALL
            0xcd => {
                let addr = self.imm_word();
                self.call(addr);
            }
            // CALL IF
            0xc4 => {
                self.call_if(!self.reg.get_flag(Flag::Z));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xcc => {
                self.call_if(self.reg.get_flag(Flag::Z));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xd4 => {
                self.call_if(!self.reg.get_flag(Flag::C));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xdc => {
                self.call_if(self.reg.get_flag(Flag::C));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }

            // RST
            0xc7 => self.rst(0x00),
            0xcf => self.rst(0x08),
            0xd7 => self.rst(0x10),
            0xdf => self.rst(0x18),
            0xe7 => self.rst(0x20),
            0xef => self.rst(0x28),
            0xf7 => self.rst(0x30),
            0xff => self.rst(0x38),

            // RET
            0xc9 => self.ret(),
            // RET IF
            0xc0 => {
                self.ret_if(!self.reg.get_flag(Flag::Z));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xc8 => {
                self.ret_if(self.reg.get_flag(Flag::Z));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xd0 => {
                self.ret_if(!self.reg.get_flag(Flag::C));
                if !self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            0xd8 => {
                self.ret_if(self.reg.get_flag(Flag::C));
                if self.reg.get_flag(Flag::Z) { extra_cycles = 3 };
            }
            // RETI  执行RET指令并启用中断
            0xd9 => {
                self.ret();
                self.ei = true;
            }

            // 执行扩展指令，由两个字节组成，第一个字节的值固定为0xcb
            0xcb => {
                is_ext = true;
                opcode = self.imm();
                self.ex_ext(opcode);
            }

            _ => {}
        };
        let cycles = if is_ext {
            // 返回执行扩展指令所需的机器周期
            EXT_OP_CYCLES[opcode as usize]
        } else {
            // 返回执行基础指令所需的机器周期（如果是带判断条件的指令，且条件满足，则加上额外花费的机器周期）
            OP_CYCLES[opcode as usize] + extra_cycles
        };
        return cycles;
    }

    // 执行扩展指令
    #[allow(unreachable_patterns)]
    fn ex_ext(&mut self, opcode: u8) {
        match opcode {
            // RLC r8
            0x00 => self.reg.b = self.rlc(self.reg.b),
            0x01 => self.reg.c = self.rlc(self.reg.c),
            0x02 => self.reg.d = self.rlc(self.reg.d),
            0x03 => self.reg.e = self.rlc(self.reg.e),
            0x04 => self.reg.h = self.rlc(self.reg.h),
            0x05 => self.reg.l = self.rlc(self.reg.l),
            0x06 => {
                let v = self.rlc(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x07 => self.reg.a = self.rlc(self.reg.a),

            // RRC r8
            0x08 => self.reg.b = self.rrc(self.reg.b),
            0x09 => self.reg.c = self.rrc(self.reg.c),
            0x0a => self.reg.d = self.rrc(self.reg.d),
            0x0b => self.reg.e = self.rrc(self.reg.e),
            0x0c => self.reg.h = self.rrc(self.reg.h),
            0x0d => self.reg.l = self.rrc(self.reg.l),
            0x0e => {
                let v = self.rrc(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x0f => self.reg.a = self.rrc(self.reg.a),

            // RL r8
            0x10 => self.reg.b = self.rl(self.reg.b),
            0x11 => self.reg.c = self.rl(self.reg.c),
            0x12 => self.reg.d = self.rl(self.reg.d),
            0x13 => self.reg.e = self.rl(self.reg.e),
            0x14 => self.reg.h = self.rl(self.reg.h),
            0x15 => self.reg.l = self.rl(self.reg.l),
            0x16 => {
                let v = self.rl(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x17 => self.reg.a = self.rl(self.reg.a),

            // RR r8
            0x18 => self.reg.b = self.rr(self.reg.b),
            0x19 => self.reg.c = self.rr(self.reg.c),
            0x1a => self.reg.d = self.rr(self.reg.d),
            0x1b => self.reg.e = self.rr(self.reg.e),
            0x1c => self.reg.h = self.rr(self.reg.h),
            0x1d => self.reg.l = self.rr(self.reg.l),
            0x1e => {
                let v = self.rr(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x1f => self.reg.a = self.rr(self.reg.a),

            // SLA r8
            0x20 => self.reg.b = self.sla(self.reg.b),
            0x21 => self.reg.c = self.sla(self.reg.c),
            0x22 => self.reg.d = self.sla(self.reg.d),
            0x23 => self.reg.e = self.sla(self.reg.e),
            0x24 => self.reg.h = self.sla(self.reg.h),
            0x25 => self.reg.l = self.sla(self.reg.l),
            0x26 => {
                let v = self.sla(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x27 => self.reg.a = self.sla(self.reg.a),

            // SRA r8
            0x28 => self.reg.b = self.sra(self.reg.b),
            0x29 => self.reg.c = self.sra(self.reg.c),
            0x2a => self.reg.d = self.sra(self.reg.d),
            0x2b => self.reg.e = self.sra(self.reg.e),
            0x2c => self.reg.h = self.sra(self.reg.h),
            0x2d => self.reg.l = self.sra(self.reg.l),
            0x2e => {
                let v = self.sra(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x2f => self.reg.a = self.sra(self.reg.a),

            // SRL r8
            0x38 => self.reg.b = self.srl(self.reg.b),
            0x39 => self.reg.c = self.srl(self.reg.c),
            0x3a => self.reg.d = self.srl(self.reg.d),
            0x3b => self.reg.e = self.srl(self.reg.e),
            0x3c => self.reg.h = self.srl(self.reg.h),
            0x3d => self.reg.l = self.srl(self.reg.l),
            0x3e => {
                let v = self.srl(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x3f => self.reg.a = self.srl(self.reg.a),

            // SWAP r8
            0x30 => self.reg.b = self.swap(self.reg.b),
            0x31 => self.reg.c = self.swap(self.reg.c),
            0x32 => self.reg.d = self.swap(self.reg.d),
            0x33 => self.reg.e = self.swap(self.reg.e),
            0x34 => self.reg.h = self.swap(self.reg.h),
            0x35 => self.reg.l = self.swap(self.reg.l),
            0x36 => {
                let v = self.swap(self.get_mem_hl());
                self.set_mem(self.reg.get_hl(), v);
            }
            0x37 => self.reg.a = self.swap(self.reg.a),

            // BIT
            0x40 => self.bit(self.reg.b, 0),
            0x41 => self.bit(self.reg.c, 0),
            0x42 => self.bit(self.reg.d, 0),
            0x43 => self.bit(self.reg.e, 0),
            0x44 => self.bit(self.reg.h, 0),
            0x45 => self.bit(self.reg.l, 0),
            0x46 => self.bit(self.get_mem_hl(), 0),
            0x47 => self.bit(self.reg.a, 0),
            0x48 => self.bit(self.reg.b, 1),
            0x49 => self.bit(self.reg.c, 1),
            0x4a => self.bit(self.reg.d, 1),
            0x4b => self.bit(self.reg.e, 1),
            0x4c => self.bit(self.reg.h, 1),
            0x4d => self.bit(self.reg.l, 1),
            0x4e => self.bit(self.get_mem_hl(), 1),
            0x4f => self.bit(self.reg.a, 1),
            0x50 => self.bit(self.reg.b, 2),
            0x51 => self.bit(self.reg.c, 2),
            0x52 => self.bit(self.reg.d, 2),
            0x53 => self.bit(self.reg.e, 2),
            0x54 => self.bit(self.reg.h, 2),
            0x55 => self.bit(self.reg.l, 2),
            0x56 => self.bit(self.get_mem_hl(), 2),
            0x57 => self.bit(self.reg.a, 2),
            0x58 => self.bit(self.reg.b, 3),
            0x59 => self.bit(self.reg.c, 3),
            0x5a => self.bit(self.reg.d, 3),
            0x5b => self.bit(self.reg.e, 3),
            0x5c => self.bit(self.reg.h, 3),
            0x5d => self.bit(self.reg.l, 3),
            0x5e => self.bit(self.get_mem_hl(), 3),
            0x5f => self.bit(self.reg.a, 3),
            0x60 => self.bit(self.reg.b, 4),
            0x61 => self.bit(self.reg.c, 4),
            0x62 => self.bit(self.reg.d, 4),
            0x63 => self.bit(self.reg.e, 4),
            0x64 => self.bit(self.reg.h, 4),
            0x65 => self.bit(self.reg.l, 4),
            0x66 => self.bit(self.get_mem_hl(), 4),
            0x67 => self.bit(self.reg.a, 4),
            0x68 => self.bit(self.reg.b, 5),
            0x69 => self.bit(self.reg.c, 5),
            0x6a => self.bit(self.reg.d, 5),
            0x6b => self.bit(self.reg.e, 5),
            0x6c => self.bit(self.reg.h, 5),
            0x6d => self.bit(self.reg.l, 5),
            0x6e => self.bit(self.get_mem_hl(), 5),
            0x6f => self.bit(self.reg.a, 5),
            0x70 => self.bit(self.reg.b, 6),
            0x71 => self.bit(self.reg.c, 6),
            0x72 => self.bit(self.reg.d, 6),
            0x73 => self.bit(self.reg.e, 6),
            0x74 => self.bit(self.reg.h, 6),
            0x75 => self.bit(self.reg.l, 6),
            0x76 => self.bit(self.get_mem_hl(), 6),
            0x77 => self.bit(self.reg.a, 6),
            0x78 => self.bit(self.reg.b, 7),
            0x79 => self.bit(self.reg.c, 7),
            0x7a => self.bit(self.reg.d, 7),
            0x7b => self.bit(self.reg.e, 7),
            0x7c => self.bit(self.reg.h, 7),
            0x7d => self.bit(self.reg.l, 7),
            0x7e => self.bit(self.get_mem_hl(), 7),
            0x7f => self.bit(self.reg.a, 7),

            // RES
            0x80 => self.reg.b = self.res(self.reg.b, 0),
            0x81 => self.reg.c = self.res(self.reg.c, 0),
            0x82 => self.reg.d = self.res(self.reg.d, 0),
            0x83 => self.reg.e = self.res(self.reg.e, 0),
            0x84 => self.reg.h = self.res(self.reg.h, 0),
            0x85 => self.reg.l = self.res(self.reg.l, 0),
            0x86 => {
                let v = self.res(self.get_mem_hl(), 0);
                self.set_mem(self.reg.get_hl(), v);
            }
            0x87 => self.reg.a = self.res(self.reg.a, 0),
            0x88 => self.reg.b = self.res(self.reg.b, 1),
            0x89 => self.reg.c = self.res(self.reg.c, 1),
            0x8a => self.reg.d = self.res(self.reg.d, 1),
            0x8b => self.reg.e = self.res(self.reg.e, 1),
            0x8c => self.reg.h = self.res(self.reg.h, 1),
            0x8d => self.reg.l = self.res(self.reg.l, 1),
            0x8e => {
                let v = self.res(self.get_mem_hl(), 1);
                self.set_mem(self.reg.get_hl(), v);
            }
            0x8f => self.reg.a = self.res(self.reg.a, 1),
            0x90 => self.reg.b = self.res(self.reg.b, 2),
            0x91 => self.reg.c = self.res(self.reg.c, 2),
            0x92 => self.reg.d = self.res(self.reg.d, 2),
            0x93 => self.reg.e = self.res(self.reg.e, 2),
            0x94 => self.reg.h = self.res(self.reg.h, 2),
            0x95 => self.reg.l = self.res(self.reg.l, 2),
            0x96 => {
                let v = self.res(self.get_mem_hl(), 2);
                self.set_mem(self.reg.get_hl(), v);
            }
            0x97 => self.reg.a = self.res(self.reg.a, 2),
            0x98 => self.reg.b = self.res(self.reg.b, 3),
            0x99 => self.reg.c = self.res(self.reg.c, 3),
            0x9a => self.reg.d = self.res(self.reg.d, 3),
            0x9b => self.reg.e = self.res(self.reg.e, 3),
            0x9c => self.reg.h = self.res(self.reg.h, 3),
            0x9d => self.reg.l = self.res(self.reg.l, 3),
            0x9e => {
                let v = self.res(self.get_mem_hl(), 3);
                self.set_mem(self.reg.get_hl(), v);
            }
            0x9f => self.reg.a = self.res(self.reg.a, 3),
            0xa0 => self.reg.b = self.res(self.reg.b, 4),
            0xa1 => self.reg.c = self.res(self.reg.c, 4),
            0xa2 => self.reg.d = self.res(self.reg.d, 4),
            0xa3 => self.reg.e = self.res(self.reg.e, 4),
            0xa4 => self.reg.h = self.res(self.reg.h, 4),
            0xa5 => self.reg.l = self.res(self.reg.l, 4),
            0xa6 => {
                let v = self.res(self.get_mem_hl(), 4);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xa7 => self.reg.a = self.res(self.reg.a, 4),
            0xa8 => self.reg.b = self.res(self.reg.b, 5),
            0xa9 => self.reg.c = self.res(self.reg.c, 5),
            0xaa => self.reg.d = self.res(self.reg.d, 5),
            0xab => self.reg.e = self.res(self.reg.e, 5),
            0xac => self.reg.h = self.res(self.reg.h, 5),
            0xad => self.reg.l = self.res(self.reg.l, 5),
            0xae => {
                let v = self.res(self.get_mem_hl(), 5);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xaf => self.reg.a = self.res(self.reg.a, 5),
            0xb0 => self.reg.b = self.res(self.reg.b, 6),
            0xb1 => self.reg.c = self.res(self.reg.c, 6),
            0xb2 => self.reg.d = self.res(self.reg.d, 6),
            0xb3 => self.reg.e = self.res(self.reg.e, 6),
            0xb4 => self.reg.h = self.res(self.reg.h, 6),
            0xb5 => self.reg.l = self.res(self.reg.l, 6),
            0xb6 => {
                let v = self.res(self.get_mem_hl(), 6);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xb7 => self.reg.a = self.res(self.reg.a, 6),
            0xb8 => self.reg.b = self.res(self.reg.b, 7),
            0xb9 => self.reg.c = self.res(self.reg.c, 7),
            0xba => self.reg.d = self.res(self.reg.d, 7),
            0xbb => self.reg.e = self.res(self.reg.e, 7),
            0xbc => self.reg.h = self.res(self.reg.h, 7),
            0xbd => self.reg.l = self.res(self.reg.l, 7),
            0xbe => {
                let v = self.res(self.get_mem_hl(), 7);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xbf => self.reg.a = self.res(self.reg.a, 7),

            // SET
            0xc0 => self.reg.b = self.set(self.reg.b, 0),
            0xc1 => self.reg.c = self.set(self.reg.c, 0),
            0xc2 => self.reg.d = self.set(self.reg.d, 0),
            0xc3 => self.reg.e = self.set(self.reg.e, 0),
            0xc4 => self.reg.h = self.set(self.reg.h, 0),
            0xc5 => self.reg.l = self.set(self.reg.l, 0),
            0xc6 => {
                let v = self.set(self.get_mem_hl(), 0);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xc7 => self.reg.a = self.set(self.reg.a, 0),
            0xc8 => self.reg.b = self.set(self.reg.b, 1),
            0xc9 => self.reg.c = self.set(self.reg.c, 1),
            0xca => self.reg.d = self.set(self.reg.d, 1),
            0xcb => self.reg.e = self.set(self.reg.e, 1),
            0xcc => self.reg.h = self.set(self.reg.h, 1),
            0xcd => self.reg.l = self.set(self.reg.l, 1),
            0xce => {
                let v = self.set(self.get_mem_hl(), 1);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xcf => self.reg.a = self.set(self.reg.a, 1),
            0xd0 => self.reg.b = self.set(self.reg.b, 2),
            0xd1 => self.reg.c = self.set(self.reg.c, 2),
            0xd2 => self.reg.d = self.set(self.reg.d, 2),
            0xd3 => self.reg.e = self.set(self.reg.e, 2),
            0xd4 => self.reg.h = self.set(self.reg.h, 2),
            0xd5 => self.reg.l = self.set(self.reg.l, 2),
            0xd6 => {
                let v = self.set(self.get_mem_hl(), 2);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xd7 => self.reg.a = self.set(self.reg.a, 2),
            0xd8 => self.reg.b = self.set(self.reg.b, 3),
            0xd9 => self.reg.c = self.set(self.reg.c, 3),
            0xda => self.reg.d = self.set(self.reg.d, 3),
            0xdb => self.reg.e = self.set(self.reg.e, 3),
            0xdc => self.reg.h = self.set(self.reg.h, 3),
            0xdd => self.reg.l = self.set(self.reg.l, 3),
            0xde => {
                let v = self.set(self.get_mem_hl(), 3);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xdf => self.reg.a = self.set(self.reg.a, 3),
            0xe0 => self.reg.b = self.set(self.reg.b, 4),
            0xe1 => self.reg.c = self.set(self.reg.c, 4),
            0xe2 => self.reg.d = self.set(self.reg.d, 4),
            0xe3 => self.reg.e = self.set(self.reg.e, 4),
            0xe4 => self.reg.h = self.set(self.reg.h, 4),
            0xe5 => self.reg.l = self.set(self.reg.l, 4),
            0xe6 => {
                let v = self.set(self.get_mem_hl(), 4);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xe7 => self.reg.a = self.set(self.reg.a, 4),
            0xe8 => self.reg.b = self.set(self.reg.b, 5),
            0xe9 => self.reg.c = self.set(self.reg.c, 5),
            0xea => self.reg.d = self.set(self.reg.d, 5),
            0xeb => self.reg.e = self.set(self.reg.e, 5),
            0xec => self.reg.h = self.set(self.reg.h, 5),
            0xed => self.reg.l = self.set(self.reg.l, 5),
            0xee => {
                let v = self.set(self.get_mem_hl(), 5);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xef => self.reg.a = self.set(self.reg.a, 5),
            0xf0 => self.reg.b = self.set(self.reg.b, 6),
            0xf1 => self.reg.c = self.set(self.reg.c, 6),
            0xf2 => self.reg.d = self.set(self.reg.d, 6),
            0xf3 => self.reg.e = self.set(self.reg.e, 6),
            0xf4 => self.reg.h = self.set(self.reg.h, 6),
            0xf5 => self.reg.l = self.set(self.reg.l, 6),
            0xf6 => {
                let v = self.set(self.get_mem_hl(), 6);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xf7 => self.reg.a = self.set(self.reg.a, 6),
            0xf8 => self.reg.b = self.set(self.reg.b, 7),
            0xf9 => self.reg.c = self.set(self.reg.c, 7),
            0xfa => self.reg.d = self.set(self.reg.d, 7),
            0xfb => self.reg.e = self.set(self.reg.e, 7),
            0xfc => self.reg.h = self.set(self.reg.h, 7),
            0xfd => self.reg.l = self.set(self.reg.l, 7),
            0xfe => {
                let v = self.set(self.get_mem_hl(), 7);
                self.set_mem(self.reg.get_hl(), v);
            }
            0xff => self.reg.a = self.set(self.reg.a, 7),
            _ => unreachable!(),
        };
    }

    // 执行指令，并返回每次执行指令所花费的时钟周期
    pub fn next(&mut self) -> u32 {
        let mac = {
            let c = self.hi();
            if c != 0 {
                // 处理中断
                c
            } else if self.halted {
                OP_CYCLES[0]
            } else {
                // 执行下一条指令
                self.ex()
            }
        };
        // 1机器周期=4时钟周期
        mac * 4
    }
}