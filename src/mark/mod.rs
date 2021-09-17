
use crate::span_table::SpanTable;

use std::rc::{Rc, Weak};
use std::cell::Cell;

#[derive(Default)]
pub struct EditingBuffer {
    buffer: Vec<u8>,
    span_table: SpanTable,
    // TODO: sort by starting pos? what about same starting pos but different ending pos
    // the buffer needs to hold all marks inside of it so that it can apply offsets
    // TODO: what about overlapping Cursors
    // TODO: probably rework this later
    // https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/src/selection.rs
    cursors: Vec<Weak<Cell<Cursor>>>,
}

// Don't let users outside the crate copy it
#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    // byte positions
    pub start: usize,
    pub end: usize,
    // saved horizontal pos
}

impl EditingBuffer {
    fn new_cursor(&mut self) -> Rc<Cell<Cursor>> {
        let cursor = Cursor {
            start: 0,
            end: 0,
        };

        let cursor = Rc::new(Cell::new(cursor));
        self.cursors.push(Rc::downgrade(&cursor));
        cursor
    }

    // All modifications go through the set operation.
    // Insertion: Cursor with no selection set to "some text"
    // Deletion: Cursor with selection set to ""
    fn set(&mut self, cursor: &Rc<Cell<Cursor>>, content: &[u8]) {
        let cursor_ref = Rc::clone(cursor);
        let mut cursor = (*cursor_ref).get();
        cursor.start = 5;
        (*cursor_ref).set(cursor);
        // we only handle carets right now
        //if (cursor.start != cursor.end) {panic!()}
        //let pos = self.span_table.byte_offset(cursor.start);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set() {
        let mut eb = EditingBuffer::default();
        let c = eb.new_cursor();
        let mut c2 = c.get();
        c2.end = 8;
        c.set(c2);

        let content = "test".as_bytes();
        println!("{:?}", c);
        eb.set(&c, content);
        println!("{:?}", c);
        assert_eq!(c.get().start, 5);
        

        assert_eq!(c.get().end, 8);
    }
}