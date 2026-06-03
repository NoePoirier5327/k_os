//! Module d'affichage de caractère dans le buffer vga. <br>
//! Code tiré du tutoriel de Philipp Opermann.<br>
//! <https://os.phil-opp.com/vga-text-mode/>

// TODO Gérer l'affichage des accents de UTF-8 vers CP437

use core::fmt;
use spin::Lazy;
use spin::Mutex;
use volatile::Volatile;


/// Type énuméré représentant les couleurs affichables à l'écran par le buffer vga.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}


/// Structure de donnée représentant le couple de couleur d'un caractère affichable à l'écran. <br>
/// Composé d'une couleur pour le caractère et d'une couleur pour le fond.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Fonction de création de couples de couleurs pour les caractères d'une string.
    ///
    /// # Arguments
    /// * `foreground` : couleur d'affichage du caractère.
    /// * `background` : couleur d'affichage du fond du caractère.
    ///
    /// # Return
    /// Instance de color_code.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}


/// Structure de donnée représentant un caractère affichable à l'écran.<br>
/// Elle est composée d'un caractère à afficher et de sa couleur sur l'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// Structure représentant la matrice de caractère composant l'écran.
#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// Structure de contrôle de l'affichage de caractères.
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    /// Fonction gérant le cas où l'on rencontre le caractère de nouvelle ligne.<br>
    /// Elle remonte l'affichage et supprime la ligne qui sort du buffer..
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Fonction de supression de ligne dans l'affichage.
    ///
    /// # Argument
    /// * `row` : ligne à vider
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// Fonction d'affichage d'un caractère sur le buffer vga.<br>
    /// Si on dépasse la plage d'affichage du buffer courant, on se place sur une nouvelle ligne
    /// et on affiche.
    ///
    /// # Argument
    /// * `byte` : caractère à afficher.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    /// Fonction d'affichage de chaines de caractères sur le buffer courant.
    ///
    /// # Argument
    /// * `s` : chaine à afficher.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }
}

/// Ajout du support des macros write! et writeln! pour qu'elles fonctionnent
/// avec le code d'affichage dans le buffer vga.
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

/// Interface d'ecriture globale dans le buffer vga. <br>
/// Elle est chargée en tant que static à partir du moment où le processeur l'appelle
/// et elle utilise une sémaphore pour bloquer son accès à chaques utilisations.
pub static WRITER: Lazy<Mutex<Writer>> = Lazy::new(|| {
    Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightGray, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    })
});

/// Support de la macro print! de la librairie standard de rust.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Support de la macro println! de la librairie standard de rust.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Fonction appelée pour les print! et println!. <br>
/// Mobilise la mémoire gérée par le mutex de l'interface du writer.
/// Tant que le mutex est lock, aucune interruption processeur ne peut avoir lieu.
///
/// # Argument
/// * `args` : arguments pour l'affichage dans le writer.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}
