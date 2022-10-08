pub trait Memory {
    fn get(&self, a: u16) -> u8;
    fn set(&mut self, a: u16, v: u8);

    // 获取一个字的内容，也就是16位的数据
    fn get_word(&self, a: u16) -> u16 {
        // 取出a和其下一个地址保存的值，拼接成一个16位的值，这里采用小端序，即低地址在前，高地址在后
        u16::from(self.get(a)) | (u16::from(self.get(a + 1)) << 8)
    }

    // 设置一个字的内容，即连续设置两个相邻地址的数据
    fn set_word(&mut self, a: u16, v: u16) {
        // 按照小端序，高地址取后8位
        self.set(a, (v & 0x00ff) as u8);
        // 低地址取前8位
        self.set(a + 1, (v >> 8) as u8);
    }
}