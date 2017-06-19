extern crate memmap;
extern crate byteorder;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use memmap::{Mmap, Protection};
use std::fs::OpenOptions;
use std::io::Result;

const DEV: &'static str = "/dev/gpiomem";
const PULLUPDNCLK_OFFSET: u32 = 38; // 0x0098 / 4
const PULLUPDN_OFFSET: u32 = 37; // 0x0094 / 4
const FSEL_OFFSET: u32 = 0; // 0x0000
const SET_OFFSET: u32 = 7; // 0x001c / 4
const CLR_OFFSET: u32 = 10; // 0x0028 / 4
const PINLEVEL_OFFSET: u32 = 13; // 0x0034 / 4
const BLOCK_SIZE: usize = 4 * 1024;

macro_rules! buf_read {
    ($buf:expr, $offset:expr) => ({
        let offset = $offset as usize * 4;
        (&$buf[offset..offset + 4]).read_u32::<LittleEndian>()
    })
}

macro_rules! buf_write {
    ($buf:expr, $offset:expr, $value:expr) => ({
        let offset = $offset as usize * 4;
        (&mut $buf[offset..offset + 4]).write_u32::<LittleEndian>($value)
    })
}

pub enum Pud {
    Off,
    Down,
    Up,
}

impl Into<u32> for Pud {
    fn into(self) -> u32 {
        match self {
            Pud::Off => 0,
            Pud::Down => 1,
            Pud::Up => 2,
        }
    }
}

pub enum Direction {
    Output,
    Input,
}

impl Into<u32> for Direction {
    fn into(self) -> u32 {
        match self {
            Direction::Output => 1,
            Direction::Input => 0,
        }
    }
}

pub enum Status {
    High,
    Low,
}

impl From<u32> for Status {
    fn from(v: u32) -> Status {
        match v {
            0 => Status::Low,
            _ => Status::High,
        }
    }
}

pub struct Gpio {
    map: Mmap,
}

impl Gpio {
    pub fn from_gpiomem() -> Result<Gpio> {
        let file = OpenOptions::new().read(true).write(true).open(DEV)?;
        Ok(Gpio {
               map: Mmap::open_with_offset(&file, Protection::ReadWrite, 0, BLOCK_SIZE)
                   .expect("aaa"),
           })
    }

    fn set_pullupdn(&mut self, pin: u32, pud: Pud) -> Result<()> {
        let clk_offset = PULLUPDNCLK_OFFSET + pin / 32;
        let shift = pin % 32;

        let mut buf = unsafe { self.map.as_mut_slice() };

        let orig = buf_read!(buf, PULLUPDN_OFFSET)?;
        let pud: u32 = pud.into();
        buf_write!(buf, PULLUPDN_OFFSET, (orig & !3) | pud)?;
        buf_write!(buf, clk_offset, 1 << shift)?;
        let value = buf_read!(buf, PULLUPDN_OFFSET)?;
        buf_write!(buf, PULLUPDN_OFFSET, value & !3)?;
        buf_write!(buf, clk_offset, 0)
    }

    pub fn setup(&mut self, pin: u32, direction: Direction, pud: Pud) -> Result<()> {
        let offset = FSEL_OFFSET + pin / 10;
        let shift = (pin % 10) * 3;
        self.set_pullupdn(pin, pud)?;

        let mut buf = unsafe { self.map.as_mut_slice() };
        let orig = buf_read!(buf, offset)?;
        let direction: u32 = direction.into();
        buf_write!(buf, offset, (orig & !(7 << shift)) | (direction << shift))?;
        Ok(())
    }

    pub fn output(&mut self, pin: u32, status: Status) -> Result<()> {
        let offset = match status {
            Status::High => SET_OFFSET + pin / 32,
            Status::Low => CLR_OFFSET + pin / 32,
        };
        let shift = pin % 32;
        let mut buf = unsafe { self.map.as_mut_slice() };
        buf_write!(buf, offset, 1 << shift)?;
        Ok(())
    }

    pub fn input(&mut self, pin: u32) -> Result<Status> {
        let offset = PINLEVEL_OFFSET + pin / 32;
        let mask = 1 << (pin % 32);
        let buf = unsafe { self.map.as_mut_slice() };
        let orig = buf_read!(buf, offset)?;
        Ok(From::from(orig & mask))
    }
}
