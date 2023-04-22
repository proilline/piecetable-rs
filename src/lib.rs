use std::ops::Range;

#[derive(Debug, Clone, Copy)]
struct Cursor {
    table_idx: usize,
    inner_offset: Utf8Metric,
}

impl Cursor {
    fn new(table_idx: usize, inner_offset: Utf8Metric) -> Cursor {
        Self {
            table_idx,
            inner_offset,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Utf8Metric {
    utf8: usize,
    byte: usize,
}

impl Utf8Metric {
    fn new(byte_size: usize, utf8_size: usize) -> Self {
        Utf8Metric {
            byte: byte_size,
            utf8: utf8_size,
        }
    }

    fn from_str(utf8_str: &str) -> Self {
        let utf8_size = utf8_str.chars().count();
        let byte_size = utf8_str.len();
        Self {
            utf8: utf8_size,
            byte: byte_size,
        }
    }
}

// Returns a offset of nth unicode's byte index
fn to_byte_offset(data: &str, unicode_offset: usize) -> usize {
    data.chars()
        .map(|ch| ch.len_utf8())
        .take(unicode_offset)
        .sum()
}

#[derive(Debug, Clone, Copy)]
enum Location {
    Original,
    Added,
}

#[derive(Debug, Clone)]
struct Piece {
    begin: usize,
    len: Utf8Metric,
    location: Location,
}

#[derive(Debug, Clone)]
pub struct PieceTable {
    src: String,
    added: String,
    table: Vec<Piece>,
}

impl Piece {
    fn new(begin: usize, len: Utf8Metric, location: Location) -> Piece {
        Piece {
            begin,
            len,
            location,
        }
    }

    fn from_str(data: &str, begin: usize) -> Piece {
        let len = Utf8Metric::from_str(data);
        let location = Location::Added;

        Piece {
            begin,
            len,
            location,
        }
    }

    fn range(&self) -> Range<usize> {
        self.begin..(self.begin + self.len.byte)
    }
}

impl PieceTable {
    fn find_offset(&self, mut unicode_offset: usize) -> Cursor {
        for (idx, piece) in self.table.iter().enumerate() {
            if unicode_offset <= piece.len.utf8 {
                let data = self.get_piece_data(piece);
                // get nth unicode character's byte offset
                let byte_offset = to_byte_offset(data, unicode_offset);
                let len = Utf8Metric::new(byte_offset, unicode_offset);
                return Cursor::new(idx, len);
            }
            unicode_offset -= piece.len.utf8;
        }
        panic!("index out of range") // when index out-of-range
    }

    fn get_piece(&self, cur: &Cursor) -> &Piece {
        &self.table[cur.table_idx]
    }

    fn get_piece_data(&self, piece: &Piece) -> &str {
        match piece.location {
            Location::Added => &self.added[piece.range()],
            Location::Original => &self.src[piece.range()],
        }
    }

    pub fn insert_at(&mut self, unicode_offset: usize, data: &str) {
        let new_piece = Piece::from_str(data, self.added.len());

        let cursor = self.find_offset(unicode_offset);
        let insert_offset = self.split_piece(&cursor);

	self. table.insert(insert_offset, new_piece);

        self.added += data;
    }

    pub fn new(src: &str) -> PieceTable {
        let len = Utf8Metric::from_str(src);
        let init = Piece::new(0, len, Location::Original);

        PieceTable {
            src: src.to_owned(),
            added: String::new(),
            table: vec![init],
        }
    }

    pub fn remove_range(&mut self, range: Range<usize>) {
        let Range { start, end } = range;
        let cur_start = self.find_offset(start);
        let cur_end = self.find_offset(end);

        let prev = self.split_piece(&cur_start);
        let next = self.split_piece(&cur_end);

	self.table.drain(prev..next);
    }

    // split the piece and return table's offset
    fn split_piece(&mut self, cur: &Cursor) -> usize {
        let offset = cur.inner_offset;
        let piece = self.get_piece(cur);
        debug_assert!(piece.len.byte >= offset.byte);
        debug_assert!(piece.len.utf8 >= offset.utf8);

        if piece.len.byte == offset.byte {
            return cur.table_idx + 1;
        }

        let prev_piece = Piece::new(piece.begin, offset, piece.location);

        let next_piece = {
            let begin = offset.byte + piece.begin;
            let byte_len = piece.len.byte - offset.byte;
            let utf8_len = piece.len.utf8 - offset.utf8;
            let len = Utf8Metric::new(byte_len, utf8_len);
            Piece::new(begin, len, piece.location)
        };

	self.table.remove(cur.table_idx);
	self.table.insert(cur.table_idx, prev_piece);
	self.table.insert(cur.table_idx + 1, next_piece);
	
	// [piece 1] [piece 2] ... [piece k] ... [piece n]
	//                           |_ cur.table_idx
	// [piece 1] [piece 2] ... [prev_piece] [next_piece] ...
	
        cur.table_idx + 1
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        for piece in self.table.iter() {
            ret += self.get_piece_data(piece);
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byteoffset() {
        let test = "다람쥐 헌 쳇바퀴에 타고파";
        assert_eq!(to_byte_offset(test, 2), "다람".len());
        assert_eq!(to_byte_offset(&test["다람".len()..], 2), "쥐 ".len());
    }

    #[test]
    fn test_get_piecedata() {
        let test = "a̐éö̲\r\nae";
        let table = PieceTable::new(test);
        assert_eq!(table.src, table.get_piece_data(&table.table[0]));
    }

    #[test]
    fn test_findoffset() {
        // TODO : Add test
    }

    #[test]
    fn test_splitpiece() {
        //let table = PieceTable::new("04/25 (화)까지 조교에게 연락하여 면담 일정을 조정해주세요");
	// TODO : add test

    }

    #[test]
    fn test_insert_at() {
        let mut table = PieceTable::new("다람쥐 헌 쳇바퀴에 타고파");
        table.insert_at(0, " test");

        assert_eq!(table.to_string(), " test다람쥐 헌 쳇바퀴에 타고파");
        table.insert_at(3, " test");
        assert_eq!(table.to_string(), " te testst다람쥐 헌 쳇바퀴에 타고파");
        table.insert_at(11, " test");
        assert_eq!(
            table.to_string(),
            " te testst다 test람쥐 헌 쳇바퀴에 타고파"
        );

        let mut table = PieceTable::new("");
        table.insert_at(0, "12345");
        assert_eq!(table.to_string(), "12345");
        table.insert_at(5, "다람쥐");
        println!("{table:#?}");
        assert_eq!(table.to_string(), "12345다람쥐");
    }
    #[test]
    fn test_remove() {
        let mut table = PieceTable::new("");
        table.insert_at(0, "12345");

        assert_eq!(table.to_string(), "12345");
        table.insert_at(5, "다람쥐");

        assert_eq!(table.to_string(), "12345다람쥐");
        table.remove_range(0..4);

        assert_eq!(table.to_string(), "5다람쥐");
        table.remove_range(0..3);

        assert_eq!(table.to_string(), "쥐");
    }
}
