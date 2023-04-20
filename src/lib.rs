use std::ops::Range;
// piece merging?
// vector vs tree vs dequeue vs linked list vs chunked array vs ... etc
// no need for random access. fast linear access
// table << multi-threading?

// using internally

#[derive(Debug, Clone, Copy)]
struct Utf8Offset {
    utf8: usize,
    byte: usize,
}

impl Utf8Offset {
    fn new(byte_size: usize, utf8_size: usize) -> Self {
        Utf8Offset {
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
    // convert as chars
    data.chars()
        .map(|ch| ch.len_utf8())
        .take(unicode_offset)
        .sum()
}

fn split_piece(piece: &Piece, offset: Utf8Offset) -> (Piece, Piece) {
    debug_assert!(piece.len.byte >= offset.byte);
    debug_assert!(piece.len.utf8 >= offset.utf8);

    let prev_piece = { Piece::new(piece.begin, offset, piece.location) };

    let next_piece = {
        let begin = offset.byte + piece.begin;
        let byte_len = piece.len.byte - offset.byte;
        let utf8_len = piece.len.utf8 - offset.utf8;
        let len = Utf8Offset::new(byte_len, utf8_len);

        Piece::new(begin, len, piece.location)
    };

    (prev_piece, next_piece)
}

#[derive(Debug, Clone, Copy)]
enum Location {
    Original,
    Added,
}

#[derive(Debug, Clone)]
struct Piece {
    begin: usize,
    len: Utf8Offset,
    location: Location,
}

#[derive(Debug, Clone)]
pub struct PieceTable {
    src: String,
    added: String,
    table: Vec<Piece>,
}

impl Piece {
    fn new(begin: usize, len: Utf8Offset, location: Location) -> Piece {
        Piece {
            begin,
            len,
            location,
        }
    }

    fn from_str(data: &str, begin: usize) -> Piece {
        let len = Utf8Offset::from_str(data);
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
    fn find_offset(&self, mut unicode_offset: usize) -> (usize, Utf8Offset) {
        for (idx, piece) in self.table.iter().enumerate() {
            if unicode_offset <= piece.len.utf8 {
                let data = self.get_piece_data(piece);
                // get nth unicode character's byte offset
                let byte_offset = to_byte_offset(data, unicode_offset);
                let len = Utf8Offset::new(byte_offset, unicode_offset);
                return (idx, len);
            }
            unicode_offset -= piece.len.utf8;
        }
        panic!("index out of range") // when index out-of-range
    }

    fn get_piece_data(&self, piece: &Piece) -> &str {
        match piece.location {
            Location::Added => &self.added[piece.range()],
            Location::Original => &self.src[piece.range()],
        }
    }

    pub fn insert_at(&mut self, unicode_offset: usize, data: &str) {
        let new_piece = Piece::from_str(data, self.added.len());

        let (piece_idx, offset) = self.find_offset(unicode_offset);
        let piece = &self.table[piece_idx];

        // no need to split piece
        if offset.byte == piece.len.byte {
            self.table.insert(piece_idx + 1, new_piece);
            self.added += data;
            return;
        }

        let (prev, next) = split_piece(piece, offset);

        //split piece
        self.table
            .splice(piece_idx..piece_idx + 1, [prev, new_piece, next]);

        self.added += data;
    }

    pub fn new(src: &str) -> PieceTable {
        let len = Utf8Offset::from_str(src);
        let init = Piece {
            begin: 0,
            len,
            location: Location::Original,
        };
        PieceTable {
            src: src.to_owned(),
            added: String::new(),
            table: vec![init],
        }
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        for piece in self.table.iter() {
            ret += self.get_piece_data(piece);
        }
        ret
    }

    pub fn remove_range(&mut self, range: Range<usize>) {
        let Range { start, end } = range;
        let (table_start, start) = self.find_offset(start);
        let (table_end, end) = self.find_offset(end);

        // no need to split start
        if self.table[table_start].len.byte == start.byte {
            let (_, next) = split_piece(&self.table[table_end], end);
            self.table.splice(table_start..=table_end, [next]);
            return;
        }

        // no need to split end
        if self.table[table_end].len.byte == start.byte {
            let (prev, _) = split_piece(&self.table[table_start], start);
            self.table.splice(table_start..=table_end, [prev]);
            return;
        }

        let (prev, _) = split_piece(&self.table[table_start], start);
        let (_, next) = split_piece(&self.table[table_end], end);

        self.table.splice(table_start..=table_end, [prev, next]);
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
        let table = PieceTable::new("Testing...a̐éö̲\r\nae");

        let (prev, next) = split_piece(&table.table[0], Utf8Offset::new(8, 8));

        assert_eq!(table.get_piece_data(&prev), "Testing.");
        assert_eq!(table.get_piece_data(&next), "..a̐éö̲\r\nae");
        let (prev, next) = split_piece(&prev, Utf8Offset::new(0, 0));

        assert_eq!(table.get_piece_data(&prev), "");
        assert_eq!(table.get_piece_data(&next), "Testing.");

        let (prev, next) = split_piece(&next, Utf8Offset::new(8, 8));

        assert_eq!(table.get_piece_data(&prev), "Testing.");
        assert_eq!(table.get_piece_data(&next), "");

        let table = PieceTable::new("a̐éö̲");
        let (prev, _) = split_piece(&table.table[0], Utf8Offset::new(3, 1));
        assert_eq!(table.get_piece_data(&prev), "a̐");
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
