//! Module de formatage des messages du noyau vers l'utilisateur.
// TODO Implémenter les logs dans un fichier de l'os à chaque utilisation des macros de message.

/// Afficheur de message d'information vers l'utilisateur.
#[macro_export]
macro_rules! disp_info {
    ($($args:tt)*) => {
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("[");
        $crate::vga_buffer::set_writer_color($crate::vga_buffer::Color::Green, $crate::vga_buffer::Color::Black);
        $crate::print!("INFO");
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("] : ");
        $crate::println!($($args)*);
    }
}

/// Afficheur de message de warning vers l'utilisateur.
#[macro_export]
macro_rules! disp_warning {
    ($($args:tt)*) => {
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("[");
        $crate::vga_buffer::set_writer_color($crate::vga_buffer::Color::Yellow, $crate::vga_buffer::Color::Black);
        $crate::print!("WARNING");
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("] : ");
        $crate::println!($($args)*);
    }
}

/// Afficheur de message d'erreur vers l'utilisateur.
#[macro_export]
macro_rules! disp_error {
    ($($args:tt)*) => {
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("[");
        $crate::vga_buffer::set_writer_color($crate::vga_buffer::Color::Yellow, $crate::vga_buffer::Color::Black);
        $crate::print!("ERROR");
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("] : ");
        $crate::println!($($args)*);
    }
}

/// Afficheur de message d'exception vers l'utilisateur.
#[macro_export]
macro_rules! disp_exception {
    ($($args:tt)*) => {
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("[");
        $crate::vga_buffer::set_writer_color($crate::vga_buffer::Color::Blue, $crate::vga_buffer::Color::LightGray);
        $crate::print!("EXCEPTION");
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("] : ");
        $crate::println!($($args)*);
    }
}

/// Afficheur de message de débug vers l'utilisateur
// TODO Faire en sorte que le message de disp_debug ne s'affiche qu'en mode debug.
#[macro_export]
macro_rules! disp_debug {
    ($($args:tt)*) => {
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("[");
        $crate::vga_buffer::set_writer_color($crate::vga_buffer::Color::Blue, $crate::vga_buffer::Color::Black);
        $crate::print!("DEBUG");
        $crate::vga_buffer::set_default_writer_color();
        $crate::print!("] : ");
        $crate::println!($($args)*);
    }
}
