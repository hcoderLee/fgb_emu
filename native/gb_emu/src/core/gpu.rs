use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::convention::{SCREEN_H, SCREEN_W, Term};
use crate::core::gpu::GPUMode::{HBlank, SearchOAM, Tran2Driver, VBlank};
use crate::core::intf::Intf;
use crate::core::intf::INTFlag;
use crate::core::memory::Memory;

/// GB模式下4种灰度所对应的rgb值
pub enum GrayShades {
    White = 0xff,
    Light = 0xc0,
    Dark = 0x60,
    Blank = 0x00,
}

/// LCD控制寄存器，控制画面中的对象是否显示以及如何显示
pub struct LCDC {
    data: u8,
}

impl LCDC {
    pub fn power_up() -> Self {
        Self { data: 0b0100_1000 }
    }

    /// LCDC寄存器的第7位，控制LCD是否亮起
    fn lcd_enable(&self) -> bool {
        self.data & 0b1000_0000 != 0x00
    }

    /// LCDC寄存器的第6位，用于寻址window tile map
    /// 其值为0时，tile map起始地址为0x9800
    /// 其值为1时，tile map起始地址为0x9c00
    fn win_tm_sel(&self) -> bool {
        self.data & 0b0100_0000 != 0x00
    }

    /// LCDC寄存器第5位，控制是否显示窗口
    fn window_enable(&self) -> bool {
        self.data & 0b0010_0000 != 0x00
    }

    /// LCDC寄存器第4位，控制BG和Window用于寻找tile data时的寻址模式
    /// 其值为0时，tile data寻址范围是：0x8800-0x97ff
    /// 其值为1时，tile data寻址范围是：0x8000-0x8fff
    fn td_sel(&self) -> bool {
        self.data & 0b0001_0000 != 0x00
    }

    /// LCDC寄存器第3位，用于寻址bg tile map
    /// 其值为0时，tile map起始地址为0x9800
    /// 其值为1时，tile map起始地址为0x9c00
    fn bg_tm_sel(&self) -> bool {
        self.data & 0b0000_1000 != 0
    }

    /// LCDC寄存器第2位，控制sprite大小，sprite可以为一个tile或2个竖直排列的tile
    fn obj_h_16(&self) -> bool {
        self.data & 0b0000_0100 != 0
    }

    /// LCDC寄存器第1位，控制是否显示sprite
    fn obj_enable(&self) -> bool {
        self.data & 0b0000_0010 != 0
    }

    /// LCDC寄存器第0位
    /// 在黑白模式下表示是否显示bg和window，当值为0，bg和window为空白，此时可能只有sprites显示
    /// 在彩色模式表示sprite的优先级，当值为0，sprite将始终显示在bg之上
    fn bg_win_pri(&self) -> bool {
        self.data & 0b0000_0001 != 0
    }
}

/// 当前GPU所处的周期
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum GPUMode {
    /// 模式0: 处于从上一行的末尾跳转到下一行开始处的过程
    HBlank = 0,
    /// 模式1: 处于从右下角跳转到左上角的过程
    VBlank = 1,
    /// 模式2: 读取OAM内存区域，获取Sprite的坐标，此时处于水平扫描的过程中
    SearchOAM = 2,
    /// 模式3: 读取卡带中的数据到LCD驱动，此时处于水平扫描的过程中
    Tran2Driver = 3,
}

/// LCD状态寄存器，控制LCD当前的显示状态
pub struct LCDS {
    /// 寄存器第6位，当LYC与LY相等且该位为1时，触发中断
    enable_ly_int: bool,
    /// 寄存器第5位，模式2 OAM中断开关
    enable_oam_int: bool,
    /// 寄存器第4位，模式1 V-Blank中断开关
    enable_vb_int: bool,
    /// 寄存器第3位，模式0 H-Blank中断开关
    enable_hb_int: bool,
    /// 0~1位，，只读
    /// 0: H-Blank
    /// 1: V-Blank
    /// 2: access OAM
    /// 3: access VRAM
    mode: GPUMode,
}

impl LCDS {
    pub fn power_up() -> Self {
        Self {
            enable_ly_int: false,
            enable_oam_int: false,
            enable_vb_int: false,
            enable_hb_int: false,
            mode: HBlank,
        }
    }

    /// 返回状态寄存器中保存的数值
    pub fn get(&self) -> u8 {
        let mut data = self.mode as u8;
        if self.enable_ly_int {
            data |= 1 << 6;
        }
        if self.enable_oam_int {
            data |= 1 << 5;
        }
        if self.enable_vb_int {
            data |= 1 << 4;
        }
        if self.enable_hb_int {
            data |= 1 << 3;
        }
        return data;
    }

    /// 设置状态寄存器
    pub fn set(&mut self, v: u8) {
        self.enable_ly_int = v & 0x40 != 0;
        self.enable_oam_int = v & 0x20 != 0;
        self.enable_vb_int = v & 0x10 != 0;
        self.enable_hb_int = v & 0x08 != 0;
    }
}

/// BGPI(Background Palette Index)寄存器，彩色模式下使用，保存用于查找背景颜色数据地址的索引
pub struct BGPI {
    /// 寄存器0~5位（表示范围0x00~0x3f），用于寻址BG palettes存储器中的一个字节
    i: u8,
    /// 自增开关，0表示禁用，1表示开启。如果开启，每次BG palettes寄存器写入数据后索引（0~5位）会自增，当读取数据时
    /// 不会自增，因此需要手动递增索引
    auto_inc: bool,
}

impl BGPI {
    pub fn power_up() -> Self {
        Self {
            i: 0x00,
            auto_inc: false,
        }
    }

    pub fn get(&self) -> u8 {
        let a = if self.auto_inc { 0x80 } else { 0x00 };
        self.i | a
    }

    pub fn set(&mut self, v: u8) {
        self.auto_inc = v & 0x80 != 0x00;
        self.i = v & 0x3f;
    }

    pub fn increase(&mut self) {
        if self.auto_inc {
            self.i += 1;
            self.i &= 0x3f;
        }
    }
}

/// 用于寻址Sprite的调色板，其结构与BGPI一致
type OBPI = BGPI;

/// BG或OBJ的属性
pub struct Attr {
    /// 第0-2位：彩色模式下的调色板编号
    cbg_pal_num: usize,
    /// 第3位：VRAM Bank Number，0表示Bank0，1表示Bank1
    /// 该属性表示是否使用Bank1
    bank: bool,
    /// 第4位：黑白模式下的调色板编号
    pal_num: usize,
    /// 第5位：是否镜像翻转x值，1表示翻转
    flip_x: bool,
    /// 第6位：是否镜像翻转y值，1表示翻转
    flip_y: bool,
    /// 第7位：BG相对于Sprite的优先级，1表示BG显示在Sprite之上（前提是BG的色彩编号不能是0，否则Sprite显示在BG之上）
    bw_over_obj: bool,
}

impl From<u8> for Attr {
    fn from(v: u8) -> Self {
        Self {
            cbg_pal_num: v as usize & 0x07,
            bank: (v >> 3) & 0x01 != 0x00,
            pal_num: (v as usize >> 4) & 0x01,
            flip_x: (v >> 5) & 0x01 != 0x00,
            flip_y: (v >> 6) & 0x01 != 0x00,
            bw_over_obj: (v >> 7) & 0x01 != 0x00,
        }
    }
}

/// 颜色数据在内存中的格式，GameBoy使用调色板来整理颜色数据
/// 一共有8个调色板（编号0-7）: Palette0 - Palette7
/// 每个调色板包含4个颜色: Color0 - Color3
/// 每个颜色占用2字节(RGB555)， Bit 0-4: Red, Bit 5-9: Green, Bit 10-14: Blue
/// 调色板共占用64字节，在内存中的排列是：Palette0-color0-byte0, Palette0-color0-byte1,
/// Palette0-color1-byte0, Palette0-color1-byte1 ... Palette7-color3-byte1
struct PaletteData {
    /// 这里我们用RGB模式表示颜色，所以每个颜色占用3字节
    data: [[[u8; 3]; 4]; 8],
}

impl PaletteData {
    fn power_up() -> Self {
        return Self {
            data: [[[0; 3]; 4]; 8],
        };
    }

    /// 获取第i个字节第数据
    fn get(&self, i: u8) -> u8 {
        // 第i个字节对应第Palette number（0~7）
        let p_num = (i >> 3) as usize;
        // 第i个字节对应第Color number（0~3）
        let c_num = (i >> 1 & 0x03) as usize;
        // 第i个字节对应Color的第几个字节（0~1）
        let b_num = i & 0x01;

        let a: u8;
        let b: u8;
        if b_num == 0 {
            // Byte0: gggrrrrr
            // red
            a = self.data[p_num][c_num][0];
            // lower part of green
            b = self.data[p_num][c_num][1] << 5;
        } else {
            // Byte1: 0bbbbbgg
            // green的下半部分: bit 0~2
            a = self.data[p_num][c_num][1] >> 3;
            // green的上半部分: bit 3~4
            b = self.data[p_num][c_num][2] << 2;
        }
        return a | b;
    }

    /// 设置第i个字节第数据
    fn set(&mut self, i: u8, v: u8) {
        // 第i个字节对应第Palette number（0~7）
        let p_num = (i >> 3) as usize;
        // 第i个字节对应第Color number（0~3）
        let c_num = (i >> 1 & 0x03) as usize;
        // 第i个字节对应Color的第几个字节（0~1）
        let b_num = i & 0x01;

        let old_green = self.data[p_num][c_num][1];
        if b_num == 0 {
            // Byte0: gggrrrrr
            // 设置red
            self.data[p_num][c_num][0] = v & 0x1f;
            // 设置green的下半部分: bit 0~2
            self.data[p_num][c_num][1] = (old_green & 0x18) | (v >> 5);
        } else {
            // Byte1: 0bbbbbgg
            // 设置green的上半部分: bit 3~4
            self.data[p_num][c_num][1] = (old_green & 0x07) | ((v & 0x03) << 3);
            // 设置blue
            self.data[p_num][c_num][2] = (v >> 2) & 0x1f;
        }
    }
}

pub struct GPU {
    /// 屏幕的像素数据，采用RGB模式
    pub data: [[[u8; 3]; SCREEN_W as usize]; SCREEN_H as usize],
    /// 用来记录LCDStat中断
    pub intf: Rc<RefCell<Intf>>,
    /// GB型号
    pub term: Term,
    /// 是否发生H-Blank
    pub h_blank: bool,
    /// 是否发生v-blank
    pub v_blank: bool,
    /// LCD控制寄存器
    lcdc: LCDC,
    /// LCD状态寄存器
    lcds: LCDS,
    /// SCY(Scroll Y)寄存器，保存LCD屏幕顶部相对于背景顶部的偏移
    /// 背景大小是: 255*255，屏幕大小是: 160*144
    scy: u8,
    /// SCX(Scroll X)寄存器，保存LCD屏幕左侧区域相对于背景左侧的偏移
    scx: u8,
    /// WY(Window Y Position)寄存器，保存窗口顶部相对于屏幕顶部的偏移
    wy: u8,
    /// WX(Window X Position)寄存器，保存窗口左侧相对于屏幕左侧的偏移
    /// 由于硬件的问题，0-7的范围不可用，所以wx-7才是窗口的实际偏移
    wx: u8,
    /// LY(LCDC-Y Coordinate)寄存器
    /// 表示屏幕中哪一行的数据正在被写入LCD驱动中（也就是正在渲染哪一行），取值范围是0-153，144-153表示发生了V-Bank
    ly: u8,
    /// LYC(LY Compare)寄存器
    /// GameBoy始终将该寄存器的值与ly寄存器中的值比较，如果相等，则触发LCDStat中断（如果允许该中断的话）
    lyc: u8,
    /// BGP(BG Palette Data)寄存器，黑白模式下使用，保存背景或窗口的调色板数据
    /// 1-0位：编号为0的灰度
    /// 3-2位：编号为1的灰度
    /// 5-4位：编号为2的灰度
    /// 7-6位：编号为3的灰度
    bgp: u8,
    /// OBP0(Object Palette 0 Data)寄存器，黑白模式下使用，保存Sprite调色板0的数据
    /// 数据格式与BGP一致，只不过低两位不使用
    obp0: u8,
    /// OBP1(Object Palette 1 Data)寄存器，黑白模式下使用，保存Sprite调色板1的数据
    /// 数据格式与OBP0一致
    obp1: u8,
    /// BGPI寄存器，彩色模式下使用，保存用于查找背景颜色数据地址的索引
    bgpi: BGPI,
    /// 保存背景调色板颜色数据的内存区域，彩色模式下使用
    bgpd: PaletteData,
    /// OBPI(Sprite Palette Index)寄存器，彩色模式下使用，保存用于查找Sprite颜色数据地址的索引
    /// 数据局格式与BGPI一致
    obpi: OBPI,
    /// 保存Sprite调色板颜色数据的内存区域，彩色模式下使用
    obpd: PaletteData,

    /// VRAM内存区域的数据，保存Tile Map和Tile Data
    /// 彩色模式下有两块VRAM内存区域，Bank0和Bank1，通过VBK寄存器来区分使用哪一块内存
    /// Bank0中保存Tile Map，Bank1中对应的区域保存Tile属性
    ram: [u8; 0x4000],
    /// VBK(VRAM Bank)寄存器，用于标识使用哪一块VRAM内存
    /// 0: 使用VRAM Bank0（ram中前0x2000部分）
    /// 1: 使用VRAM Bank1 (ram中后0x2000部分)
    vbk: usize,
    /// 保存Sprite属性表的内存区域，每个Sprite属性占用4个字节
    /// Byte0: Y position，即Sprite底部相对屏幕顶部的距离，Y=0 或 Y>160，Sprite不可见（160代表屏幕高度加sprite
    /// 高度，即: 144+16）
    /// Byte1: X Position，即Sprite右侧相对屏幕左侧的距离，X=0 或 X>168，Sprite不可见（168代表屏幕宽度加sprite
    /// 宽度，即: 160+8）
    /// Byte2: Tile编号，取值范围0-255
    /// Byte3: Tile的属性，结构参考Attr
    oam: [u8; 0xa0],
    /// 表示当前扫描线一共扫描了几个点
    dots: u32,
}

impl GPU {
    pub fn power_up(term: Term, intf: Rc<RefCell<Intf>>) -> Self {
        Self {
            data: [[[0xff; 3]; SCREEN_W as usize]; SCREEN_H as usize],
            intf,
            term,
            h_blank: false,
            v_blank: false,
            lcdc: LCDC::power_up(),
            lcds: LCDS::power_up(),
            scy: 0,
            scx: 0,
            wy: 0,
            wx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            bgpi: BGPI::power_up(),
            bgpd: PaletteData::power_up(),
            obpi: OBPI::power_up(),
            obpd: PaletteData::power_up(),
            ram: [0; 0x4000],
            vbk: 0,
            oam: [0; 0xa0],
            dots: 0,
        }
    }

    /// 获取VRAM中[addr]地址的数据
    fn get_vram(&self, addr: u16) -> u8 {
        return if self.vbk == 0 {
            self.get_ram0(addr)
        } else {
            self.get_ram1(addr)
        };
    }

    /// 获取VRAM Bank0中的数据
    fn get_ram0(&self, addr: u16) -> u8 {
        // 返回ram中前0x2000范围内的数据
        self.ram[addr as usize - 0x8000]
    }

    /// 获取VRAM Bank1中的数据
    fn get_ram1(&self, addr: u16) -> u8 {
        // 返回ram中后0x2000范围内的数据
        self.ram[addr as usize - 0x6000]
    }

    /// 设置VRAM中[addr]地址的数据为[v]
    fn set_vram(&mut self, addr: u16, v: u8) {
        if self.vbk == 0 {
            // 设置VRAM Bank0中的数据
            self.ram[addr as usize - 0x8000] = v;
        } else {
            // 设置VRAM Bank1中的数据
            self.ram[addr as usize - 0x6000] = v;
        }
    }

    /// 是否是彩色模式
    fn is_cgb_mode(&self) -> bool {
        self.term == Term::GBC
    }

    /// 重置GPU数据，当屏幕熄灭时调用
    fn reset(&mut self) {
        self.dots = 0;
        self.ly = 0;
        self.lcds.mode = HBlank;
        // 重置像素数据
        self.data = [[[0xffu8; 3]; SCREEN_W as usize]; SCREEN_H as usize];
        self.v_blank = true;
    }

    /// 执行GPU渲染操作，cycles是CPU执行最近一条指令的时钟周期加上DMA运行的时钟周期
    /// LCD控制器有一个4.194MHZ的dot时钟，一帧画面有154条扫描线，每条扫描线有456个点，一共有70224个点
    pub fn next(&mut self, cycles: u32) {
        if !self.lcdc.lcd_enable() {
            // LCD没有亮起
            return;
        }
        self.h_blank = false;

        if cycles == 0 {
            return;
        }

        let c = (cycles - 1) / 80 + 1;
        for i in 0..c {
            if i == c - 1 {
                self.dots += cycles % 80;
            } else {
                self.dots += 80;
            }
            let d = self.dots;
            self.dots %= 456;
            if d != self.dots {
                // 扫描线换行
                self.ly = (self.ly + 1) % 154;
                if self.lcds.enable_ly_int && self.lyc == self.ly {
                    self.intf.borrow_mut().hi(INTFlag::LCDStat);
                }
            }

            if self.ly >= 144 {
                // 当扫描线在144-153的范围内，GPU处于VBlank模式
                if self.lcds.mode == VBlank {
                    continue;
                }
                self.lcds.mode = VBlank;
                self.v_blank = true;
                self.intf.borrow_mut().hi(INTFlag::VBlank);
                if self.lcds.enable_vb_int {
                    self.intf.borrow_mut().hi(INTFlag::LCDStat);
                }
            } else if self.dots <= 80 {
                // 扫描的点小于80，GPU处于模式2(SearchOAM)
                if self.lcds.mode == SearchOAM {
                    continue;
                }
                self.lcds.mode = SearchOAM;
                if self.lcds.enable_oam_int {
                    self.intf.borrow_mut().hi(INTFlag::LCDStat);
                }
            } else if self.dots <= 80 + 172 {
                // 时钟周期在168和291之间(取决于sprite的数量)，GPU处于模式3(Tran2Driver)
                self.lcds.mode = Tran2Driver;
            } else {
                // 扫描的点位于85到208(取决于模式3的时间)，GPU处于模式0(HBlank)
                if self.lcds.mode == HBlank {
                    continue;
                }
                self.lcds.mode = HBlank;
                self.h_blank = true;
                if self.lcds.enable_hb_int {
                    self.intf.borrow_mut().hi(INTFlag::LCDStat);
                }

                let mut render: Box<dyn Render> = if self.is_cgb_mode() {
                    Box::new(CGBRender::init())
                } else {
                    Box::new(GBRender::init())
                };
                // 渲染屏幕中的一行数据
                render.draw(self);
            }
        }
    }
}

/// 负责背景和Sprite的绘制工作
trait Render {
    /// 渲染扫描线(屏幕中的一行数据)
    fn draw(&mut self, gpu: &mut GPU) {
        // 先渲染背景
        self.draw_bg(gpu);
        // 再渲染Sprite
        self.draw_sprites(gpu);
    }

    /// 在屏幕中绘制一行背景
    ///
    /// 一帧背景的大小是256*256，可平均分成32*32个Tile，作为背景的Tile列表（每个Tile的大小是8*8）,每一帧背景只有
    /// 160*144大小的区域输出在LCD显示屏上
    ///
    /// 首先存在一个全局Tile列表，保存了当前画面所用到的所有Tile数据。TileMap作为一个映射集合，将背景Tile列表一一
    /// 映射到全局Tile列表中。TileMap一共有32*32条映射，对应背景Tile列表的32*32个Tile，每条映射数据为一个8位整数
    /// 类型，表示在全局Tile列表中的编号。此外，背景和Window使用的是不同的TileMap，但他们使用到的Tile数据都在全局
    /// Tile列表中。全局Tile列表保存在内存区域：0x8000-0x97ff，并将其分为三块:
    /// Block0: 0x8000-0x87ff
    /// Block1: 0x8800-0x8fff
    /// Block2: 0x9000-0x97ff
    /// 一共有两种寻址模式(根据LCDC寄存器的第4位来决定寻址模式)：
    /// 8000模式：将0x8000作为起始地址，全局Tile列表保存在Block0和Block1中
    /// 8800模式：将0x8800作为起始地址，全局Tile列表保存在Block1和Block2中
    ///
    /// Tile中每个像素点记录了颜色编号（每个像素点占用2 bit，一个Tile占用16 byte），通过颜色编号可以去调色板查找到
    /// 对应的颜色。在黑白模式下，调色板数据保存在寄存器BGP中，在彩色模式下，调色板数据保存在bgpd内存区域
    fn draw_bg(&mut self, gpu: &mut GPU);

    /// 在屏幕中绘制一行Sprites
    ///
    /// 一个Sprite占用1个Tile或2个纵向排列的Tile（根据LCDC寄存器的第2位决定）
    /// Sprite只能使用8000寻址模式来查找Tile数据
    fn draw_sprites(&mut self, gpu: &mut GPU);

    /// 计算Tile的位置
    /// @Params:
    /// sx: 当前像素点在屏幕中的横坐标
    ///
    /// @Return: (tx, ty, tmap_addr)
    /// tx: 像素点在Tile中的横坐标
    /// ty: 像素点在Tile中的纵坐标
    /// tmap_addr: 当前Tile的Tile map映射地址
    fn _tile_location(&self, gpu: &mut GPU, sx: usize) -> (u8, u8, u16) {
        // wx-7为真实的window水平偏移量
        let wx = gpu.wx.wrapping_sub(7);
        // 当前点是否处于window区域内
        let in_win = gpu.lcdc.window_enable()
            && gpu.ly >= gpu.wy
            && sx as u8 >= wx;
        // 当前像素点的横向偏移量
        let x: u8;
        // 当前像素点的纵向偏移量
        let y: u8;
        // 前点所处的Tile在Tile列表中的哪一行
        let t_row: u8;
        // 当前点所处的Tile在Tile列表中的哪一列，根据t_row和t_col可以定位到当前点所在的Tile
        let t_col: u8;
        // tmap_base保存TileMap内存区域的起始地址（背景和Window使用不同的TileMap）
        let tmap_base: u16;
        if gpu.lcdc.window_enable()
            && gpu.ly >= gpu.wy {
            y = gpu.ly - gpu.wy;
        } else {
            y = gpu.scy.wrapping_add(gpu.ly);
        }
        if in_win {
            // Window TileMap根据像素点在Window内的偏移来定位Tile
            x = sx as u8 - wx;
            // 根据LCDC寄存器的第6位来决定Window TileMap的起始地址
            tmap_base = if gpu.lcdc.win_tm_sel() { 0x9c00 } else { 0x9800 };
        } else {
            // 背景TileMap根据像素点在背景中的偏移来定位Tile
            x = gpu.scx.wrapping_add(sx as u8);
            // 根据LCDC寄存器的第3位来决定背景TileMap的起始地址
            tmap_base = if gpu.lcdc.bg_tm_sel() { 0x9c00 } else { 0x9800 };
        };
        t_row = y / 8;
        t_col = x / 8;

        // 当前像素在Tile中的横坐标
        let tx = x % 8;
        // 当前像素在Tile中的纵坐标
        let ty = y % 8;
        // 当前像素点所处的Tile的TileMap映射地址
        let tmap_addr = tmap_base + t_row as u16 * 32 + t_col as u16;
        return (tx, ty, tmap_addr);
    }

    /// 计算Tile data的地址
    /// @Param
    /// tmap_addr: 当前Tile在Tile map中的映射地址
    ///
    /// @Return
    /// 当前Tile数据的地址
    fn _tile_addr(&self, gpu: &mut GPU, tmap_addr: u16) -> u16 {
        // 当前像素点所处的Tile在全局Tile列表中的编号(0~255)
        let t_num = gpu.get_ram0(tmap_addr);
        // 全局Tile列表的起始地址
        let td_addr_base: u16;
        // 保存Tile数据的地址距离全局Tile列表起始地址的偏移量
        let t_offset: u16;

        // 根据LCDC寄存器的第4位来决定全局Tile列表寻址模式
        if gpu.lcdc.td_sel() {
            // 全局Tile列表采用8000寻址模式，TileMap中的编号是一个无符号8位数
            td_addr_base = 0x8000;
            t_offset = t_num as u16;
        } else {
            // 全局Tile列表采用8800寻址模式，此时Tile的数据保存在0x8800到0x97ff之间，TileMap中的编号是一个
            // 有符号8位数（-127~127），所以需要将获取到的Tile编号加128，以获得正确的内存地址
            td_addr_base = 0x8800;
            t_offset = (i16::from(t_num as i8) + 128) as u16;
        }

        return td_addr_base + t_offset * 16;
    }

    /// 计算像素点的颜色编号
    /// tx: Tile单行中的第几个像素
    /// b1: Tile单行像素数据的第1个字节
    /// b2: Tile单行像素数据的第2个字节
    ///
    /// 一个像素点由2个bit组成，Tile包含8*8个像素点，共占用16个字节，每个像素点保存的是颜色编号，取值范围：0~3
    /// Tile每一行占用2字节，这两个字节按照如下规则交叉组成每一个像素的颜色编号：
    /// Pixel: 1 2 3 4 5 6 7 8
    /// Byte2: 0 1 0 0 1 1 1 0
    /// Byte1: 1 0 0 1 0 0 0 1
    /// 第1个像素的颜色编号：01
    /// 第2个像素的颜色编号：10
    /// 第3个像素的颜色编号：00
    /// 第4个像素的颜色编号：01
    /// 第5个像素的颜色编号：10
    /// 第6个像素的颜色编号：10
    /// 第7个像素的颜色编号：10
    /// 第8个像素的颜色编号：01
    fn _cal_color_num(&self, tx: u8, b1: u8, b2: u8) -> usize {
        let color_l = if (0x80 >> tx) & b1 != 0 { 0x01 } else { 0x00 };
        let color_h = if (0x80 >> tx) & b2 != 0 { 0x02 } else { 0x00 };
        (color_h | color_l) as usize
    }

    /// 返回Sprite的OAM数据: (sy, sx, t_num, attr)
    /// sy: Sprite顶部相对于屏幕顶部的偏移
    /// sx: Sprite左侧相对于屏幕左侧的偏移
    /// t_num: 组成Sprite的Tile的编号
    /// attr: Sprite的属性
    fn _oam_data(&self, gpu: &mut GPU, i: usize) -> (u8, u8, u8, Attr) {
        // Sprite的属性列表的内存地址
        let attr_addr = 0xfe00 + (i as u16) * 4;
        // Sprite顶部相对于屏幕顶部的偏移，sprite默认占用两个tile，所以要减去16
        let sy = gpu.get(attr_addr).wrapping_sub(16);
        // Sprite左侧相对于屏幕左侧的偏移
        let sx = gpu.get(attr_addr + 1).wrapping_sub(8);
        // 组成Sprite的Tile的编号
        let mut t_num = gpu.get(attr_addr + 2);
        if gpu.lcdc.obj_h_16() {
            // 如果sprite的高度是16，相邻的地址指向同一个tile number
            t_num &= 0xfe;
        }
        // Sprite的属性
        let attr = Attr::from(gpu.get(attr_addr + 3));
        return (sy, sx, t_num, attr);
    }

    /// 判断是否需要绘制此Sprite
    /// sx: Sprite左边相对与屏幕左边的偏移
    /// sy: Sprite顶部相对于屏幕顶部的偏移
    /// sprite_height: Sprite高度 (8或16)
    fn _is_draw_sprite(&self, gpu: &mut GPU, sx: u8, sy: u8, sprite_height: u8) -> bool {
        if sy <= 0xff - sprite_height + 1 {
            // Sprite底部没有超过画面（255*255）
            if gpu.ly < sy || gpu.ly >= sy + sprite_height {
                // 当前扫描线并没有经过该Sprite
                return false;
            }
        } else if gpu.ly >= sy.wrapping_add(sprite_height) {
            // Sprite底部超出画面（屏幕大小255*255, 超出部分显示在画面顶部）且当前扫描线并没有经过该Sprite
            return false;
        }
        if sx >= SCREEN_W as u8 && sx <= 0xff - 7 {
            // Sprite左侧超出画面
            return false;
        }
        return true;
    }

    /// Sprite的高度，一个Sprite可能是一个Tile， 也可能是两个纵向排列的Tile组成，由LCDC寄存器的第2位决定
    fn _sprite_height(&self, gpu: &mut GPU) -> u8 {
        if gpu.lcdc.obj_h_16() { 16 } else { 8 }
    }

    /// 是否允许绘制Sprite
    fn _enable_sprite(&self, gpu: &mut GPU) -> bool {
        gpu.lcdc.obj_enable()
    }
}

/// 黑白模式下绘制背景和sprite
struct GBRender {
    /// 记录已经绘制过的Sprite在屏幕中的横坐标，升序排列
    _has_draw: Vec<u8>,
    /// 记录背景是否为透明
    _bg_trans: Vec<bool>,
}

impl GBRender {
    fn init() -> Self {
        return Self {
            _has_draw: Vec::with_capacity(10),
            _bg_trans: vec![false; SCREEN_W as usize],
        };
    }
    /// 根据颜色编号获取相应的灰度值
    /// bgp: 参考BGP寄存器的数据结构
    /// i: 灰度值编号，0-3
    fn _get_gray_shades(&self, bgp: u8, i: usize) -> GrayShades {
        assert!(i < 4, "Invalid gray shade number {}, it has to be at range [0, 3]", i);
        match (bgp >> (i * 2)) & 0x03 {
            0x00 => GrayShades::White,
            0x01 => GrayShades::Light,
            0x02 => GrayShades::Dark,
            _ => GrayShades::Blank,
        }
    }

    /// 将指定像素点的灰度值填充到屏幕像素数据中
    fn _set_gray(&mut self, gpu: &mut GPU, x: usize, g: u8) {
        gpu.data[gpu.ly as usize][x] = [g, g, g];
    }
}

impl Render for GBRender {
    /// 黑白模式下绘制一行背景
    fn draw_bg(&mut self, gpu: &mut GPU) {
        if !gpu.lcdc.bg_win_pri() {
            // 黑白模式下由LCDC的第0位决定是否渲染背景
            return;
        }

        // 首先遍历ly记录的扫描线，找出每个点所在的Tile
        for sx in 0..SCREEN_W as usize {
            // tx: 当前像素在Tile中的横坐标 (以左上角为原点)
            // ty: 当前像素在Tile中的纵坐标
            let (tx, ty, tmap_addr) = self._tile_location(gpu, sx);
            // 保存当前Tile数据的地址
            let tile_addr = self._tile_addr(gpu, tmap_addr);
            // 保存当前像素所处的Tile行的内存地址
            let tr_addr = tile_addr + ty as u16 * 2;
            // 黑白模式下只有VRAM Bank0可用
            // 当前像素点所处的Tile行颜色数据的第1个字节
            let tr0 = gpu.get_ram0(tr_addr);
            // 当前像素点所处的Tile行颜色数据的第2个字节
            let tr1 = gpu.get_ram0(tr_addr + 1);
            // 当前像素点的颜色编号
            let color_num = self._cal_color_num(tx, tr0, tr1);
            // 记录当前绘制的背景是否透明
            self._bg_trans[sx] = color_num == 0;
            let gray = self._get_gray_shades(gpu.bgp, color_num) as u8;
            self._set_gray(gpu, sx, gray);
        }
    }

    /// 黑白模式下绘制一行Sprite
    fn draw_sprites(&mut self, gpu: &mut GPU) {
        if !self._enable_sprite(gpu) {
            return;
        }

        // Sprite的高度
        let sprite_height = self._sprite_height(gpu);

        // 屏幕中最多显示40个Sprites
        for i in 0..40 {
            let (sy, sx, t_num, attr) = self._oam_data(gpu, i);
            if !self._is_draw_sprite(gpu, sx, sy, sprite_height) {
                continue;
            }

            let oty = gpu.ly.wrapping_sub(sy);
            // 要绘制的点在Sprite中的纵坐标（考虑到是否垂直翻转该Sprite）
            let ty = if attr.flip_y { sprite_height - 1 - oty } else { oty };
            // 要绘制的点所处的Tile行的内存地址，在全局Tile列表中查找Sprite，只有8000这一种寻址模式
            let tr_addr = 0x8000 + t_num as u16 * 16 + ty as u16 * 2;
            // 要绘制像素点数据的第1个字节
            let tr0 = gpu.get_ram0(tr_addr);
            // 要绘制像素点数据的第2个字节
            let tr1 = gpu.get_ram0(tr_addr + 1);

            // 所有已绘制的Sprite按照自己在屏幕出现的横坐标升序排序，当前Sprite排第几
            let si = bs_insert_at(&self._has_draw, sx as u8);
            // 绘制一行Sprite
            for x in 0..8u8 {
                // 当前点在屏幕中的横坐标
                let px = sx.wrapping_add(x);
                if px >= SCREEN_W as u8 {
                    // 超出屏幕可显示区域
                    continue;
                }

                // 在黑白模式下，多个Sprite重叠时，靠近屏幕左侧的Sprite优先级最高
                if si > 0 && self._has_draw[si - 1] + 7 >= px {
                    // 当前Sprite被左侧的Sprite遮挡
                    continue;
                }

                // 处理Sprite和bg，window的优先级
                if attr.bw_over_obj && !self._bg_trans[px as usize] {
                    // Sprite属性的第7位是1，且bg的颜色编号不为0，优先显示bg
                    continue;
                }

                // 要绘制的点在Sprite中的横坐标（考虑到是否水平翻转该Sprite）
                let tx = if attr.flip_x { 7 - x } else { x };
                // 要绘制像素点的颜色编号
                let color_num = self._cal_color_num(tx, tr0, tr1);
                if color_num == 0 {
                    // Sprite不能使用颜色编号0（Sprite中color 0表示透明）
                    continue;
                }

                // 从调色板中获取实际的颜色并向屏幕数据区域填充该像素的rgb数据
                let palette = if attr.pal_num == 1 { gpu.obp1 } else { gpu.obp0 };
                let gray = self._get_gray_shades(palette, color_num) as u8;
                self._set_gray(gpu, px as usize, gray);
            }

            // 记录已绘制的Sprite
            self._has_draw.insert(si, sx);
        }
    }
}

/// 彩色模式下绘制背景和sprite
struct CGBRender {
    /// 记录当前绘制行的背景Tile中的priority属性
    _bg_prio: [bool; SCREEN_W as usize],
    /// 记录当前绘制行的背景颜色编号
    _bg_colors: [u8; SCREEN_W as usize],
    /// 记录一行中的点是否被绘制过Sprite，用于处理多个Sprites重叠的问题
    _draws: HashMap<u8, bool>,

}

impl CGBRender {
    fn init() -> Self {
        CGBRender {
            _bg_prio: [false; SCREEN_W as usize],
            _bg_colors: [0; SCREEN_W as usize],
            _draws: HashMap::new(),
        }
    }

    /// 将指定像素点的rgb值填充到屏幕像素数据中
    fn _set_rgb(&self, gpu: &mut GPU, x: usize, r: u8, g: u8, b: u8) {
        // 原始rgb数据每个通道只有5位，只能表示0到32
        assert!(r <= 0x1f, "Invalid red channel {:#04x}, it has to be at range [0x00, 0x1f]", r);
        assert!(g <= 0x1f, "Invalid green channel {:#04x}, it has to be at range [0x00, 0x1f]", r);
        assert!(b <= 0x1f, "Invalid blue channel {:#04x}, it has to be at range [0x00, 0x1f]", r);
        // 将原始的0~32的色彩通道范围拉伸到0~255
        let r = u32::from(r);
        let g = u32::from(g);
        let b = u32::from(b);
        // 非线性的拉伸算法，产生的结果对人眼比较友好
        let lr = ((r * 13 + g * 2 + b) >> 1) as u8;
        let lg = ((g * 3 + b) << 1) as u8;
        let lb = ((r * 3 + g * 2 + b * 11) >> 1) as u8;
        gpu.data[gpu.ly as usize][x] = [lr, lg, lb];
    }
}


impl Render for CGBRender {
    /// 彩色模式下绘制一行背景
    fn draw_bg(&mut self, gpu: &mut GPU) {
        // 首先遍历ly记录的扫描线，找出每个点所在的Tile
        for sx in 0..SCREEN_W as usize {
            // tx: 当前像素在Tile中的横坐标 (以左上角为原点)
            // ty: 当前像素在Tile中的纵坐标
            let (mut tx, mut ty, tmap_addr) = self._tile_location(gpu, sx);
            // 保存当前Tile数据的地址
            let tile_addr = self._tile_addr(gpu, tmap_addr);

            // 彩色模式下需要通过TileMap地址从VRAM Bank1获取Tile属性
            let t_attr = Attr::from(gpu.get_ram1(tmap_addr));
            if t_attr.flip_x {
                // x轴镜像翻转Tile
                tx = 7 - tx;
            }
            if t_attr.flip_y {
                // y轴镜像翻转Tile
                ty = 7 - ty;
            }

            // 保存当前像素所处的Tile行的内存地址
            let tr_addr = tile_addr + ty as u16 * 2;
            // tr0: 当前像素点所处的Tile行颜色数据的第1个字节
            // tr1: 当前像素点所处的Tile行颜色数据的第2个字节
            let (tr0, tr1) =
                // 根据Tile属性中的bank标志位来决定使用哪一块VRAM内存空间
                if t_attr.bank {
                    // 从VRAM Bank1中获取Tile数据
                    (gpu.get_ram1(tr_addr), gpu.get_ram1(tr_addr + 1))
                } else {
                    // 从VRAM Bank0中获取Tile数据
                    (gpu.get_ram0(tr_addr), gpu.get_ram0(tr_addr + 1))
                };

            // 当前像素点的颜色编号
            let color_num = self._cal_color_num(tx, tr0, tr1);
            // 根据颜色编号获取实际的rgb颜色
            let color = gpu.bgpd.data[t_attr.cbg_pal_num][color_num];
            // 向屏幕数据区域填充该像素的rgb数据
            self._set_rgb(gpu, sx, color[0], color[1], color[2]);

            // 保存背景的优先级信息
            self._bg_prio[sx] = t_attr.bw_over_obj;
            // 保存背景颜色信息
            self._bg_colors[sx] = color_num as u8;
        }
    }

    /// 彩色模式绘制一行sprite
    fn draw_sprites(&mut self, gpu: &mut GPU) {
        if !self._enable_sprite(gpu) {
            return;
        }
        let sprite_height = self._sprite_height(gpu);

        // 屏幕中最多显示40个Sprites
        for i in 0..40 {
            let (sy, sx, t_num, attr) = self._oam_data(gpu, i);
            if !self._is_draw_sprite(gpu, sx, sy, sprite_height) {
                continue;
            }
            let oty = gpu.ly.wrapping_sub(sy);
            // 要绘制的点在Sprite中的纵坐标（考虑到是否垂直翻转该Sprite）
            let ty = if attr.flip_y { sprite_height - 1 - oty } else { oty };
            // let ty = if attr.flip_y { sprite_height - 1 - (gpu.ly - sy) } else { gpu.ly - sy };
            // 要绘制的点所处的Tile行的内存地址，在全局Tile列表中查找Sprite，只有8000这一种寻址模式
            let tr_addr = 0x8000 + t_num as u16 * 16 + ty as u16 * 2;
            // tr0: 要绘制像素点数据的第1个字节, tr1: 要绘制像素点数据的第2个字节
            let (tr0, tr1) = if attr.bank {
                // 彩色模式下，如果Sprite属性中的bank为1，则从VRAM Bank1中获取Tile数据
                (gpu.get_ram1(tr_addr), gpu.get_ram1(tr_addr + 1))
            } else {
                // 从VRAM Bank0中获取Tile数据
                (gpu.get_ram0(tr_addr), gpu.get_ram0(tr_addr + 1))
            };

            // 绘制一行Sprite
            for x in 0..8u8 {
                // 当前点在屏幕中的横坐标
                let px = sx.wrapping_add(x) as usize;
                if px >= SCREEN_W as usize {
                    // 超出屏幕可显示区域
                    continue;
                }
                // 该像素点是否已经被绘制过
                let has_draw = *self._draws.entry(px as u8).or_default();

                // 在彩色模式下，多个Sprite重叠时，在OAM中最先出现的Sprite优先级最高，也就是说，如果当前点已经被
                // 绘制过，则后出现的Sprite不应该再覆盖之前的绘制
                if has_draw {
                    continue;
                }

                // 要绘制的点在Sprite中的横坐标（考虑到是否水平翻转该Sprite）
                let tx = if attr.flip_x { 7 - x } else { x };

                // 要绘制像素点的颜色编号
                let color_num = self._cal_color_num(tx, tr0, tr1);
                if color_num == 0 {
                    // Sprite不能使用颜色编号0（Sprite中color 0表示透明）
                    continue;
                }

                // 根据Sprite和背景的优先级，判断是否绘制Sprite
                // 在彩色模式下如果LCDC寄存器的第0位是0，Sprite将显示到背景之上（忽略Tile属性和OAM中的设置），否
                // 则依次根据背景Tile属性和Sprite属性来决定Sprite和背景的优先级
                if gpu.lcdc.bg_win_pri() {
                    if self._bg_prio[px] {
                        // 如果背景Tile属性中的priority为1，则优先显示背景，否则采用Sprite属性中的优先级
                        continue;
                    }
                    if attr.bw_over_obj && self._bg_colors[px] != 0 {
                        // 如果Sprite属性中的priority为1并且背景颜色不为0，则优先显示背景，否则显示Sprite
                        continue;
                    }
                }

                // 从调色板中获取实际的颜色并向屏幕数据区域填充该像素的rgb数据
                let color = gpu.obpd.data[attr.pal_num][color_num];
                self._set_rgb(gpu, px, color[0], color[1], color[2]);

                // 标记当前像素点已经被绘制过
                self._draws.insert(px as u8, true);
            }
        }
    }
}

/// list是一个有序数组，v是一个将要被插入数组的元素，用二分法查找v应该被插入的位置 (假设v要插入到返回位置的左边)
fn bs_insert_at(list: &Vec<u8>, v: u8) -> usize {
    if list.is_empty() {
        // list为空，直接插入头部
        return 0;
    }

    if list.len() == 1 {
        // 只有一个元素
        return if v > list[0] { 1 } else { 0 };
    }

    let max = list.len() - 1;
    // 用二分法找出插入的位置, l是左边界，r是右边界，c是当前对比的索引
    let mut l = 0;
    let mut r = max;
    let mut c = (l + r) / 2;

    loop {
        if v >= list[c] {
            // 第c位偏小，调整左边界
            l = c;
        } else {
            // 第c位偏大，调整右边界
            r = c;
        }
        c = (l + r) / 2;

        if c == l {
            // l和r相邻，达到遍历终止条件
            return if v < list[l] {
                // v会被插入第l个元素左边
                l
            } else if v > list[r] {
                // v会被插入在第r个元素右边
                r + 1
            } else {
                // v会被插入第l和r个元素之间
                r
            };
        }
    }
}

impl Memory for GPU {
    fn get(&self, a: u16) -> u8 {
        match a {
            // VRAM中的数据
            0x8000..=0x9fff => self.get_vram(a),
            // OAM内存区域的数据
            0xfe00..=0xfe9f => self.oam[a as usize - 0xfe00],
            // LCD控制寄存器中的值
            0xff40 => self.lcdc.data,
            // LCD状态寄存器中的值
            0xff41 => self.lcds.get(),
            // Scroll Y 寄存器的值
            0xff42 => self.scy,
            // Scroll X 寄存器的值
            0xff43 => self.scx,
            // LY(LCDC-Y Coordinate)寄存器的值
            0xff44 => self.ly,
            // LYC(LY Compare)寄存器的值
            0xff45 => self.lyc,
            // BGP(BG Palette Data)寄存器的值
            0xff47 => self.bgp,
            // OBP0(Object Palette 0 Data)寄存器的值
            0xff48 => self.obp0,
            // OBP1(Object Palette 1 Data)寄存器的值
            0xff49 => self.obp1,
            // WY(Window Y Position)寄存器的值
            0xff4a => self.wy,
            // WX(Window X Position)寄存器的值
            0xff4b => self.wx,
            // VBK(VRAM Bank)寄存器的值
            0xff4f => 0xfe | self.vbk as u8,
            // BGPI寄存器的值
            0xff68 => self.bgpi.get(),
            // 根据BGPI寄存器中的索引，获取1字节的背景调色板中的颜色数据
            0xff69 => self.bgpd.get(self.bgpi.i),
            // OBPI寄存器的值
            0xff6a => self.obpi.get(),
            // 根据OBPI寄存器中的索引，获取1字节的Sprite调色板中的颜色数据
            0xff6b => self.obpd.get(self.obpi.i),
            _ => panic!("Invalid to read GPU address: {}", a),
        }
    }

    fn set(&mut self, a: u16, v: u8) {
        match a {
            // 设置VRAM中的数据
            0x8000..=0x9fff => self.set_vram(a, v),
            // 设置OAM内存区域的数据
            0xfe00..=0xfe9f => {
                self.oam[a as usize - 0xfe00] = v;
            }
            // 设置LCD控制寄存器中的值
            0xff40 => {
                self.lcdc.data = v;
                if !self.lcdc.lcd_enable() {
                    // 屏幕熄灭
                    self.reset();
                }
            }
            // 设置LCD状态寄存器中的值
            0xff41 => self.lcds.set(v),
            // 设置Scroll Y 寄存器的值
            0xff42 => self.scy = v,
            // 设置Scroll X 寄存器的值
            0xff43 => self.scx = v,
            // 设置LY(LCDC-Y Coordinate)寄存器的值
            0xff44 => {}
            // 设置LYC(LY Compare)寄存器的值
            0xff45 => self.lyc = v,
            // 设置BGP(BG Palette Data)寄存器的值
            0xff47 => self.bgp = v,
            // 设置OBP0(Object Palette 0 Data)寄存器的值
            0xff48 => self.obp0 = v,
            // 设置OBP1(Object Palette 1 Data)寄存器的值
            0xff49 => self.obp1 = v,
            // 设置WY(Window Y Position)寄存器的值
            0xff4a => self.wy = v,
            // 设置WX(Window X Position)寄存器的值
            0xff4b => self.wx = v,
            // 设置VBK(VRAM Bank)寄存器的值
            0xff4f => self.vbk = (v & 0x01) as usize,
            // 设置BGPI寄存器的值
            0xff68 => self.bgpi.set(v),
            // 根据BGPI寄存器中的索引，设置背景调色板中的颜色数据
            0xff69 => {
                self.bgpd.set(self.bgpi.i, v);
                self.bgpi.increase();
            }
            // 设置OBPI寄存器的值
            0xff6a => self.obpi.set(v),
            // 根据OBPI寄存器中的索引，设置Sprite调色板中的颜色数据
            0xff6b => {
                self.obpd.set(self.obpi.i, v);
                self.obpi.increase();
            }
            _ => panic!("Invalid to set GPU address: {}", a),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_draw() {
        let mut has_draw: Vec<u8> = Vec::with_capacity(40);

        let sx_list = vec![10, 32, 14, 2, 210, 1, 0, 230, 240, 96];
        let order_list = vec![0, 1, 1, 0, 4, 0, 0, 7, 8, 6];
        let final_list = vec![0, 1, 2, 10, 14, 32, 96, 210, 230, 240];
        for i in 0..sx_list.len() {
            let sx = sx_list[i];
            let insert_at = bs_insert_at(&has_draw, sx);

            assert_eq!(insert_at, order_list[i]);
            has_draw.insert(insert_at, sx);
        }

        assert_eq!(&has_draw[..], &final_list);
    }
}
