use rand::Rng; // Nécessaire pour l'instruction CXNN (aléatoire)
use std::fs::File;
use std::io::Read;

// Le set de police (inchangé par rapport au C++)
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    // Composants internes (privés comme en C++)
    opcode: u16,
    memory: [u8; 4096],
    v: [u8; 16],      // Registres V0-VF
    i: u16,           // Index register
    pc: u16,          // Program counter
    
    stack: [u16; 16],
    sp: u16,          // Stack pointer

    pub delay_timer: u8,
    pub sound_timer: u8,

    // Composants publics (accessibles par le main)
    // En Rust, on préfère souvent des getters, mais pour rester proche
    // de ton code C++ (gfx public), on les met 'pub'.
    pub gfx: [u8; 64 * 32],
    pub key: [u8; 16],
    pub draw_flag: bool,
}

impl Chip8 {
    // Constructeur : remplace Chip8() et init()
    pub fn new() -> Self {
        let mut c = Chip8 {
            opcode: 0,
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200, // Le PC commence à 512
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            gfx: [0; 64 * 32],
            key: [0; 16],
            draw_flag: false,
        };

        // Charger la police en mémoire (0x000 à 0x050)
        for i in 0..80 {
            c.memory[i] = FONTSET[i];
        }

        c
    }

    // Chargement de la ROM
    // Différence majeure : On retourne un Result pour gérer les erreurs proprement
    // au lieu de return true/false et fprintf.
    pub fn load(&mut self, file_path: &str) -> std::io::Result<()> {
        println!("Loading ROM: {}", file_path);

        let mut file = File::open(file_path)?;
        let mut buffer = Vec::new();

        // Lecture du fichier entier dans un buffer dynamique (pas de malloc manuel !)
        file.read_to_end(&mut buffer)?;

        let rom_size = buffer.len();
        if rom_size > (4096 - 512) {
            panic!("ROM too large to fit in memory"); // Ou retourner une erreur custom
        }

        // Copie dans la mémoire du Chip8
        for (i, &byte) in buffer.iter().enumerate() {
            self.memory[i + 512] = byte;
        }

        Ok(())
    }

    pub fn emulate_cycle(&mut self) {
        // Fetch Opcode
        // En Rust, on doit caster en u16 explicitement pour le shift
        let pc = self.pc as usize;
        let op_byte1 = self.memory[pc] as u16;
        let op_byte2 = self.memory[pc + 1] as u16;
        
        self.opcode = (op_byte1 << 8) | op_byte2;

        // Décodage
        // On utilise 'match' sur le quartet de poids fort (ex: 0xA2F0 & 0xF000 => 0xA000)
        match self.opcode & 0xF000 {
            0x0000 => {
                match self.opcode & 0x000F {
                    0x0000 => { // 00E0: Clear screen
                        self.gfx = [0; 64 * 32];
                        self.draw_flag = true;
                        self.pc += 2;
                    },
                    0x000E => { // 00EE: Return from subroutine
                        self.sp -= 1;
                        self.pc = self.stack[self.sp as usize];
                        self.pc += 2;
                    },
                    _ => panic!("Unknown opcode [0x0000]: {:X}", self.opcode),
                }
            },
            0x1000 => { // 1NNN: Jump
                self.pc = self.opcode & 0x0FFF;
            },
            0x2000 => { // 2NNN: Call subroutine
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = self.opcode & 0x0FFF;
            },
            0x3000 => { // 3XNN: Skip if VX == NN
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let nn = (self.opcode & 0x00FF) as u8;
                if self.v[x] == nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            0x4000 => { // 4XNN: Skip if VX != NN
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let nn = (self.opcode & 0x00FF) as u8;
                if self.v[x] != nn {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            0x5000 => { // 5XY0: Skip if VX == VY
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let y = ((self.opcode & 0x00F0) >> 4) as usize;
                if self.v[x] == self.v[y] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            0x6000 => { // 6XNN: Set VX = NN
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                self.v[x] = (self.opcode & 0x00FF) as u8;
                self.pc += 2;
            },
            0x7000 => { // 7XNN: Add NN to VX (No carry flag)
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let nn = (self.opcode & 0x00FF) as u8;
                // wrapping_add simule le comportement du C++ (overflow silencieux)
                self.v[x] = self.v[x].wrapping_add(nn);
                self.pc += 2;
            },
            0x8000 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let y = ((self.opcode & 0x00F0) >> 4) as usize;
                
                match self.opcode & 0x000F {
                    0x0000 => { self.v[x] = self.v[y]; self.pc += 2; },
                    0x0001 => { self.v[x] |= self.v[y]; self.pc += 2; },
                    0x0002 => { self.v[x] &= self.v[y]; self.pc += 2; },
                    0x0003 => { self.v[x] ^= self.v[y]; self.pc += 2; },
                    0x0004 => { // Add with carry
                        let (res, overflow) = self.v[x].overflowing_add(self.v[y]);
                        self.v[0xF] = if overflow { 1 } else { 0 };
                        self.v[x] = res;
                        self.pc += 2;
                    },
                    0x0005 => { // Sub with borrow
                        let (res, overflow) = self.v[x].overflowing_sub(self.v[y]);
                        // En C++ tu avais : if VY > VX ... borrow
                        // Rust overflowing_sub retourne true s'il y a eu "wrap around" (donc un borrow)
                        // Attention : Chip8 spec dit VF = 0 si borrow, 1 si pas borrow.
                        // Si overflow est true, ça veut dire v[x] < v[y], donc borrow.
                        self.v[0xF] = if overflow { 0 } else { 1 };
                        self.v[x] = res;
                        self.pc += 2;
                    },
                    0x0006 => { // Shift Right
                        self.v[0xF] = self.v[x] & 0x1;
                        self.v[x] >>= 1;
                        self.pc += 2;
                    },
                    0x0007 => { // SubN (VY - VX)
                        let (res, overflow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[0xF] = if overflow { 0 } else { 1 };
                        self.v[x] = res;
                        self.pc += 2;
                    },
                    0x000E => { // Shift Left
                        self.v[0xF] = (self.v[x] >> 7) & 1;
                        self.v[x] <<= 1;
                        self.pc += 2;
                    },
                    _ => panic!("Unknown opcode [0x8000]: {:X}", self.opcode),
                }
            },
            0x9000 => { // 9XY0: Skip if VX != VY
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let y = ((self.opcode & 0x00F0) >> 4) as usize;
                if self.v[x] != self.v[y] {
                    self.pc += 4;
                } else {
                    self.pc += 2;
                }
            },
            0xA000 => { // ANNN: Set I
                self.i = self.opcode & 0x0FFF;
                self.pc += 2;
            },
            0xB000 => { // BNNN: Jump to NNN + V0
                let nnn = self.opcode & 0x0FFF;
                self.pc = nnn + (self.v[0] as u16);
            },
            0xC000 => { // CXNN: Random
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                let nn = (self.opcode & 0x00FF) as u8;
                let rng: u8 = rand::random(); // Génération aléatoire
                self.v[x] = rng & nn;
                self.pc += 2;
            },
            0xD000 => { // DXYN: Draw
                let x = self.v[((self.opcode & 0x0F00) >> 8) as usize] as u16;
                let y = self.v[((self.opcode & 0x00F0) >> 4) as usize] as u16;
                let height = self.opcode & 0x000F;
                
                self.v[0xF] = 0;
                
                for yline in 0..height {
                    let pixel = self.memory[(self.i + yline) as usize];
                    for xline in 0..8 {
                        if (pixel & (0x80 >> xline)) != 0 {
                            let idx = (x + xline + ((y + yline) * 64)) as usize;
                            // Sécurité: on évite de sortir du tableau gfx
                            if idx < self.gfx.len() {
                                if self.gfx[idx] == 1 {
                                    self.v[0xF] = 1;
                                }
                                self.gfx[idx] ^= 1;
                            }
                        }
                    }
                }
                self.draw_flag = true;
                self.pc += 2;
            },
            0xE000 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                match self.opcode & 0x00FF {
                    0x009E => { // Key pressed
                        if self.key[self.v[x] as usize] != 0 {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    },
                    0x00A1 => { // Key not pressed
                        if self.key[self.v[x] as usize] == 0 {
                            self.pc += 4;
                        } else {
                            self.pc += 2;
                        }
                    },
                    _ => panic!("Unknown opcode [0xE000]: {:X}", self.opcode),
                }
            },
            0xF000 => {
                let x = ((self.opcode & 0x0F00) >> 8) as usize;
                match self.opcode & 0x00FF {
                    0x0007 => { self.v[x] = self.delay_timer; self.pc += 2; },
                    0x000A => { // Wait for key
                         let mut key_pressed = false;
                         for i in 0..16 {
                             if self.key[i] != 0 {
                                 self.v[x] = i as u8;
                                 key_pressed = true;
                             }
                         }
                         if !key_pressed {
                             return; // On sort sans augmenter PC, donc on rejoue l'instruction
                         }
                         self.pc += 2;
                    },
                    0x0015 => { self.delay_timer = self.v[x]; self.pc += 2; },
                    0x0018 => { self.sound_timer = self.v[x]; self.pc += 2; },
                    0x001E => { // Add to I
                        if self.i + (self.v[x] as u16) > 0xFFF {
                            self.v[0xF] = 1;
                        } else {
                            self.v[0xF] = 0;
                        }
                        self.i += self.v[x] as u16;
                        self.pc += 2;
                    },
                    0x0029 => { // Font char
                        self.i = (self.v[x] as u16) * 5;
                        self.pc += 2;
                    },
                    0x0033 => { // BCD
                        self.memory[self.i as usize] = self.v[x] / 100;
                        self.memory[(self.i + 1) as usize] = (self.v[x] / 10) % 10;
                        self.memory[(self.i + 2) as usize] = self.v[x] % 10;
                        self.pc += 2;
                    },
                    0x0055 => { // Dump Regs
                        for i in 0..=x {
                            self.memory[(self.i as usize) + i] = self.v[i];
                        }
                        self.i += (x as u16) + 1;
                        self.pc += 2;
                    },
                    0x0065 => { // Load Regs
                        for i in 0..=x {
                            self.v[i] = self.memory[(self.i as usize) + i];
                        }
                        self.i += (x as u16) + 1;
                        self.pc += 2;
                    },
                    _ => panic!("Unknown opcode [0xF000]: {:X}", self.opcode),
                }
            },
            _ => panic!("Unknown opcode: {:X}", self.opcode),
        }

        // Timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // Sound logic (not implemented in original)
            }
            self.sound_timer -= 1;
        }
    }
}
