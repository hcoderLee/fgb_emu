use std::cell::RefCell;
use std::cmp::min;
use std::rc::Rc;
use blip_buf::BlipBuf;
use crate::core::apu::Channel::{Mixer, Noise, Square1, Square2, Wave};
use crate::core::convention::CPU_FREQ;
use crate::core::memory::Memory;
use std::sync::{Arc, Mutex};
use crate::core::clock::Clock;
use crate::core::motherboard::MotherBoard;

#[derive(Clone, Eq, PartialEq)]
enum Channel {
    // 有扫频和包络的方波
    Square1,
    // 有包络的方波
    Square2,
    // 可编程的波形
    Wave,
    // 白噪音
    Noise,
    Mixer,
}

/// 每种音频通道有5个寄存器来控制，nr0～nr5
///
///        Square1
/// NR10 FF10 -PPP NSSS; FF10: NR10寄存器的地址, -: 没有使用, S: Sweep period, N: negate, S: shift
/// NR11 FF11 DDLL LLLL Duty, Length load (64-L)
/// NR12 FF12 VVVV APPP Starting volume, Envelope add mode, period
/// NR13 FF13 FFFF FFFF Frequency LSB
/// NR14 FF14 TL-- -FFF Trigger, Length enable, Frequency MSB
///
///        Square 2
///      FF15 ---- ---- Not used
/// NR21 FF16 DDLL LLLL Duty, Length load (64-L)
/// NR22 FF17 VVVV APPP Starting volume, Envelope add mode, period
/// NR23 FF18 FFFF FFFF Frequency LSB
/// NR24 FF19 TL-- -FFF Trigger, Length enable, Frequency MSB
///
///        Wave
/// NR30 FF1A E--- ---- DAC power
/// NR31 FF1B LLLL LLLL Length load (256-L)
/// NR32 FF1C -VV- ---- Volume code (00=0%, 01=100%, 10=50%, 11=25%)
/// NR33 FF1D FFFF FFFF Frequency LSB
/// NR34 FF1E TL-- -FFF Trigger, Length enable, Frequency MSB
///
///        Noise
///      FF1F ---- ---- Not used
/// NR41 FF20 --LL LLLL Length load (64-L)
/// NR42 FF21 VVVV APPP Starting volume, Envelope add mode, period
/// NR43 FF22 SSSS WDDD Clock shift, Width mode of LFSR, Divisor code
/// NR44 FF23 TL-- ---- Trigger, Length enable
///
///        Control/Status
/// NR50 FF24 ALLL BRRR Vin L enable, Left vol, Vin R enable, Right vol
/// NR51 FF25 NW21 NW21 Left enables, Right enables
/// NR52 FF26 P--- NW21 Power control/status, Channel length statuses
///
///        Not used
///      FF27 ---- ----
///      .... ---- ----
///      FF2F ---- ----
///
///        Wave Table
///      FF30 0000 1111 Samples 0 and 1
///      ....
///      FF3F 0000 1111 Samples 30 and 31
struct Register {
    channel: Channel,
    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,
}

impl Register {
    fn power_up(channel: Channel) -> Self {
        Self {
            channel,
            nrx0: 0x00,
            nrx1: 0x00,
            nrx2: 0x00,
            nrx3: 0x00,
            nrx4: 0x00,
        }
    }

    /// 扫频器时钟周期
    fn get_sweep_period(&self) -> u8 {
        // sweep period是Square1通道独有的
        assert!(self.channel == Square1);
        // 取nr0寄存器的4～6位
        (self.nrx0 >> 4) & 0x07
    }

    fn get_negate(&self) -> bool {
        // negate是Square1通道独有的
        assert!(self.channel == Square1);
        // 判断nr0寄存器的第3位是否不为0
        self.nrx0 & 0x08 != 0
    }

    /// 扫频器的移位值，用于计算新频率
    fn get_shift(&self) -> u8 {
        // shift是Square1通道独有的
        assert!(self.channel == Square1);
        // 取nr0的低3位
        self.nrx0 & 0x07
    }

    /// 返回DAC是否可用 (DAC是数模转换器，自定义波形通道需要使用DAC)
    fn get_dac_power(&self) -> bool {
        // DAC power是Wave通道独有的
        assert!(self.channel == Wave);
        // 判断nr0的最高位是否不为0
        self.nrx0 & 0x80 != 0
    }

    fn get_duty(&self) -> u8 {
        // duty是Square1和Square2独有的
        assert!(self.channel == Square1 || self.channel == Square2);
        // 取nr1的高2位
        self.nrx1 >> 6
    }


    fn get_length_load(&self) -> u16 {
        if self.channel == Wave {
            (1 << 8) - u16::from(self.nrx1)
        } else {
            (1 << 6) - u16::from(self.nrx1 & 0x3f)
        }
    }

    fn get_starting_volume(&self) -> u8 {
        assert!(self.channel != Wave);
        self.nrx2 >> 4
    }

    fn get_volume_code(&self) -> u8 {
        assert!(self.channel == Wave);
        (self.nrx2 >> 5) & 0x03
    }

    fn get_envelope_add_mode(&self) -> bool {
        assert!(self.channel != Wave);
        self.nrx2 & 0x08 != 0
    }

    /// 音量包络时钟周期
    fn get_period(&self) -> u8 {
        assert!(self.channel != Wave);
        self.nrx2 & 0x07
    }

    /// 获取当前通道的频率
    fn get_frequency(&self) -> u16 {
        assert!(self.channel != Noise);
        u16::from(self.nrx4 & 0x07) << 8 | u16::from(self.nrx3)
    }

    /// 设置当前通道的频率
    fn set_frequency(&mut self, v: u16) {
        assert!(self.channel != Noise);
        self.nrx3 = v as u8;
        let h = ((v >> 8) & 0x07) as u8;
        self.nrx4 = (self.nrx4 & 0xf8) | h;
    }

    fn get_clock_shift(&self) -> u8 {
        assert!(self.channel == Noise);
        self.nrx3 >> 4
    }

    fn get_lfsr_width_mode(&self) -> bool {
        assert!(self.channel == Noise);
        self.nrx3 & 0x08 != 0
    }

    fn get_dividor_code(&self) -> u8 {
        assert!(self.channel == Noise);
        self.nrx3 & 0x07
    }

    fn get_trigger(&self) -> bool {
        self.nrx4 & 0x80 != 0
    }

    fn set_trigger(&mut self, is_trigger: bool) {
        if is_trigger {
            self.nrx4 |= 0x80;
        } else {
            self.nrx4 &= 0x7f;
        }
    }

    fn get_length_enable(&self) -> bool {
        self.nrx4 & 0x40 != 0
    }

    /// 左声道主音量
    fn get_l_vol(&self) -> u8 {
        assert!(self.channel == Mixer);
        (self.nrx0 >> 4) & 0x07
    }

    /// 右声道主音量
    fn get_r_vol(&self) -> u8 {
        assert!(self.channel == Mixer);
        self.nrx0 & 0x07
    }

    /// 音频是否可用
    fn get_power(&self) -> bool {
        assert!(self.channel == Mixer);
        self.nrx2 & 0x80 != 0x00
    }

    fn reset(&mut self) {
        self.nrx0 = 0x00;
        self.nrx1 = 0x00;
        self.nrx2 = 0x00;
        self.nrx3 = 0x00;
        self.nrx4 = 0x00;
    }
}

/// 音频序列发生器，由512HZ的音频时钟控制
struct FrameSequencer {
    /// 已8为周期，控制长度计数器，音量包络，扫频器的触发时机
    /// 长度计数器(Length Ctr)在step为0，2，4，6时触发
    /// 音量包络(Vol Env)在step为7时触发
    /// 扫频器(Sweep)在step为2，6时触发
    step: u8,
}

impl FrameSequencer {
    fn power_up() -> Self {
        Self { step: 0x00 }
    }

    fn next(&mut self) -> u8 {
        self.step += 1;
        self.step %= 8;
        self.step
    }
}

/// 长度计数器，每次被序列发生器触发时，计数减1，直到计数为0, 当计数位0时将关闭音频通道，也就是将trigger设置成false
struct LengthCounter {
    register: Rc<RefCell<Register>>,
    // 计数
    n: u16,
}

impl LengthCounter {
    fn power_up(register: Rc<RefCell<Register>>) -> Self {
        Self {
            register,
            n: 0,
        }
    }

    /// 每次被序列发生器触发时执行，nr4寄存器最高位被写入1时也会触发次方法
    fn next(&mut self) {
        if !self.register.borrow().get_length_enable() || self.n == 0 {
            return;
        }

        // 计数自减1
        self.n -= 1;
        if self.n == 0 {
            // 计数已减为0，将设置trigger设置成false
            self.register.borrow_mut().set_trigger(false);
        }
    }

    /// 重置计数
    fn reload(&mut self) {
        if self.n != 0 { return; }

        // 将计数恢复到默认值，也就是Length load(保存在寄存器nr1中)的长度
        self.n = if self.register.borrow().channel == Wave {
            // Wave channel的Length load是8位
            1 << 8
        } else {
            // 其他channel的Length load是6位
            1 << 6
        }
    }
}

/// 音量包络，会自动修改当前通道的音量
struct VolumeEnvelope {
    register: Rc<RefCell<Register>>,
    /// 内部计时器
    timer: Clock,
    /// 最终输出的音量
    volume: u8,
}

impl VolumeEnvelope {
    fn power_up(register: Rc<RefCell<Register>>) -> Self {
        Self {
            register,
            timer: Clock::power_up(8),
            volume: 0x00,
        }
    }

    fn reload(&mut self) {
        // 获取nr2寄存器中设置的音量包络计时器周期
        let p = self.register.borrow().get_period();
        // 如果寄存器设置的周期为0则使用默认周期8
        self.timer.period = if p == 0 { 8 } else { u32::from(p) };
        self.volume = self.register.borrow().get_starting_volume();
    }

    fn next(&mut self) {
        if self.register.borrow().get_period() == 0 {
            return;
        }

        // 只在内部时钟的周期末才会执行音量增益操作
        if self.timer.next(1) == 0 {
            return;
        }

        // 计算音量增益后的值
        let v = if self.register.borrow().get_envelope_add_mode() {
            self.volume.wrapping_add(1)
        } else {
            self.volume.wrapping_sub(1)
        };
        if v <= 15 {
            // 只有增益后的音量在0到15之间才更新音量
            self.volume = v;
        }
    }
}

/// 扫频器, 会自动修改当前通道的频率
struct FrequencySweep {
    register: Rc<RefCell<Register>>,
    /// 内部时钟
    timer: Clock,
    /// 是否启用
    enable: bool,
    /// shadow寄存器，保存当前通道的频率
    shadow: u16,
    /// 新频率
    new_freq: u16,
}

impl FrequencySweep {
    fn power_up(register: Rc<RefCell<Register>>) -> Self {
        Self {
            register,
            timer: Clock::power_up(8),
            enable: false,
            shadow: 0x0000,
            new_freq: 0x0000,
        }
    }

    /// 重置频率扫描器
    fn reload(&mut self) {
        // 从寄存器中获取通道的频率
        self.shadow = self.register.borrow().get_frequency();
        // 获取nr2寄存器中设置的扫频器计时器周期
        let p = self.register.borrow().get_sweep_period();
        // 如果寄存器设置的周期为0则使用默认周期8
        self.timer.period = if p == 0 { 8 } else { u32::from(p) };
        let shift = self.register.borrow().get_shift();
        self.enable = p != 0 || shift != 0;
        if shift != 0x00 {
            // 扫描移位不为0，执行频率计算，溢出检查
            self.frequency_calculation();
            self.overflow_check();
        }
    }

    /// 计算新频率
    fn frequency_calculation(&mut self) {
        let offset = self.shadow >> self.register.borrow().get_shift();
        if self.register.borrow().get_negate() {
            self.new_freq = self.shadow.wrapping_sub(offset);
        } else {
            self.new_freq = self.shadow.wrapping_add(offset);
        }
    }

    /// 溢出检查
    fn overflow_check(&mut self) {
        if self.new_freq >= 2048 {
            // 新频率大于2047，禁用Square1通道
            self.register.borrow_mut().set_trigger(false);
        }
    }

    fn next(&mut self) {
        if !self.enable || self.register.borrow().get_sweep_period() == 0 {
            return;
        }

        // 只在内部时钟每个周期末尾执行之后的操作
        if self.timer.next(1) == 0 {
            return;
        }

        self.frequency_calculation();
        self.overflow_check();
        if self.new_freq < 2048 && self.register.borrow().get_shift() != 0 {
            // 将计算出的新频率写入shadow寄存器和当前通道的nr3，nr4寄存器
            self.shadow = self.new_freq;
            self.register.borrow_mut().set_frequency(self.new_freq);
            // 再次执行计算新频率和溢出检查
            self.frequency_calculation();
            self.overflow_check();
        }
    }
}

// 线性反馈移位寄存器 (Linear feedback shift register)，用于生成伪随机数
struct LFSR {
    register: Rc<RefCell<Register>>,
    seed: u16,
}

impl LFSR {
    fn power_up(register: Rc<RefCell<Register>>) -> Self {
        Self {
            register,
            seed: 0x0001,
        }
    }

    fn next(&mut self) -> bool {
        let origin = self.seed;
        // 右移一位
        self.seed >>= 1;
        // 低2位的异或结果
        let xor = (self.seed ^ origin) & 0x0001;
        // 最高位(此时为0)设置位低2位低异或值
        self.seed |= xor << 15;
        if self.register.borrow().get_lfsr_width_mode() {
            // 如果with mode为1， 则将bit 6也设置为低2位异或值
            self.seed = (self.seed & 0xffbf) | (xor << 6);
        }
        // 返回最低位取反
        return self.seed & 0x0001 == 1;
    }

    fn reload(&mut self) {
        self.seed = 0x0001;
    }
}

/// 方波通道
struct ChannelSquare {
    register: Rc<RefCell<Register>>,
    /// 时钟
    timer: Clock,
    /// 长度计数器
    lc: LengthCounter,
    ///  音量包络
    ve: VolumeEnvelope,
    /// 扫频器
    fs: FrequencySweep,
    /// 音量数据
    blip: Blip,
    /// 当前处理的波形编号，在0-7内，每次时钟周期内自增
    wave_idx: u8,
}

impl ChannelSquare {
    fn power_up(blip: BlipBuf, mode: Channel) -> Self {
        let register = Rc::new(RefCell::new(Register::power_up(mode.clone())));
        Self {
            register: register.clone(),
            timer: Clock::power_up(8192),
            lc: LengthCounter::power_up(register.clone()),
            ve: VolumeEnvelope::power_up(register.clone()),
            fs: FrequencySweep::power_up(register.clone()),
            blip: Blip::power_up(blip),
            wave_idx: 1,
        }
    }

    fn next(&mut self, cycles: u32) {
        // 根据duty选取波形，波形是8个bit，每个时钟周期内处理1个bit
        let waveform = match self.register.borrow().get_duty() {
            0 => 0b0000_0001,
            1 => 0b1000_0001,
            2 => 0b1000_0111,
            3 => 0b0111_1110,
            _ => unreachable!(),
        };
        let vol = i32::from(self.ve.volume);
        for _ in 0..self.timer.next(cycles) {
            // 计算振幅
            let ampl = if !self.register.borrow().get_trigger() || self.ve.volume == 0 {
                0x00
            } else if (waveform >> self.wave_idx) & 0x01 != 0 {
                // 当前波形编号对应的波形数据是1
                vol
            } else {
                // 当前波形编号对应的波形数据是0
                vol * -1
            };
            // 将振幅写入音频数据
            self.blip.set(self.blip.from + self.timer.period, ampl);
            // 波形编号自增
            self.wave_idx = (self.wave_idx + 1) % 8;
        }
    }

    /// 在扫频器触发之后可能会改变当前通道频率, 调用次方法更新当前通道的频率
    fn update_freq(&mut self) {
        self.timer.period = 4 * (2048 - u32::from(self.register.borrow().get_frequency()));
    }
}

impl Memory for ChannelSquare {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff10 | 0xff15 => self.register.borrow().nrx0,
            0xff11 | 0xff16 => self.register.borrow().nrx1,
            0xff12 | 0xff17 => self.register.borrow().nrx2,
            0xff13 | 0xff18 => self.register.borrow().nrx3,
            0xff14 | 0xff19 => self.register.borrow().nrx4,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff10 | 0xff15 => self.register.borrow_mut().nrx0 = v,
            0xff11 | 0xff16 => {
                self.register.borrow_mut().nrx1 = v;
                self.lc.n = self.register.borrow().get_length_load();
            }
            0xff12 | 0xff17 => self.register.borrow_mut().nrx2 = v,
            0xff13 | 0xff18 => {
                self.register.borrow_mut().nrx3 = v;
                // 修改了方波通道的频率，重置时钟
                self.update_freq();
            }
            0xff14 | 0xff19 => {
                self.register.borrow_mut().nrx4 = v;
                // 修改了方波通道的频率，重置时钟
                self.update_freq();

                if self.register.borrow().get_trigger() {
                    self.lc.reload();
                    self.ve.reload();
                    if self.register.borrow().channel == Square1 {
                        self.fs.reload();
                    }
                }
            }
            _ => unreachable!(),
        };
    }
}

/// 自定义波形通道
struct ChannelWave {
    register: Rc<RefCell<Register>>,
    timer: Clock,
    lc: LengthCounter,
    /// 保存最终生成的音频数据
    blip: Blip,
    /// 采样数据表，每个采样数据占用4位，wave table一共保存32个采样
    wave_table: [u8; 16],
    /// 当前采样的编号，在0～31之间，每次采样后递增
    sample_idx: usize,
}

impl ChannelWave {
    fn power_up(blip: BlipBuf) -> Self {
        let register = Rc::new(RefCell::new(Register::power_up(Wave)));
        Self {
            register: register.clone(),
            timer: Clock::power_up(8192),
            lc: LengthCounter::power_up(register.clone()),
            blip: Blip::power_up(blip),
            wave_table: [0x00; 16],
            sample_idx: 0,
        }
    }

    fn next(&mut self, cycles: u32) {
        let register = self.register.borrow();
        // 根据volume code计算出采样数据的右移值
        let shift = match register.get_volume_code() {
            0 => 4,
            1 => 0,
            2 => 1,
            3 => 2,
            _ => unreachable!(),
        };
        for _ in 0..self.timer.next(cycles) {
            // 当前采样在采样数据表中的下标
            let table_idx = self.sample_idx / 2;
            // 是否取采样字节数据的低4位
            let is_lower = self.sample_idx & 0x01 == 0;
            // 进行一次采样, 计算采样值
            let sample = if is_lower {
                // 取低4位
                self.wave_table[table_idx] & 0x0f
            } else {
                // 取高4位
                self.wave_table[table_idx] >> 4
            };
            let ampl = if !register.get_trigger() || !register.get_dac_power() {
                // 当前通道不可用
                0x00
            } else {
                // 将采样数据右移计算出最终的振幅
                i32::from(sample >> shift)
            };
            // 将计算的振幅写入音频数据
            self.blip.set(self.blip.from + self.timer.period, ampl);
            // 更新采样编号
            self.sample_idx = (self.sample_idx + 1) % 32;
        }
    }

    /// 在扫频器触发之后可能会改变当前通道频率, 调用次方法更新当前通道的频率
    fn update_freq(&mut self) {
        self.timer.period = 2 * (2048 - u32::from(self.register.borrow().get_frequency()));
    }
}

impl Memory for ChannelWave {
    fn get(&self, a: u16) -> u8 {
        let register = self.register.borrow();
        match a {
            0xff1a => register.nrx0,
            0xff1b => register.nrx1,
            0xff1c => register.nrx2,
            0xff1d => register.nrx3,
            0xff1e => register.nrx4,
            0xff30..=0xff3f => self.wave_table[a as usize - 0xff30],
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        // let mut register = self.register.borrow_mut();
        match a {
            0xff1a => self.register.borrow_mut().nrx0 = v,
            0xff1b => {
                self.register.borrow_mut().nrx1 = v;
                self.lc.n = self.register.borrow().get_length_load();
            }
            0xff1c => self.register.borrow_mut().nrx2 = v,
            0xff1d => {
                self.register.borrow_mut().nrx3 = v;
                // 修改了自定义波形通道的频率，重置时钟
                self.update_freq();
            }
            0xff1e => {
                self.register.borrow_mut().nrx4 = v;
                // 修改了自定义波形通道的频率，重置时钟
                self.update_freq();
                if self.register.borrow().get_trigger() {
                    self.lc.reload();
                    self.sample_idx = 0;
                }
            }
            0xff30..=0xff3f => self.wave_table[a as usize - 0xff30] = v,
            _ => unreachable!(),
        }
    }
}

/// 噪声通道
struct ChannelNoise {
    register: Rc<RefCell<Register>>,
    timer: Clock,
    lc: LengthCounter,
    ve: VolumeEnvelope,
    /// 线性反馈移位寄存器，用于生成随机数
    lfsr: LFSR,
    /// 保存音频数据
    blip: Blip,
}

impl ChannelNoise {
    fn power_up(blip: BlipBuf) -> Self {
        let register = Rc::new(RefCell::new(Register::power_up(Noise)));
        Self {
            register: register.clone(),
            timer: Clock::power_up(4096),
            lc: LengthCounter::power_up(register.clone()),
            ve: VolumeEnvelope::power_up(register.clone()),
            lfsr: LFSR::power_up(register.clone()),
            blip: Blip::power_up(blip),
        }
    }

    fn next(&mut self, cycles: u32) {
        for _ in 0..self.timer.next(cycles) {
            // 通道当前音量
            let volume = self.ve.volume;
            // 计算振幅
            let ampl = if !self.register.borrow().get_trigger() || volume == 0 {
                0x00
            } else if self.lfsr.next() {
                i32::from(volume)
            } else {
                i32::from(volume) * -1
            };
            // 写入音频数据
            self.blip.set(self.blip.from + self.timer.period, ampl);
        }
    }

    /// 修改Noise通道的频率
    fn update_freq(&mut self) {
        // 根据divider code和clock shift来计算时钟周期
        let register = self.register.borrow();
        let d = match register.get_dividor_code() {
            0 => 0,
            n => (u32::from(n) + 1) * 16
        };
        self.timer.period = d << register.get_clock_shift();
    }
}

impl Memory for ChannelNoise {
    fn get(&self, a: u16) -> u8 {
        let register = self.register.borrow();
        match a {
            0xff1f => register.nrx0,
            0xff20 => register.nrx1,
            0xff21 => register.nrx2,
            0xff22 => register.nrx3,
            0xff23 => register.nrx4,
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            0xff1f => self.register.borrow_mut().nrx0 = v,
            0xff20 => {
                self.register.borrow_mut().nrx1 = v;
                self.lc.n = self.register.borrow().get_length_load();
            }
            0xff21 => self.register.borrow_mut().nrx2 = v,
            0xff22 => {
                self.register.borrow_mut().nrx3 = v;
                self.update_freq();
            }
            0xff23 => {
                self.register.borrow_mut().nrx4 = v;
                if self.register.borrow().get_trigger() {
                    self.lc.reload();
                    self.ve.reload();
                    self.lfsr.reload();
                }
            }
            _ => unreachable!()
        }
    }
}

pub struct APU {
    register: Register,
    timer: Clock,
    fs: FrameSequencer,
    square1_channel: ChannelSquare,
    square2_channel: ChannelSquare,
    wave_channel: ChannelWave,
    noise_channel: ChannelNoise,
    /// 采样率
    sample_rate: u32,
    /// 最终要播放的音频数据，包含的采样数据不能大于1s
    pub buffer: Arc<Mutex<Vec<(f32, f32)>>>,
}

impl APU {
    pub fn power_up(sample_rate: u32) -> Self {
        let buf1 = create_blipbuf(sample_rate);
        let buf2 = create_blipbuf(sample_rate);
        let buf3 = create_blipbuf(sample_rate);
        let buf4 = create_blipbuf(sample_rate);

        Self {
            register: Register::power_up(Mixer),
            // 设置音频时钟频率为512HZ
            timer: Clock::power_up(CPU_FREQ / 512),
            fs: FrameSequencer::power_up(),
            square1_channel: ChannelSquare::power_up(buf1, Square1),
            square2_channel: ChannelSquare::power_up(buf2, Square2),
            wave_channel: ChannelWave::power_up(buf3),
            noise_channel: ChannelNoise::power_up(buf4),
            sample_rate,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn next(&mut self, cycles: u32) {
        if !self.register.get_power() {
            return;
        }

        for _ in 0..self.timer.next(cycles) {
            // 开始一帧采样，让各个音频通道写入音频数据
            self.square1_channel.next(self.timer.period);
            self.square2_channel.next(self.timer.period);
            self.wave_channel.next(self.timer.period);
            self.noise_channel.next(self.timer.period);

            let step = self.fs.next();
            if step == 0 || step == 2 || step == 4 {
                // 触发长度控制器
                self.square1_channel.lc.next();
                self.square2_channel.lc.next();
                self.wave_channel.lc.next();
                self.noise_channel.lc.next();
            }

            if step == 7 {
                // 触发音量包络
                self.square1_channel.ve.next();
                self.square2_channel.ve.next();
                self.noise_channel.ve.next();
            }

            if step == 2 || step == 6 {
                // 触发扫频器
                self.square1_channel.fs.next();
                // 更新通道频率
                self.square1_channel.update_freq();
            }

            let duration = self.timer.period;
            // 结束本帧采样
            end_frame(duration, &mut self.square1_channel.blip);
            end_frame(duration, &mut self.square2_channel.blip);
            end_frame(duration, &mut self.wave_channel.blip);
            end_frame(duration, &mut self.noise_channel.blip);

            self.mix();
        }
    }

    /// 将所有音频通道的数据混合并播放
    fn mix(&mut self) {
        // 所有的音频通道应该拥有相同的样本数
        let sample_size = self.square1_channel.blip.data.samples_avail();
        assert_eq!(self.square2_channel.blip.data.samples_avail(), sample_size);
        assert_eq!(self.wave_channel.blip.data.samples_avail(), sample_size);
        assert_eq!(self.noise_channel.blip.data.samples_avail(), sample_size);

        let mut sum = 0;
        let factor = (1.0 / 15.0) * 0.25;
        let l_vol = (f32::from(self.register.get_l_vol()) / 7.0) * factor;
        let r_vol = (f32::from(self.register.get_r_vol()) / 7.0) * factor;

        while sum < sample_size {
            // 左声道数据
            let buf_l = &mut [0f32; 2048];
            // 右声道数据
            let buf_r = &mut [0f32; 2048];
            // 从音频通道中读取的数据
            let buf = &mut [0i16; 2048];
            // nr51寄存器，记录各个音频通道的左右声道是否可用
            let nr51 = self.register.nrx1;
            // 左右声道混入当前音频的数据
            let mut mix_data = |count, enable_l, enable_r, buf: &[i16]| {
                for (i, v) in buf[..count].iter().enumerate() {
                    if enable_l {
                        // 左声道混入当前音频通道的数据
                        buf_l[i] += f32::from(*v) * l_vol;
                    }
                    if enable_r {
                        // 右声道混入当前音频通道的数据
                        buf_r[i] += f32::from(*v) * r_vol;
                    }
                }
            };

            // 读取Square1通道的数据
            let s1_count = self.square1_channel.blip.data.read_samples(buf, false);
            // Square1通道左声道是否可用
            let s1_enable_l = nr51 & 0x01 == 0x01;
            // Square1通道右声道是否可用
            let s1_enable_r = nr51 & 0x10 == 0x10;
            // 左右声道混入Square1通道的数据
            mix_data(s1_count, s1_enable_l, s1_enable_r, buf);


            // 读取Square2通道中的数据
            let s2_count = self.square2_channel.blip.data.read_samples(buf, false);
            assert_eq!(s2_count, s1_count);
            // Square2通道左声道是否可用
            let s2_enable_l = nr51 & 0x02 == 0x02;
            // Square2通道右声道是否可用
            let s2_enable_r = nr51 & 0x20 == 0x20;
            // 左右声道混入Square2通道的数据
            mix_data(s2_count, s2_enable_l, s2_enable_r, buf);

            // 读取Wave通道中的数据
            let w_count = self.wave_channel.blip.data.read_samples(buf, false);
            // Wave通道左声道是否可用
            let w_enable_l = nr51 & 0x04 == 0x04;
            // Wave通道右声道是否可用
            let w_enable_r = nr51 & 0x40 == 0x40;
            // 左右声道混入wave通道的数据
            mix_data(w_count, w_enable_l, w_enable_r, buf);

            // 读取Noise通道中的数据
            let n_count = self.noise_channel.blip.data.read_samples(buf, false);
            // Noise通道左声道是否可用
            let n_enable_l = nr51 & 0x04 == 0x04;
            // Noise通道右声道是否可用
            let n_enable_r = nr51 & 0x40 == 0x40;
            // 左右声道混入Noise通道的数据
            mix_data(n_count, n_enable_l, n_enable_r, buf);

            // 写入最终混合好的音频数据
            self.play(buf_l, buf_r);
            sum += s1_count as u32;
        }
    }

    /// 写入最终要播放的音频数据
    fn play(&mut self, l: &[f32], r: &[f32]) {
        assert_eq!(l.len(), r.len());
        let mut buffer = self.buffer.lock().unwrap();
        for (lv, rv) in l.iter().zip(r) {
            if buffer.len() > self.sample_rate as usize {
                // 不能写入大于1s的采样数据
                return;
            }
            buffer.push((*lv, *rv));
        }
    }
}

impl Memory for APU {
    fn get(&self, a: u16) -> u8 {
        match a {
            0xff10..=0xff14 => self.square1_channel.get(a),
            0xff15..=0xff19 => self.square2_channel.get(a),
            0xff1a..=0xff1e => self.wave_channel.get(a),
            0xff1f..=0xff23 => self.noise_channel.get(a),
            0xff24 => self.register.nrx0,
            0xff25 => self.register.nrx1,
            // NR52寄存器 P--- NW21 Power control/status, Channel length statuses
            0xff26 => {
                let upper = self.register.nrx2 & 0xf0;
                let s1_trigger = if self.square1_channel.register.borrow().get_trigger() { 0x01 } else { 0x00 };
                let s2_trigger = if self.square2_channel.register.borrow().get_trigger() { 0x02 } else { 0x00 };
                let w_trigger = if self.wave_channel.register.borrow().get_trigger() { 0x40 } else { 0x00 };
                let n_trigger = if self.noise_channel.register.borrow().get_trigger() { 0x80 } else { 0x00 };
                upper | s1_trigger | s2_trigger | w_trigger | n_trigger
            }
            0xff27..=0xff2f => 0x00,
            0xff30..=0xff3f => self.wave_channel.get(a),
            _ => unreachable!(),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        if a != 0xff26 && !self.register.get_power() {
            return;
        }

        match a {
            0xff10..=0xff14 => self.square1_channel.set(a, v),
            0xff15..=0xff19 => self.square2_channel.set(a, v),
            0xff1a..=0xff1e => self.wave_channel.set(a, v),
            0xff1f..=0xff23 => self.noise_channel.set(a, v),
            0xff24 => self.register.nrx0 = v,
            0xff25 => self.register.nrx1 = v,
            0xff26 => {
                self.register.nrx2 = v;
                // 关闭APU时将所有的nrx寄存器写入0x00，wave通道的波形数据不受影响
                if !self.register.get_power() {
                    self.square1_channel.register.borrow_mut().reset();
                    self.square2_channel.register.borrow_mut().reset();
                    self.wave_channel.register.borrow_mut().reset();
                    self.noise_channel.register.borrow_mut().reset();
                    self.register.reset();
                }
            }
            // 未被使用的内存段
            0xff27..=0xff2f => {}
            // 写入wave通道的波形数据
            0xff30..=0xff3f => self.wave_channel.set(a, v),
            _ => unreachable!(),
        }
    }
}

/// 音频数据，保存的是多个时刻和与之对应的振幅，可以理解为{time: amplitude}键值对
struct Blip {
    data: BlipBuf,
    /// 最后一次改变振幅的时间点
    from: u32,
    /// 当前振幅
    ampl: i32,
}

impl Blip {
    fn power_up(data: BlipBuf) -> Self {
        Self {
            data,
            from: 0x0000_0000,
            ampl: 0x0000_0000,
        }
    }

    /// 设置时间和对应的振幅
    fn set(&mut self, time: u32, ampl: i32) {
        self.from = time;
        let d = ampl - self.ampl;
        self.ampl = ampl;
        self.data.add_delta(time, d);
    }
}

/// 创建一个BlipBuf
fn create_blipbuf(sample_rate: u32) -> BlipBuf {
    // buf最多保存sample_rate个样本
    let mut buf = BlipBuf::new(sample_rate);
    // 以Game boy的CPU时钟频率为基准时间，也就是将一个CPU时钟周期作为采样的单位时间，最终转换生成采样率为sample_rate的数据
    buf.set_rates(f64::from(CPU_FREQ), f64::from(sample_rate));
    return buf;
}

/// 结束一帧的采样
fn end_frame(duration: u32, blip: &mut Blip) {
    blip.data.end_frame(duration);
    blip.from -= duration;
}

#[cfg(feature = "audio")]
pub fn initialize_audio(mbrd: &MotherBoard) {
    use cpal::StreamData;
    // 设置音频播放环境
    let device = cpal::default_output_device().unwrap();
    let sample_rate = device.default_output_format().unwrap().sample_rate;
    let format = cpal::Format {
        channels: 2,
        sample_rate,
        data_type: cpal::SampleFormat::F32,
    };
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    // 设置播放源，外放设备将播放音频流中的数据
    event_loop.play_stream(stream_id);

    let apu = APU::power_up(sample_rate.0);
    // APU生成的音频数据
    let audio_data = apu.buffer.clone();
    mbrd.mmu.borrow_mut().apu = Some(apu);

    std::thread::spawn(move || {
        // 音频流回调函数，负责向音频流里填充音频数据，每当音频流需要新数据时，调用此函数
        let stream_callback = move |_, stream_data: StreamData| {
            // 解封装APU生成的数据
            let mut audio_data = audio_data.lock().unwrap();
            // 解封装音频流内需要填充的数据集合
            if let StreamData::Output { buffer } = stream_data {
                let len = min(buffer.len() / 2, audio_data.len());
                match buffer {
                    cpal::UnknownTypeOutputBuffer::F32(mut buffer) => {
                        // 将APU生成的F32格式的音频数据写入音频流
                        for (i, (l, r)) in audio_data.drain(..len).enumerate() {
                            // 偶数下标写入左声道数据
                            buffer[i * 2] = l;
                            // 奇数下标写入右声道数据
                            buffer[i * 2 + 1] = r;
                        }
                    }
                    cpal::UnknownTypeOutputBuffer::U16(mut buffer) => {
                        // 将F32类型的数转换为U16类型
                        let convert = |v: f32| { (v * f32::from(i16::MAX) + f32::from(u16::MAX) / 2.0) as u16 };
                        // 将APU生成的F32格式的音频数据准换成U16类型再写入音频流
                        for (i, (l, r)) in audio_data.drain(..len).enumerate() {
                            buffer[i * 2] = convert(l);
                            buffer[i * 2 + 1] = convert(r);
                        }
                    }
                    cpal::UnknownTypeOutputBuffer::I16(mut buffer) => {
                        // 将F32类型的数转换为I16类型
                        let convert = |v: f32| { (v * f32::from(i16::MAX)) as i16 };
                        // 将APU生成的F32格式的音频数据转换成I16类型再写入音频流
                        for (i, (l, r)) in audio_data.drain(..len).enumerate() {
                            buffer[i * 2] = convert(l);
                            buffer[i * 2 + 1] = convert(r);
                        }
                    }
                }
            }
        };
        // 设置音频流回调函数
        event_loop.run(stream_callback);
    });
}

