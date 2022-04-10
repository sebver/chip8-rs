extern crate sdl2;

use sdl2::{pixels::Color, rect::Rect, render::Canvas, video::Window, Sdl};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

const BLOCK_SIZE: u32 = 10;

struct Emulator {
    memory: [u8; 4096],
    pc: usize,
    display: [[bool; WIDTH]; HEIGHT],
    index_register: usize,
    var_registers: [u8; 16],
    stack: Vec<usize>,
}

impl Emulator {
    fn new() -> Emulator {
        Emulator {
            memory: [0; 4096],
            pc: 0x200,
            display: [[false; WIDTH]; HEIGHT],
            index_register: 0,
            var_registers: [0; 16],
            stack: Vec::new(),
        }
    }

    fn load_rom(&mut self, rom: Vec<u8>) -> &mut Self {
        for (i, b) in rom.into_iter().enumerate() {
            let idx = 0x200 + i;
            self.memory[idx] = b;
        }
        self
    }

    /// Returns true when display has changed, false otherwise.
    fn execute_current(&mut self) -> bool {
        let instruction = (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16;
        self.pc += 2;
        self.execute(instruction)
    }

    fn execute(&mut self, op: u16) -> bool {
        let nibbles = (
            (0xF000 & op) >> 12,
            (0x0F00 & op) >> 8,
            (0x00F0 & op) >> 4,
            0x000F & op,
        );
        let nnn = 0xFFF & op as usize;
        let nn = 0xFF & op as u8;
        let n = nibbles.3 as u8;
        let x = nibbles.1 as usize;
        let y = nibbles.2 as usize;
        match nibbles {
            (0x0, 0x0, 0xE, 0x0) => {
                self.display = [[false; WIDTH]; HEIGHT];
                true
            }
            (0x0, 0x0, 0xE, 0xE) => {
                self.pc = self.stack.pop().unwrap();
                false
            }
            (0x1, _, _, _) => {
                self.pc = nnn;
                false
            }
            (0x2, _, _, _) => {
                self.stack.push(self.pc);
                self.pc = nnn;
                false
            }
            (0x3, _, _, _) => {
                self.pc += if self.var_registers[x] == nn { 2 } else { 0 };
                false
            }
            (0x4, _, _, _) => {
                self.pc += if self.var_registers[x] != nn { 2 } else { 0 };
                false
            }
            (0x6, _, _, _) => {
                self.var_registers[x] = nn;
                false
            }
            (0x7, _, _, _) => {
                self.var_registers[x] += nn;
                false
            }
            (0x8, _, _, 0x0) => {
                self.var_registers[x] = self.var_registers[y];
                false
            }
            (0x8, _, _, 0x7) => {
                let (result, overflowing) =
                    self.var_registers[y].overflowing_sub(self.var_registers[x]);
                self.var_registers[x] = result;
                self.var_registers[0xF] = if overflowing { 0 } else { 1 };
                false
            }
            (0xA, _, _, _) => {
                self.index_register = nnn;
                false
            }
            (0xC, _, _, _) => {
                self.var_registers[x] = rand::random::<u8>() & nn;
                false
            }
            (0xD, _, _, _) => self.draw(x, y, n as usize),
            _ => todo!("{:>4X?}", op),
        }
    }

    fn draw(&mut self, x: usize, y: usize, height: usize) -> bool {
        let mut changed = false;
        let coord_x = (self.var_registers[x] % WIDTH as u8) as usize;
        let coord_y = (self.var_registers[y] % HEIGHT as u8) as usize;
        for (i, row) in (coord_y..coord_y + height).enumerate() {
            let sprite = self.memory[self.index_register + i];
            for (j, col) in (coord_x..coord_x + 8).enumerate() {
                if col < WIDTH {
                    let pixel = &mut self.display[row][col];
                    let sprite_pixel = 1 & (sprite >> (7 - j)) == 1;
                    if sprite_pixel != *pixel {
                        *pixel = !*pixel;
                        changed = true;
                        if !*pixel {
                            self.var_registers[0xF] = 1;
                        }
                    }
                }
            }
        }

        changed
    }

    fn debug_display(&self) {
        print!("{}[2J", 27 as char); // clear screen
        for r in 0..HEIGHT {
            print!("[{:0>2}]: ", r);
            for c in 0..WIDTH {
                print!("{}", if self.display[r][c] { '#' } else { ' ' });
            }
            print!("\n");
        }
    }
}

fn main() {
    let rom = std::fs::read("rom/br8kout.ch8").unwrap();
    let mut emulator = Emulator::new();

    emulator.load_rom(rom);

    let sdl_context = sdl2::init().unwrap();
    let mut canvas = create_canvas(
        &sdl_context,
        WIDTH as u32 * BLOCK_SIZE,
        HEIGHT as u32 * BLOCK_SIZE,
    )
    .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        for _ in event_pump.poll_iter() {
            // Do something
        }
        if emulator.execute_current() {
            // emulator.debug_display();
            draw_canvas(&mut canvas, &emulator.display);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn create_canvas(sdl_context: &Sdl, width: u32, height: u32) -> Result<Canvas<Window>, String> {
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("CHIP-8 emulator!", width, height)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    Ok(canvas)
}

fn draw_canvas(canvas: &mut Canvas<Window>, pixels: &[[bool; WIDTH]; HEIGHT]) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    for (i, row) in pixels.iter().enumerate() {
        for (j, col) in row.iter().enumerate() {
            if *col {
                let rect = Rect::new(
                    (BLOCK_SIZE * j as u32) as i32,
                    (BLOCK_SIZE * i as u32) as i32,
                    BLOCK_SIZE,
                    BLOCK_SIZE,
                );
                canvas.fill_rect(rect).unwrap();
            }
        }
    }
    canvas.present();
}
