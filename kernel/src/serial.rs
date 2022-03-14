pub struct EmergencySerial;

impl EmergencySerial {
    pub fn write(&mut self, bytes: &[u8]) {
        bytes.iter().for_each(|&b| unsafe {
            while {
                let ret: u8;
                core::arch::asm!("in al, dx", in("dx") 0x3f8 + 5, out("al") ret);
                ret
            } & 0x20
                == 0
            {}
            core::arch::asm!("out dx, al", in("dx") 0x3f8, in("al") b);
        })
    }
}

impl core::fmt::Write for EmergencySerial {
    fn write_str(&mut self, string: &str) -> core::fmt::Result {
        self.write(string.as_bytes());
        Ok(())
    }
}
