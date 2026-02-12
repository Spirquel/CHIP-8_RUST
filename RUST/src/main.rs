use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::process;
use std::time::Duration;
use std::thread;

// On importe le module qu'on a créé à l'étape précédente
mod chip8;
use chip8::Chip8;

const SCREEN_WIDTH: u32 = 1024;
const SCREEN_HEIGHT: u32 = 512;
const CHIP8_WIDTH: usize = 64;
const CHIP8_HEIGHT: usize = 32;

// Mapping des touches (Clavier moderne -> Hex Keypad Chip8)
// C'est l'équivalent de ton tableau 'keymap' en C++, mais on utilise une fonction
// pour faire la conversion proprement sans risque de dépassement.
fn key2btn(key: Keycode) -> Option<usize> {
    match key {
        Keycode::X => Some(0x0),
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::Z => Some(0xA),
        Keycode::C => Some(0xB),
        Keycode::Num4 => Some(0xC),
        Keycode::R => Some(0xD),
        Keycode::F => Some(0xE),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

fn main() {
    // 1. Gestion des arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: cargo run <ROM file>");
        process::exit(1);
    }
    let rom_path = &args[1];

    // 2. Initialisation de SDL2
    // En Rust, SDL est découpé en sous-systèmes pour gérer l'ownership.
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("CHIP-8 Emulator (Rust)", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, CHIP8_WIDTH as u32, CHIP8_HEIGHT as u32)
        .unwrap();

    // 3. Initialisation du Chip8
    let mut chip8 = Chip8::new();
    
    // Gestion propre de l'erreur de chargement (Result)
    if let Err(e) = chip8.load(rom_path) {
        eprintln!("Erreur lors du chargement de la ROM: {}", e);
        process::exit(2);
    }

    let mut event_pump = sdl_context.event_pump().unwrap();

    // 4. Boucle principale (Game Loop)
    'gameloop: loop {
        // --- Émulation ---
        chip8.emulate_cycle();

        // --- Gestion des événements (Input) ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'gameloop;
                },
                // Remplacement du "goto load" par une réinitialisation propre
                Event::KeyDown { keycode: Some(Keycode::F1), .. } => {
                    println!("F1 pressed: Resetting game...");
                    chip8 = Chip8::new();
                    if let Err(e) = chip8.load(rom_path) {
                         eprintln!("Erreur critique au rechargement: {}", e);
                         break 'gameloop;
                    }
                },
                Event::KeyDown { keycode: Some(key), .. } => {
                    if let Some(i) = key2btn(key) {
                        chip8.key[i] = 1;
                    }
                },
                Event::KeyUp { keycode: Some(key), .. } => {
                    if let Some(i) = key2btn(key) {
                        chip8.key[i] = 0;
                    }
                },
                _ => {}
            }
        }

        // --- Rendu Graphique ---
        if chip8.draw_flag {
            chip8.draw_flag = false;

            // Conversion du buffer monochrome (1 bit) en pixels ARGB (32 bits)
            // On utilise un vecteur temporaire.
            // Le format ARGB8888 demande 4 octets par pixel : B, G, R, A (selon l'endianness)
            // C++ faisait : (0x00FFFFFF * pixel) | 0xFF000000
            
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..CHIP8_HEIGHT {
                    for x in 0..CHIP8_WIDTH {
                        let offset = y * pitch + x * 4;
                        let pixel = chip8.gfx[y * 64 + x];
                        
                        // Couleur : Blanc (255, 255, 255) ou Noir (0, 0, 0)
                        let color_val = if pixel != 0 { 255 } else { 0 };

                        buffer[offset] = color_val;      // B
                        buffer[offset + 1] = color_val;  // G
                        buffer[offset + 2] = color_val;  // R
                        buffer[offset + 3] = 255;        // A (Toujours opaque)
                    }
                }
            }).unwrap();

            canvas.clear();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }

        // --- Temporisation ---
        // Remplace std::this_thread::sleep_for
        thread::sleep(Duration::from_micros(1200));
    }
}