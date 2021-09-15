

/*
TODO: make resistant from false bounds from plugins
*/

#[derive(Copy, Clone)]
pub struct Span {
    start: usize,
    end: usize
    // Potentially cache the number of lines present
}

pub enum Operation {
    InsertSpan {span: Span, index: usize},
    RemoveSpan {span: Span, index: usize},
    // Split span at byte_offset into the span
    SplitSpan {span: Span, index: usize, byte_offset: usize}
    //NewString: add to the append only buffer, this should probably be a command to the server instead of the piece chain
}
/*
pub struct EditBuffer {
    Buffer
    SpanTable
    Generation
}
*/

// how do i mutate the buffer in an append only way

// Piece Table
pub struct SpanTable<'a> {
    buffer: &'a Vec<u8>,
    spans: Vec<Span>,
    // TODO: rename to operations
    commands: Vec<Operation>,
    // TODO: last edited span for contigous edits
}

impl<'a> SpanTable<'a> {
    pub fn new(buffer: &'a Vec<u8>) -> Self {
        SpanTable {
            buffer,
            spans: Vec::new(),
            commands: Vec::new(),
        }
    }

    pub fn command_idx(&self) -> usize {
        self.commands.len()
    }

    pub fn insert_span(&mut self, span: Span, index: usize) {
        // TODO: merge if the previous command was insertspan
        self.spans.insert(index, span);
        self.commands.push(Operation::InsertSpan {span, index});
    }

    pub fn remove_span(&mut self, index: usize) {
        let span = self.spans.remove(index);
        self.commands.push(Operation::RemoveSpan {span, index});
    }

    // split at byte offset into the span
    pub fn split_span(&mut self, index: usize, byte_offset: usize) {
        let original_span = self.spans[index];

        let left_span = Span {
            start: original_span.start,
            end: original_span.start + byte_offset
        };
        let right_span = Span {
            start: original_span.start + byte_offset,
            end: original_span.end
        };

        self.spans[index] = left_span;
        self.spans.insert(index + 1, right_span);

        self.commands.push(Operation::SplitSpan {span: original_span, index, byte_offset});
    }

    pub fn contents(&self) -> Vec<u8> {
        let mut contents: Vec<u8> = Vec::new();
        for span in &self.spans {
            contents.extend(&self.buffer[span.start .. span.end]);
        }
        contents
    }
    
    pub fn spans(&self) -> Vec<&[u8]> {
        let mut spans: Vec<&[u8]> = Vec::new();
        for span in &self.spans {
            spans.push(&self.buffer[span.start .. span.end]);
        }
        spans
    }
}

/* merge_span 
    combine two spans into one span object
    if the two spans are adjacent in the buffer, this just returns a new span
    otherwise it copies the two together and adds it to the end of the buffer
    - maybe consider not supporting adding to the end of the buffer

    removespan twice, then insertspan

    split_span
        removespan once, then insertspan twice
*/

mod test {
    use super::*;

    fn new_str(vec: &mut Vec<u8>, add: &str) -> Span {
        let span = Span {
            start: vec.len(),
            end: vec.len() + add.len()
        };
        vec.extend(add.as_bytes());
        span
    }

    fn assert_span_table_equals(span_table: &SpanTable<'_>, expected: &str) {
        assert_eq!(expected, std::str::from_utf8(&span_table.contents()).unwrap());
    }

    fn assert_spans_equal(span_table: &SpanTable<'_>, expected: &[&str]) {
        let spans = span_table.spans();
        let spans: Vec<&str> = spans.into_iter().map(|x| std::str::from_utf8(x).unwrap()).collect();
        assert_eq!(spans, expected);
    }

    #[test]
    fn test_insert() {
        let mut v: Vec<u8> = Vec::new();
        let hello = new_str(&mut v, "hello");
        let world = new_str(&mut v, "world");
        let onetwothree = new_str(&mut v, "123");
        let abc = new_str(&mut v, "abc");
        

        let mut st = SpanTable::new(&v);

        st.insert_span(hello, 0);
        assert_span_table_equals(&st, "hello");

        st.insert_span(world, 1);
        assert_span_table_equals(&st, "helloworld");

        st.insert_span(abc, 1);
        assert_span_table_equals(&st, "helloabcworld");

        st.insert_span(onetwothree, 0);
        assert_span_table_equals(&st, "123helloabcworld");
    }

    #[test]
    fn test_remove() {
        let mut v: Vec<u8> = Vec::new();
        let hello = new_str(&mut v, "hello");
        let world = new_str(&mut v, "world");
        let onetwothree = new_str(&mut v, "123");
        let abc = new_str(&mut v, "abc");
        

        let mut st = SpanTable::new(&v);

        st.insert_span(hello, 0);
        st.insert_span(world, 1);
        st.insert_span(abc, 1);
        st.insert_span(onetwothree, 0);
        assert_span_table_equals(&st, "123helloabcworld");

        st.remove_span(1);
        assert_span_table_equals(&st, "123abcworld");

        st.remove_span(0);
        assert_span_table_equals(&st, "abcworld");

        st.remove_span(1);
        assert_span_table_equals(&st, "abc");

        st.remove_span(0);
        assert_span_table_equals(&st, "");
    }

    #[test]
    fn test_split() {
        let mut v: Vec<u8> = Vec::new();
        let contents = new_str(&mut v, "123helloabcworld");
        let mut st = SpanTable::new(&v);

        st.insert_span(contents, 0);
        assert_span_table_equals(&st, "123helloabcworld");

        st.split_span(0, 3);
        assert_spans_equal(&st, &["123", "helloabcworld"]);

        st.split_span(1, 5);
        assert_spans_equal(&st, &["123", "hello", "abcworld"]);

        st.split_span(2, 3);
        assert_spans_equal(&st, &["123", "hello", "abc", "world"]);
    }

}