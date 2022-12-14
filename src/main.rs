use sfml::{
    graphics::{
        Color, RectangleShape, RenderTarget,
        RenderWindow, Shape, Transformable,
    },
    window::{
        ContextSettings, Event, 
        Style, Key, VideoMode,
    },
    system::Vector2i,
    // TODO: audio::{Sound, SoundBuffer}
};

use std::{env, fs::File, io::Read, thread, time,};
use rand::Rng;


// TODO: Make this externally configurable
const CLOCK_SLEEP: u64 = 1000/1000;


struct Chip8 {
    opcode: u16,
    memory: [u8; 4096],
    V: [u8; 16],
    I: u16,
    pc: u16,
    gfx: [[u8; 64]; 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    sp: u16,
    key: [u8; 16]
}

const FONTSET: [u8; 80] = [ 
    0xF0, 0x90, 0x90, 0x90, 0xF0,
    0x20, 0x60, 0x20, 0x20, 0x70,
    0xF0, 0x10, 0xF0, 0x80, 0xF0,
    0xF0, 0x10, 0xF0, 0x10, 0xF0,
    0x90, 0x90, 0xF0, 0x10, 0x10,
    0xF0, 0x80, 0xF0, 0x10, 0xF0,
    0xF0, 0x80, 0xF0, 0x90, 0xF0,
    0xF0, 0x10, 0x20, 0x40, 0x40,
    0xF0, 0x90, 0xF0, 0x90, 0xF0,
    0xF0, 0x90, 0xF0, 0x10, 0xF0,
    0xF0, 0x90, 0xF0, 0x90, 0x90,
    0xE0, 0x90, 0xE0, 0x90, 0xE0,
    0xF0, 0x80, 0x80, 0x80, 0xF0,
    0xE0, 0x90, 0x90, 0x90, 0xE0,
    0xF0, 0x80, 0xF0, 0x80, 0xF0,
    0xF0, 0x80, 0xF0, 0x80, 0x80 
];

fn rom_from_file(filepath: &str) -> ([u8; 4096], usize) {
    let mut f = File::open(filepath).expect("File not found");
    let mut buf = [0u8; 4096];

    let bytes_read = match f.read(&mut buf) {
        Ok(bytes_read) => { bytes_read }
        Err(_) => 0,
    };

    (buf, bytes_read)
}

fn split_opcode(opcode: u16) -> (u16, u16, u16, u16) {
    let n1 = opcode / 0x1000;
    let n2 = opcode % 0x1000 / 0x100;
    let n3 = opcode % 0x100 / 0x10;
    let n4 = opcode % 0x10 / 0x1;

    (n1, n2, n3, n4)
}

pub trait JoinHexInt {
    fn join_hex_ints(&self) -> u16;
}

impl JoinHexInt for (u16, u16) {
    fn join_hex_ints(&self) -> u16 {
        let (a, b) = self;
        a * 0x10 + b * 0x1
    }
}
impl JoinHexInt for (u16, u16, u16) {
    fn join_hex_ints(&self) -> u16 {
        let (a, b, c) = self;
        a * 0x100 + b * 0x10 + c * 0x1
    }
}

impl Chip8 {
    pub fn new(filepath: &str) -> Chip8 {
        let mut mem: [u8; 4096] = [0; 4096];

        for i in 0..80 {
            mem[i] = FONTSET[i];
        }
    
        let rom_and_size = rom_from_file(filepath);
        let rom = rom_and_size.0;
        let bufsize = rom_and_size.1;
    
        for i in 0..bufsize {
            mem[i + 512] = rom[i];
        }

        Chip8 {
            opcode: 0,
            memory: mem,
            V: [0; 16],
            I: 0,
            pc: 0x200,
            gfx: [[0; 64]; 32],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            sp: 0,
            key: [0; 16]
        }
    }
    
    pub fn clockcycle(&mut self, sleep_time_ms: u64) {
        let mem_pc1 = self.memory[self.pc as usize] as u16;
        let mem_pc2 = self.memory[self.pc as usize + 1] as u16;
        self.opcode = mem_pc1 << 8 | mem_pc2;
        self.pc += 2;

        let split_opcode: (u16, u16, u16, u16) = split_opcode(self.opcode);    
        self.exec_opcode(split_opcode);

        // thread::sleep(time::Duration::from_millis(sleep_time_ms));
    }

    pub fn exec_opcode(&mut self, split_opcode: (u16, u16, u16, u16)) {
        match split_opcode {
            (0x0, 0x0, 0xe, 0x0) => self.cls(),
            (0x0, 0x0, 0xe, 0xe) => self.ret(),

            (0x1, n1, n2, n3) => self.jmp((n1, n2, n3).join_hex_ints()),
            (0x2, n1, n2, n3) => self.call((n1, n2, n3).join_hex_ints()),
            (0x3, x, k1, k2) => self.se_vx_b(x, (k1, k2).join_hex_ints()),
            (0x4, x, k1, k2) => self.sne_vx_b(x, (k1, k2).join_hex_ints()),
            (0x5, x, y, 0x0) => self.se_vx_vy(x, y),
            (0x6, x, k1, k2) => self.ld_vx_b(x, (k1, k2).join_hex_ints()),
            (0x7, x, k1, k2) => self.add_vx_b(x, (k1, k2).join_hex_ints()),

            (0x8, x, y, 0x0) => self.ld_vx_vy(x, y),    
            (0x8, x, y, 0x1) => self.or(x, y),
            (0x8, x, y, 0x2) => self.and(x, y),
            (0x8, x, y, 0x3) => self.xor(x, y),
            (0x8, x, y, 0x4) => self.add_vf(x, y),
            (0x8, x, y, 0x5) => self.sub(x, y),
            (0x8, x, _y, 0x6) => self.shr(x),
            (0x8, x, y, 0x7) => self.subn(x, y),
            (0x8, x, _y, 0xe) => self.shl(x),
            (0x9, x, y, 0x0) => self.sne(x, y),

            (0xA, n1, n2, n3) => self.ld_i_a((n1, n2, n3).join_hex_ints()),
            (0xB, n1, n2, n3) => self.jmp_v0_a((n1, n2, n3).join_hex_ints()),
            (0xC, x, k1, k2) => self.rnd(x, (k1, k2).join_hex_ints()),
            (0xD, x, y, n) => self.draw(x, y, n),
            
            (0xE, x, 0x9, 0xE) => self.skp_vx(x),
            (0xE, x, 0xA, 0x1) => self.sknp_vx(x),

            (0xF, x, 0x0, 0x7) => self.ld_vx_dt(x),
            (0xF, x, 0x0, 0xA) => self.ld_vx_k(x),
            (0xF, x, 0x1, 0x5) => self.ld_dt_vx(x),
            (0xF, x, 0x1, 0x8) => self.ld_st_vx(x),
            (0xF, x, 0x1, 0xE) => self.add_i_vx(x),
            (0xF, x, 0x2, 0x9) => self.ld_f_vx(x),
            (0xF, x, 0x3, 0x3) => self.ld_vx(x),
            (0xF, x, 0x5, 0x5) => self.ld_i_vx(x),
            (0xF, x, 0x6, 0x5) => self.ld_vx_i(x),
            
            _ => {
                let oc = self.opcode;
                println!("Error, opcode: {oc}");
            },
        }       
    }


    fn cls(&mut self) {
        self.gfx = [[0; 64]; 32];
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    fn jmp(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    fn call(&mut self, nnn: u16) {
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = nnn;
    }

    fn se_vx_b(&mut self, x: u16, kk: u16) {
        if self.V[x as usize] == kk as u8 {
            self.pc += 2;
        }
    }

    fn sne_vx_b(&mut self, x: u16, kk: u16) {
        if self.V[x as usize] != kk as u8 {
            self.pc += 2;
        }
    }

    fn se_vx_vy(&mut self, x: u16, y: u16) {
        if self.V[x as usize] == self.V[y as usize] {
            self.pc += 2;
        }
    }

    fn ld_vx_b(&mut self, x: u16, kk: u16) {
        self.V[x as usize] = kk as u8;
    } 

    fn add_vx_b(&mut self, x: u16, kk: u16) {
        self.V[x as usize] = self.V[x as usize].wrapping_add(kk as u8);
    }

    fn ld_vx_vy(&mut self, x: u16, y: u16) {
        self.V[x as usize] = self.V[y as usize];
    }

    fn or(&mut self, x: u16, y: u16) {
        self.V[x as usize] |= self.V[y as usize]; 
    }

    fn and(&mut self, x: u16, y: u16) {
        self.V[x as usize] &= self.V[y as usize];
    }

    fn xor(&mut self, x: u16, y: u16) {
        self.V[x as usize] ^= self.V[y as usize];
    }

    fn add_vf(&mut self, x: u16, y: u16) {
        let vx = self.V[x as usize] as u16;
        let vy = self.V[y as usize] as u16;
        let x_and_y = vx + vy;

        self.V[0xF] = if x_and_y > 0xff {1} else {0};
        self.V[x as usize] = x_and_y as u8 & 0xff;
    }

    fn sub(&mut self, x: u16, y: u16) {
        let vx = self.V[x as usize];
        let vy = self.V[y as usize];

        self.V[0xF] = if vx > vy {1} else {0};
        self.V[x as usize] = self.V[x as usize].wrapping_sub(self.V[y as usize]);
    }

    fn shr(&mut self, x: u16) {
        self.V[0xF] = self.V[x as usize] & 1;
        self.V[x as usize] >>= 1;
    }

    fn subn(&mut self, x: u16, y: u16) {
        self.V[0xF] = 0;

        if self.V[y as usize] > self.V[x as usize] {
            self.V[0xF] = 1;
        }
        self.V[x as usize] = self.V[y as usize].wrapping_sub(self.V[x as usize]);
    }

    fn shl(&mut self, x: u16) {
        self.V[0xF] = self.V[x as usize] & 0x80;
        self.V[x as usize] <<= 1;
    }

    fn sne(&mut self, x: u16, y: u16) {
        if self.V[x as usize] != self.V[y as usize] {
            self.pc += 2;
        }
    }

    fn ld_i_a(&mut self, nnn: u16) {
        self.I = nnn;
    }

    fn jmp_v0_a(&mut self, nnn: u16) {
        self.pc += nnn + self.V[0x0] as u16;
    }

    fn rnd(&mut self, x: u16, kk: u16) {
        let rng_num: u8 = rand::thread_rng().gen::<u8>();
        self.V[x as usize] = rng_num & kk as u8;
    }
    
    fn draw(&mut self, x: u16, y: u16, n: u16) {
        self.V[0xF] = 0x0;

        for row_iter in 0..n as u8 {
            let mut sprite_iterable = self.memory[self.I as usize + row_iter as usize];
            let y_coord = (self.V[y as usize] + row_iter) % 32;

            for col_iter in 0..8 as u8 {
                let sprite_iteration = (sprite_iterable & 0x80) >> 7;
                let x_coord = (self.V[x as usize] + col_iter) % 64;
                if sprite_iteration == 1 {
                    match self.gfx[y_coord as usize][x_coord as usize] {
                        1 => {
                             self.gfx[y_coord as usize][x_coord as usize] = 0;
                             self.V[0xF] = 1;
                        },
                        _ => self.gfx[y_coord as usize][x_coord as usize] = 1,
                    }
                }
                sprite_iterable <<= 1;
            }
        }
    }  

    fn skp_vx(&mut self, x: u16) {
        if self.key[self.V[x as usize] as usize] == 1 {
            self.pc += 2;    
        }
    }

    fn sknp_vx(&mut self, x: u16) {
        if self.key[self.V[x as usize] as usize] == 0 {
            self.pc += 2;    
        }
    }

    fn ld_vx_dt(&mut self, x: u16) {
        self.V[x as usize] = self.delay_timer;
    }

    fn ld_vx_k(&mut self, x: u16) {
        let mut key_pressed = false;

        for i in 0..16 {
            if self.key[i] == 1 {
                self.V[x as usize] = i as u8;
                key_pressed = true;
            }
        }

        while !key_pressed {
            self.pc -= 2;
        }
    }

    fn ld_dt_vx(&mut self, x: u16) {
        self.delay_timer = self.V[x as usize];
        while self.delay_timer > 0 {
            self.delay_timer = self.delay_timer - 1;
            thread::sleep(time::Duration::from_millis(1000/60));
        }
    }

    fn ld_st_vx(&mut self, x: u16) {
        self.sound_timer = self.V[x as usize];
        while self.sound_timer > 0 {
            self.sound_timer = self.sound_timer - 1;
            thread::sleep(time::Duration::from_millis(1000/60));
        }
    }

    fn add_i_vx(&mut self, x: u16) {
        self.I += self.V[x as usize] as u16;
    }
    
    fn ld_f_vx(&mut self, x: u16) {
        self.I = self.V[x as usize] as u16 * 5;
    }

    fn ld_vx(&mut self, x: u16) {
        let vx = self.V[x as usize];
        let hundreds = vx / 100;
        let tens = (vx % 100) / 10;
        let ones = vx % 10;

        self.memory[self.I as usize] = hundreds as u8;
        self.memory[(self.I + 1) as usize] = tens as u8;
        self.memory[(self.I + 2) as usize] = ones as u8;
    }

    fn ld_i_vx(&mut self, x: u16) {
        for i in 0..=x as usize{
            self.memory[self.I as usize + i] = self.V[i];
        }
        // TODO: Maak aan of uitzetten van deze lijn code extern configureerbaar
        // self.I += x + 1;
    }

    fn ld_vx_i(&mut self, x: u16) {
        for i in 0..=x as usize{
            self.V[i] = self.memory[self.I as usize + i];
        }
        // TODO: idem vorige functie
        // self.I += x + 1;
    }

}

fn main() {
    const DISPLAY_WIDTH: u32 = 64;
    const DISPLAY_HEIGHT: u32 = 32;
    const PIXEL_SIZE: u32 = 10;

    let args: Vec<String> = env::args().collect();
    let filepath: &String = &args[1];

    let mut chip8: Chip8 = Chip8::new(filepath);

    let mut rw = RenderWindow::new(
        (DISPLAY_WIDTH * PIXEL_SIZE, DISPLAY_HEIGHT * PIXEL_SIZE),
        "CHIP-8",
        Style::CLOSE,
        &ContextSettings::default(),
    );

    let display: VideoMode = VideoMode::desktop_mode();
    let x_center: i32 = (display.width   / 2 - rw.size().x / 2) as i32;
    let y_center: i32 = (display.height  / 2 - rw.size().y / 2) as i32;

    rw.set_position(Vector2i::new(x_center, y_center));

    let keypad = [
            Key::NUM1, Key::NUM2, Key::NUM3, Key::NUM4,
            Key::Q,    Key::W,    Key::E,    Key::R,
            Key::A,    Key::S,    Key::D,    Key::F,
            Key::Z,    Key::X,    Key::C,    Key::V
        ];
    
    while rw.is_open() {
        chip8.clockcycle(CLOCK_SLEEP);

        while let Some(ev) = rw.poll_event() {
            match ev {
                Event::Closed => rw.close(),
                _ => {}
            }
        }

        for (pos, key) in keypad.iter().enumerate() {
            chip8.key[pos] = key.is_pressed() as u8;
        }

        let mut shape: RectangleShape = RectangleShape::new();

        for y in 0..DISPLAY_HEIGHT as usize {
            for x in 0..DISPLAY_WIDTH as usize {
                shape.set_size((PIXEL_SIZE as f32, PIXEL_SIZE as f32));
                shape.set_position((
                    x as f32 * PIXEL_SIZE as f32,
                    y as f32 * PIXEL_SIZE as f32,
                ));

                shape.set_fill_color(
                    match chip8.gfx[y][x] {
                        1 => Color::WHITE,
                        _ => Color::BLACK,
                });
                
                rw.draw(&shape);
            }
        }
        rw.display();
    }
}
