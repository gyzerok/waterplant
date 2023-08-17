use core::marker::PhantomData;

use embedded_hal_async::delay::DelayUs;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

// const CFG_MODE_4: u8 = 0x00;
// const CFG_MODE_8: u8 = 0x10;
const CFG_1LINE: u8 = 0x00;
const CFG_2LINE: u8 = 0x08;
const CFG_5X8DOTS: u8 = 0x00;
const CFG_5X10DOTS: u8 = 0x04;

const MODE_CMD: u8 = 0x00;
const MODE_DATA: u8 = 0x01;

const BACKLIGHT_ON: u8 = 0x08;

const CMD_CLEAR_DISPLAY: u8 = 0x01;
const CMD_RETURN_HOME: u8 = 0x02;
const CMD_ENTRY_MODE_SET: u8 = 0x04;
const CMD_DISPLAY_CONTROL: u8 = 0x08;
const CMD_SHIFT_CURSOR: u8 = 0x10;
const CMD_FUNCTION_SET: u8 = 0x20;
const CMD_SET_CGRAM_ADDR: u8 = 0x40;
const CMD_SET_DDRAM_ADDR: u8 = 0x80;

const DISPLAY_CURSOR_BLINK_ON: u8 = 0x01;
const DISPLAY_CURSOR_ON: u8 = 0x02;
const DISPLAY_ON: u8 = 0x04;

const ENTRY_LEFT: u8 = 0x02;
// const ENTRY_SHIFT_INCREMENT: u8 = 0x01;

const MOVE_DISPLAY: u8 = 0x08;
const MOVE_RIGHT: u8 = 0x04;

pub struct Lcd<State, I: I2c<SevenBitAddress>, D: DelayUs> {
    i2c: I,
    delay: D,
    addr: SevenBitAddress,
    rows: u8,
    cell_size: u8,
    backlight: u8,
    display_control: u8,
    entry_mode: u8,
    _marker: PhantomData<State>,
}

pub struct Idle;
pub struct Enabled;

impl<I: I2c<SevenBitAddress>, D: DelayUs> Lcd<Idle, I, D> {
    pub fn new(i2c: I, delay: D) -> Self {
        Self {
            addr: 0x27,
            delay,
            i2c,
            rows: 1,
            cell_size: 0,
            backlight: 0,
            display_control: 0,
            entry_mode: 0,
            _marker: PhantomData,
        }
    }

    pub fn with_addr(mut self, addr: SevenBitAddress) -> Self {
        self.addr = addr;
        self
    }

    pub fn with_1row(mut self) -> Self {
        self.rows = 1;
        self
    }

    pub fn with_2rows(mut self) -> Self {
        self.rows = 2;
        self
    }

    pub fn with_3rows(mut self) -> Self {
        self.rows = 3;
        self
    }

    pub fn with_4rows(mut self) -> Self {
        self.rows = 4;
        self
    }

    pub fn with_5x8dots(mut self) -> Self {
        self.cell_size = CFG_5X8DOTS;
        self
    }

    pub fn with_5x10dots(mut self) -> Self {
        self.cell_size = CFG_5X10DOTS;
        self
    }

    pub async fn enable(mut self) -> Result<Lcd<Enabled, I, D>, I::Error> {
        // SEE PAGE 45/46 FOR INITIALIZATION SPECIFICATION!
        // according to datasheet, we need at least 40ms after power rises above 2.7V
        // before sending commands. Arduino can turn on way befer 4.5V so we'll wait 50
        self.delay.delay_ms(50).await;

        for _ in 0..3 {
            self.write4bits(0x03 << 4).await?;
        }
        self.write4bits(0x02 << 4).await?;

        let lines = if self.rows > 1 { CFG_2LINE } else { CFG_1LINE };
        self.cmd(CMD_FUNCTION_SET | self.cell_size | lines).await?;

        let mut enabled = Lcd {
            i2c: self.i2c,
            delay: self.delay,
            addr: self.addr,
            rows: self.rows,
            cell_size: self.cell_size,
            backlight: self.backlight,
            display_control: self.display_control,
            entry_mode: self.entry_mode,
            _marker: PhantomData,
        };
        enabled.left_to_right().await?;
        enabled.wakeup().await?;
        enabled.scroll_reset().await?;
        enabled.clear().await?;
        // Somehow return home is not enough to make everything work smoothly.
        // Without this cursor is moving always to the right and goes beyond the
        // screen space.
        // But we do it inside .clear() function, that's why we can omit it here.
        // enabled.move_cursor_to(0, 0).await?;

        Ok(enabled)
    }
}

impl<I: I2c<SevenBitAddress>, D: DelayUs> Lcd<Enabled, I, D> {
    pub async fn write_str(&mut self, s: &str) -> Result<(), I::Error> {
        for byte in s.as_bytes() {
            self.write_u8(*byte).await?;
        }
        Ok(())
    }

    pub async fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), I::Error> {
        for byte in bytes {
            self.write_u8(*byte).await?
        }
        Ok(())
    }

    pub async fn write_u8(&mut self, byte: u8) -> Result<(), I::Error> {
        self.send(byte, MODE_DATA).await
    }

    pub async fn register_custom_char(&mut self, slot: u8, data: &[u8]) -> Result<u8, I::Error> {
        // TODO: handle 5x10 having only 4 chars
        let slot = slot.max(7);

        self.cmd(CMD_SET_CGRAM_ADDR | (slot << 3)).await?;
        for byte in data {
            self.write_u8(*byte).await?;
        }

        Ok(slot)
    }

    pub async fn sleep(&mut self) -> Result<(), I::Error> {
        self.backlight_off().await?;
        self.rendering_off().await
    }

    pub async fn wakeup(&mut self) -> Result<(), I::Error> {
        self.backlight_on().await?;
        self.rendering_on().await
    }

    pub async fn backlight_on(&mut self) -> Result<(), I::Error> {
        self.backlight = BACKLIGHT_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn backlight_off(&mut self) -> Result<(), I::Error> {
        self.backlight = 0x00;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn rendering_on(&mut self) -> Result<(), I::Error> {
        self.display_control |= DISPLAY_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn rendering_off(&mut self) -> Result<(), I::Error> {
        self.display_control &= !DISPLAY_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn cursor_on(&mut self) -> Result<(), I::Error> {
        self.display_control |= DISPLAY_CURSOR_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn cursor_off(&mut self) -> Result<(), I::Error> {
        self.display_control &= !DISPLAY_CURSOR_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn cursor_blink_on(&mut self) -> Result<(), I::Error> {
        self.display_control |= DISPLAY_CURSOR_BLINK_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn cursor_blink_off(&mut self) -> Result<(), I::Error> {
        self.display_control &= !DISPLAY_CURSOR_BLINK_ON;
        self.cmd(CMD_DISPLAY_CONTROL | self.display_control).await
    }

    pub async fn left_to_right(&mut self) -> Result<(), I::Error> {
        self.entry_mode |= ENTRY_LEFT;
        self.cmd(CMD_ENTRY_MODE_SET | self.entry_mode).await
    }

    pub async fn right_to_left(&mut self) -> Result<(), I::Error> {
        self.entry_mode &= !ENTRY_LEFT;
        self.cmd(CMD_ENTRY_MODE_SET | self.entry_mode).await
    }

    pub async fn scroll_left(&mut self) -> Result<(), I::Error> {
        self.cmd(CMD_SHIFT_CURSOR | MOVE_DISPLAY).await
    }

    pub async fn scroll_right(&mut self) -> Result<(), I::Error> {
        self.cmd(CMD_SHIFT_CURSOR | MOVE_DISPLAY | MOVE_RIGHT).await
    }

    pub async fn scroll_reset(&mut self) -> Result<(), I::Error> {
        self.cmd(CMD_RETURN_HOME).await
    }

    pub async fn clear(&mut self) -> Result<(), I::Error> {
        self.cmd(CMD_CLEAR_DISPLAY).await?;
        self.move_cursor_to(0, 0).await
    }

    pub async fn move_cursor_to(&mut self, row: u8, col: u8) -> Result<(), I::Error> {
        let offsets = [0x00, 0x40, 0x14, 0x54];
        let row = if row > self.rows - 1 {
            row.max(self.rows - 1)
        } else {
            row
        };
        let offset_bits: u8 = offsets[usize::from(row)] + col;
        self.cmd(CMD_SET_DDRAM_ADDR | offset_bits).await
    }
}

impl<State, I: I2c<SevenBitAddress>, D: DelayUs> Lcd<State, I, D> {
    async fn cmd(&mut self, data: u8) -> Result<(), I::Error> {
        self.send(data, MODE_CMD).await
    }

    async fn send(&mut self, data: u8, mode: u8) -> Result<(), I::Error> {
        let high_bits: u8 = data & 0xf0;
        let low_bits: u8 = (data << 4) & 0xf0;
        self.write4bits(high_bits | mode).await?;
        self.write4bits(low_bits | mode).await
    }

    async fn write4bits(&mut self, data: u8) -> Result<(), I::Error> {
        const ENABLE: u8 = 0x04;

        self.i2c
            .write(self.addr, &[data | ENABLE | self.backlight])
            .await?;
        self.delay.delay_ms(1).await;
        self.i2c.write(self.addr, &[self.backlight]).await?;
        self.delay.delay_ms(5).await;

        Ok(())
    }
}
