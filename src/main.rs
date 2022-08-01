use sfml::{
    graphics::{
        Color, Rect, RectangleShape, RenderTarget, RenderWindow, Shape, Transformable,
    },
    system::Vector2,
    window::{ContextSettings, Event, Style, Key},
};

use std::{env, fs::File, io::Read, thread, time};
use rand::Rng;


struct Chip8 {
    pub opcode: u16,
    pub memory: [u8; 4096],
    V: [u8; 16],
    I: u16,
    pc: u16,
    pub gfx: [[u8; 64]; 32],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    sp: u16,
    key: [u8; 16],
    pub draw_enabled: bool
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

const DRAW_AREA_TOPLEFT: (u16, u16) = (0, 0);

const DP_WIDTH: u8 = 64;
const DP_HEIGHT: u8 = 32;
const PIXEL_WH: u8 = 10;


fn rom_from_file(filepath: &str) -> ([u8; 4096], usize) {
    let mut f = File::open(filepath).expect("File not found");
    let mut buf = [0u8; 4096];

    let bytes_read = match f.read(&mut buf) {
        Ok(bytes_read) => { bytes_read }
        Err(_) => 0,
    };

    (buf, bytes_read)
}

fn parse_nr(xth: u16, opcode: u16) -> u16 {
    match xth {
        1 => opcode >> 12,
        2 => opcode << 4 >> 12,
        3 => opcode << 8 >> 12,
        4 => opcode << 12 >> 12,
        _ => 0x0,
    }
}

fn parse_last_nrs(amount: u16, opcode: u16) -> u16 {
    match amount {
        2 => opcode << 8 >> 8,
        3 => opcode << 4 >> 4,
        _ => 0,
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
            key: [0; 16],
            draw_enabled: false
        }

    }    

    
    pub fn clockcycle(&mut self) {
        // fetch
        let mem_pc1 = self.memory[self.pc as usize] as u16;
        let mem_pc2 = self.memory[self.pc as usize + 1] as u16;
        self.opcode = mem_pc1 << 8 | mem_pc2;
        self.pc += 2;

        let oc = self.opcode;

        let first_num = parse_nr(1, oc);
        let second_num = parse_nr(2, oc);
        let third_num = parse_nr(3, oc);
        let last_num = parse_nr(4, oc);
        let last_two = parse_last_nrs(2, oc);
        let last_three = parse_last_nrs(3, oc);

        // println!("{oc:x} {first_num:x} {second_num:x} {third_num:x} {last_num:x} {last_two:x} {last_three:x}");

        match first_num {
            0x0 => {match self.opcode {
                0x00e0 => self.cls(),
                0x00ee => self.ret(),
                _ => {},
            }},
            0x1 => self.jmp(last_three),
            0x2 => self.call(last_three),
            0x3 => self.se_vx_b(second_num, last_two),
            0x4 => self.sne_vx_b(second_num, last_two),
            0x5 => self.se_vx_vy(second_num, third_num),
            0x6 => self.ld_vx_b(second_num, last_two),
            0x7 => self.add_vx_b(second_num, last_two),
            0x8 => {
                match last_num {
                    0x0 => self.ld_vx_vy(second_num, third_num),
                    0x1 => self.or(second_num, third_num),
                    0x2 => self.and(second_num, third_num),
                    0x3 => self.xor(second_num, third_num),
                    0x4 => self.add_vf(second_num, third_num),
                    0x5 => self.sub(second_num, third_num),
                    0x6 => self.shr(second_num),
                    0x7 => self.subn(second_num, third_num),
                    0xe => self.shl(second_num),
                    _ => {},
                }
            },
            0x9 => self.sne(second_num, third_num),

            0xA => self.ld_i_a(last_three),
            0xB => self.jmp_v0_a(last_three),
            0xC => self.rnd(second_num, last_two),
            0xD => self.draw(second_num, third_num, last_num),
            0xE => {
                match last_two {
                    0x9E => self.skp_vx(second_num),
                    0xA1 => self.sknp_vx(second_num),
                    _ => {},
                }
            },
            0xF => {
                match last_two {
                    0x07 => self.ld_vx_dt(second_num),
                    0x0A => self.ld_vx_k(second_num),
                    0x15 => self.ld_dt_vx(second_num),
                    0x18 => self.ld_st_vx(second_num),
                    0x1E => self.add_i_vx(second_num),
                    0x29 => self.ld_f_vx(second_num),
                    0x33 => self.ld_vx(second_num),
                    0x55 => self.ld_i_vx(second_num),
                    0x65 => self.ld_vx_i(second_num),
                    _ => {},
                }
            },
            _ => {},
        }       
        // thread::sleep(time::Duration::from_millis(1));
    }

    fn cls(&mut self) {
        self.gfx = [[0; 64]; 32]
    }

    fn ret(&mut self) {
        self.pc = self.stack[self.sp as usize];
        self.sp -= 1;
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
        self.V[x as usize] += kk as u8;
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
        self.V[x as usize] -= self.V[y as usize];
    }

    fn shr(&mut self, x: u16) {
        self.V[0xF] = self.V[x as usize] & 1;
        self.V[x as usize] >>= 1;
    }

    fn subn(&mut self, x: u16, y: u16) {
        self.V[0xF] = if self.V[y as usize] > self.V[y as usize] {1} else {0};
        self.V[x as usize] -= self.V[y as usize];
    }

    fn shl(&mut self, x: u16) {
        self.V[0xF] = self.V[x as usize] & 0x80 >> 7;
        self.V[x as usize] *= 2;
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
        let x_coord = self.V[x as usize]%64;
            let y_coord = self.V[y as usize]%32;
            let height = n as u8;

            self.V[0xF] = 0;

            for ydir in 0..height {
                let pixel = self.memory[self.I as usize + ydir as usize];
                for xdir in 0..8 {
                    let xdraw = xdir as usize + x_coord as usize;
                    let ydraw = ydir as usize + y_coord as usize;
                    if pixel & (0x80 >> xdir) != 0 {
                        if self.gfx[ydraw][xdraw] == 1 {
                            self.V[0xF] = 1;
                        } 
                        self.gfx[ydraw][xdraw] ^= 1;
                    }
                }
            }
            // for y in 0..32 {
            //     for x in 0..64 {
            //         let a = self.gfx[y][x];
            //         let b = if a == 0 {"."} else {"*"};
            //         print!("{b}");
            //     }
            //     println!("");
            // }
            // println!("\n");

            self.draw_enabled = true;
            // self.pc += 2;
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

        while !key_pressed {
            for i in 0..16 {
                if self.key[i] == 1 {
                    self.V[x as usize] = self.key[i];
                    key_pressed = true;
                }
            }
        }
    }

    fn ld_dt_vx(&mut self, x: u16) {
        self.delay_timer = self.V[x as usize];
    }

    fn ld_st_vx(&mut self, x: u16) {
        self.sound_timer = self.V[x as usize];
    }

    fn add_i_vx(&mut self, x: u16) {
        self.I += self.V[x as usize] as u16;
    }
    
    fn ld_f_vx(&mut self, x: u16) {
        self.I = self.V[x as usize] as u16 * 5;
    }

    fn ld_vx(&mut self, x: u16) {
        let hundreds = x / 100;
        let tens = x / 10 % 10;
        let ones = x % 100 % 10;

        self.memory[self.I as usize] = hundreds as u8;
        self.memory[self.I as usize + 1] = tens as u8;
        self.memory[self.I as usize + 2] = ones as u8;
    }
git 
    fn ld_i_vx(&mut self, x: u16) {
        for i in 0..=x as usize{
            self.memory[self.I as usize + i] = self.V[i];
        }
    }

    fn ld_vx_i(&mut self, x: u16) {
        for i in 0..=x as usize{
            self.V[i] = self.memory[self.I as usize + i];
        }
    }

}




fn main() {
    let args: Vec<String> = env::args().collect();

    let filepath = &args[1];

    let mut chip8 = Chip8::new(filepath);





    let mut rw = RenderWindow::new(
        (640, 320),
        "CHIP8",
        Style::CLOSE,
        &ContextSettings::default(),
    );

    while rw.is_open() {
        while let Some(ev) = rw.poll_event() {
            match ev {
                Event::Closed => rw.close(),
                _ => {}
            }
        }

        chip8.clockcycle();

        let keypad = [
            Key::NUM1, Key::NUM2, Key::NUM3, Key::NUM4,
            Key::Q,    Key::W,    Key::E,    Key::R,
            Key::A,    Key::S,    Key::D,    Key::F,
            Key::Z,    Key::X,    Key::C,    Key::V
        ];

        for i in 0..16 {
            chip8.key[i] = keypad[i].is_pressed() as u8;
        }

        let aa = chip8.key[0];
        println!("{aa}");

        rw.clear(Color::BLACK);

        let mut shape = RectangleShape::default();
        shape.set_fill_color(Color::TRANSPARENT);

        for y in 0..32 {
            for x in 0..64 {


                // shape.set_outline_color(Color::WHITE);
                // shape.set_outline_thickness(1.0);
                shape.set_fill_color(Color::BLACK);
                shape.set_size((PIXEL_WH as f32, PIXEL_WH as f32));
                shape.set_position((
                    DRAW_AREA_TOPLEFT.0 as f32 + (x as f32 * PIXEL_WH as f32),
                    DRAW_AREA_TOPLEFT.1 as f32 + (y as f32 * PIXEL_WH as f32),
                ));
                if chip8.gfx[y][x] == 1 {
                    shape.set_fill_color(Color::WHITE);
                }
                rw.draw(&shape);
            }
        }
        rw.display();
    }
}