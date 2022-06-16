const MAGIC: u32 = 0x71 << 24 | 0x6f << 16 | 0x69 << 8 | 0x72;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Operation {
    NONE,
    RGB(u32),
    RGBA(u32),
    INDEX(u8),
    DIFF(u8),
    LUMA(u16),
    RUN(u8),
}

impl Operation {
    fn encode(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();

        match *self {
            Operation::RGB(v) => {
                bytes.push(0xfe);
                bytes.push((v >> 24) as u8);
                bytes.push((v >> 16) as u8);
                bytes.push((v >>  8) as u8);
            },
            Operation::RGBA(v) => {
                bytes.push(0xff);
                write_32(&mut bytes, v);
            },
            Operation::INDEX(v) => bytes.push(0x00 | v),
            Operation::DIFF(v)  => bytes.push(0x40 | v),
            Operation::LUMA(v)  => {
                bytes.push(0x80 | (v >> 8 & 0xbf) as u8);
                bytes.push(v as u8);
            }
            Operation::RUN(v)   => bytes.push(0xc0 | v),
            _ => {},
        }

        return bytes;
    }

    fn decode(data: &[u8], i: usize) -> (Self, usize) {
        let byte = data[i];

        if byte == 0xfe {
            return (Operation::RGB(
                (data[i + 1] as u32) << 24 |
                (data[i + 2] as u32) << 16 |
                (data[i + 3] as u32) <<  8 |
                0xff
            ), 4);
        } else if byte == 0xff {
            return (Operation::RGBA(
                (data[i + 1] as u32) << 24 |
                (data[i + 2] as u32) << 16 |
                (data[i + 3] as u32) <<  8 |
                (data[i + 4] as u32)
            ), 5)
        }
        return match byte & 0xc0 {
            0x00 => (Operation::INDEX(byte), 1),
            0x40 => (Operation::DIFF(byte), 1),
            0x80 => (Operation::LUMA((byte as u16) << 8 | data[i + 1] as u16), 2),
            0xc0 => (Operation::RUN(byte), 1),
            _    => (Operation::NONE, 1),
        }
    }
}

#[derive(Debug)]
struct Header {
    magic: u32,
    width: u32,
    height: u32,
    channels: u8,
    colorspace: u8,
}

#[derive(Clone, Copy, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl std::fmt::Display for Pixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a);
    }
}

impl Pixel {
    fn new() -> Self {
        return Pixel{ r: 0, g: 0, b: 0, a: 255 };
    }

    fn rgba(&self) -> u32 {
        return (self.r as u32) << 24 |
               (self.g as u32) << 16 |
               (self.b as u32) << 8  |
               (self.a as u32);
    }

    fn from(content: u32) -> Self {
        return Pixel {
            r: ((content >> 24) & 0xff) as u8,
            g: ((content >> 16) & 0xff) as u8,
            b: ((content >>  8) & 0xff) as u8,
            a: ((content)       & 0xff) as u8,
        }
    }
}

fn write_32(bytes: &mut Vec<u8>, content: u32) {
    bytes.push(((content >> 24) & 0xff) as u8);
    bytes.push(((content >> 16) & 0xff) as u8);
    bytes.push(((content >>  8) & 0xff) as u8);
    bytes.push(((content)       & 0xff) as u8);
}

fn read_32(bytes: &[u8], index: usize) -> u32 {
    return (bytes[index    ] as u32) << 24 |
           (bytes[index + 1] as u32) << 16 |
           (bytes[index + 2] as u32) <<  8 |
           (bytes[index + 3] as u32);
}

fn pixel_index(p: Pixel) -> usize {
    return ((
        p.r.wrapping_mul(3)
           .wrapping_add(p.g.wrapping_mul(5))
           .wrapping_add(p.b.wrapping_mul(7))
           .wrapping_add(p.a.wrapping_mul(11))
    ) % 64) as usize;
}

fn encode(data: &[u8], header: Header) -> Result<Vec<u8>, String> {
    if
        data.len() == 0 ||
        header.width == 0 || header.height == 0 ||
        header.channels < 3 || header.channels > 4 ||
        header.colorspace > 1
    {
        return Err("wrong header".to_string());
    }

    let mut bytes: Vec<u8> = Vec::new();
    write_32(&mut bytes, header.magic);
    write_32(&mut bytes, header.width);
    write_32(&mut bytes, header.height);
    bytes.push(header.channels);
    bytes.push(header.colorspace);

    let mut operations: Vec<Operation> = Vec::new();
    let mut index = [Pixel::new(); 64];

    let channels: u32 = header.channels.into();
    let length: usize = (header.width * header.height * channels) as usize;
    let px_end = length - (header.channels as usize);

    let mut i: usize = 0;
    let mut prev_op = Operation::NONE;
    let mut prev_px = Pixel::new();
    let mut run = 0;

    while i < length {
        let px = Pixel {
            r: data[i],
            g: data[i + 1],
            b: data[i + 2],
            a: if channels == 4 { data[i + 3] } else { 0xff },
        };

        let op: Operation;

        let px_index = pixel_index(px);
        if index[px_index] == px {
            op = Operation::INDEX(px_index as u8);
        } else {
            index[px_index] = px;

            if px.a != prev_px.a {
                op = Operation::RGBA(px.rgba());
            } else {

                let vr = (px.r as i8).wrapping_sub(prev_px.r as i8);
                let vg = (px.g as i8).wrapping_sub(prev_px.g as i8);
                let vb = (px.b as i8).wrapping_sub(prev_px.b as i8);

                let vrg = vr.wrapping_sub(vg);
                let vbg = vb.wrapping_sub(vg);

                if vr > -3 && vr < 2 &&
                   vg > -3 && vg < 2 &&
                   vb > -2 && vb < 2 {
                    op = Operation::DIFF(0x40 | ((vr + 2) << 4 | (vg + 2) << 2 | (vb + 2)) as u8);
                } else if vrg >  -9 && vrg <  8 &&
                          vg  > -33 && vg  < 32 &&
                          vbg >  -9 && vbg <  8 {
                    op = Operation::LUMA(
                        0x8000 |
                        (vg.wrapping_add(32) as u16) << 8 |
                        (vrg.wrapping_add(8) as u16) << 4 |
                        (vbg.wrapping_add(8) as u16)
                    );
                } else {
                    op = Operation::RGB(px.rgba());
                }
            }
        }

        if op == prev_op {
            run += 1;
            if run == 62 || i == px_end {
                operations.push(Operation::RUN(0xc0 | (run - 1)));
                run = 0;
            }
        } else if run > 0 {
            operations.push(Operation::RUN(0xc0 | (run - 1)));
            operations.push(op);
            run = 0;
        } else {
            operations.push(op);
            run = 0;
        }

        prev_px = px;
        prev_op = op;

        i += channels as usize;
    }

    for op in operations {
        bytes.extend(op.encode());
    }
    bytes.extend(vec![0, 0, 0, 0, 0, 0, 0, 0, 1]);

    return Ok(bytes);
}

fn decode(data: &[u8]) -> Result<(Header, Vec<u8>), String> {
    if data.len() < 14 + 8 {
        return Err("length is 0".to_string());
    }

    let header = Header {
        magic:      read_32(data, 0),
        width:      read_32(data, 4),
        height:     read_32(data, 8),
        channels:   data[12],
        colorspace: data[13],
    };

    if header.magic != MAGIC {
        return Err(format!("header magic expected {}, found {}", MAGIC, header.magic))
    }

    if header.width == 0 || header.height == 0 ||
       header.channels < 3 || header.channels > 4 ||
       header.colorspace > 1 {
        return Err("wrong header".to_string());
    }

    let channels: u32 = header.channels.into();
    let px_len = (header.width * header.height * channels) as usize;
    let mut bytes: Vec<u8> = vec![0; px_len];
    let mut px_pos: usize = 0;
    let mut i: usize = 14;

    let mut index = [Pixel::new(); 64];
    let mut prev_op = Operation::NONE;
    let mut px = Pixel::new();
    let mut run = 0;

    while px_pos < px_len {
        let op: Operation;

        if run > 0 {
            run -= 1;
            op = prev_op;
        } else {
            let offset: usize;
            (op, offset) = Operation::decode(&data, i);
            if op == Operation::NONE {
                return Err(format!("cannot parse operation {}", bytes[i]));
            }
            i += offset;
        }

        match op {
            Operation::RGB(v) => px = Pixel::from(v),
            Operation::RGBA(v) => px = Pixel::from(v),
            Operation::DIFF(v) => {
                px.r = px.r.wrapping_add((v >> 4) & 0x03).wrapping_sub(2);
                px.g = px.g.wrapping_add((v >> 2) & 0x03).wrapping_sub(2);
                px.b = px.b.wrapping_add((v)      & 0x03).wrapping_sub(2);
            },
            Operation::INDEX(v) => px = index[v as usize],
            Operation::LUMA(v) => {
                let b1: u8 = (v >> 8) as u8;
                let b2: u8 = v as u8;
                let vg = (b1 & 0x3f).wrapping_sub(32);

                px.r = px.r.wrapping_add(vg).wrapping_sub(8).wrapping_add((b2 >> 4) & 0x0f);
                px.g = px.g.wrapping_add(vg);
                px.b = px.b.wrapping_add(vg).wrapping_sub(8).wrapping_add(b2 & 0x0f);
            },
            Operation::RUN(v) => run = (v & 0x3f) + 1,
            _ => {},
        }

        if !matches!(op, Operation::RUN(_)) {
            index[pixel_index(px) as usize] = px;
            bytes[px_pos    ] = px.r;
            bytes[px_pos + 1] = px.g;
            bytes[px_pos + 2] = px.b;
            if channels == 4 {
                bytes[px_pos + 3] = px.a;
            }

            px_pos += channels as usize;
            prev_op = op;
        }
    }

    return Ok((header, bytes));
}

fn write(filename: &str, data: &Vec<u8>, header: Header) -> Result<(), Box<dyn std::error::Error>> {
    let encoded = encode(&data, header)?;

    std::fs::write(filename, encoded)?;

    return Ok(());
}

fn read(filename: &str) -> Result<(Header, Vec<u8>), Box<dyn std::error::Error>> {
    let data = std::fs::read(filename)?;

    let decoded = decode(&data)?;

    return Ok(decoded);
}

fn main() {
    let header =  Header {
        magic: MAGIC,
        width: 400,
        height: 400,
        channels: 4,
        colorspace: 0,
    };

    let filename = "a.raw";
    let raw_a = std::fs::read(filename).unwrap();

    if let Err(e) = write("a.qoi", &raw_a, header) {
        panic!("write error: {}", e);
    }

    match read("a.qoi") {
        Ok((_header, data)) => {
            let mut v: Vec<u8> = Vec::new();
            v.extend(data);

            match std::fs::write("b.raw", v) {
                Err(e) => panic!("{}", e),
                _ => {},
            }
        },
        Err(e) => panic!("read error: {}", e),
    };
}
