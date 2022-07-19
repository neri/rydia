use core::fmt::Write;

pub trait Uart {
    fn is_output_ready(&mut self) -> bool;

    fn is_input_ready(&mut self) -> bool;

    fn write_byte(&mut self, ch: u8);

    fn read_byte(&mut self) -> u8;
}

impl Write for dyn Uart {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            if ch == '\n' {
                self.write_byte('\r' as u8);
            }
            self.write_byte(ch as u8);
        }
        Ok(())
    }
}
