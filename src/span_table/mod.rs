

/*
TODO: make resistant from false bounds from plugins
TODO: make iterator
*/

// TODO: move into SpanTable (nested types?)

#[derive(Copy, Clone)]
pub struct Span {
    // TODO: phantomdata?
    pub start: usize,
    pub end: usize
    // Potentially cache the number of lines present
}

impl Span {
    pub fn len(&self) -> usize {self.end - self.start}
}

pub enum Operation {
    InsertSpan {span: Span, index: usize},
    RemoveSpan {span: Span, index: usize},
    // Split span at byte_offset into the span
    SplitSpan {span: Span, index: usize, byte_offset: usize}
    //NewString: add to the append only buffer, this should probably be a command to the server instead of the piece chain
}
/*
TODO: make this memory mapped and shared buffer between processes, maybe have a (start, end, memory_segment) span to handle resizing in a less dumb way
would still have good locality because we would map enough space for the file + a generous buffer, so most things would stay on the same allocation, or at worst two allocations

pub struct EditBuffer {
    Buffer
    SpanTable
    Generation
    Vec<Mark>
}
*/

// Piece Table
// owns no data, only manages ranges
#[derive(Default)]
pub struct SpanTable {
    // TODO: handle zero length spans?
    spans: Vec<Span>,
    // TODO: rename to operations
    commands: Vec<Operation>,
    // TODO: last edited span for contigous edits
}

// TODO: store generation in debug mode so that is is only valid for one generation
pub struct SpanPos {
    pub span_index: usize,
    pub byte_offset: usize
}

impl SpanTable {
    pub fn command_idx(&self) -> usize {
        self.commands.len()
    }

    pub fn span_len(&self) -> usize {
        self.spans.len()
    }

    // TODO: write tests
    pub fn byte_offset(&self, offset: usize) -> SpanPos {
        if offset == 0 {
            return SpanPos {span_index: 0, byte_offset: 0}
        }

        let mut travelled = 0;
        for (span_index, span) in self.spans.iter().enumerate() {
            if travelled + span.len() >= offset {
                return SpanPos {span_index, byte_offset: offset - travelled}
            }
            travelled = travelled + span.len()
        }
        // TODO: handle offset outside of span_table
        panic!();
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

    pub fn contents(&self, buffer: &Vec<u8>) -> Vec<u8> {
        let mut contents: Vec<u8> = Vec::new();
        for span in &self.spans {
            contents.extend(&buffer[span.start .. span.end]);
        }
        contents
    }

    pub fn spans<'a>(&self, buffer: &'a Vec<u8>) -> Vec<&'a [u8]> {
        let mut spans: Vec<&[u8]> = Vec::new();
        for span in &self.spans {
            spans.push(&buffer[span.start .. span.end]);
        }
        spans
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Default)]
    struct SpanTableBuffer {
        buffer: Vec<u8>,
        pub st: SpanTable
    }

    impl SpanTableBuffer {
        fn span(&mut self, add: &str) -> Span {
            let span = Span {
                start: self.buffer.len(),
                end: self.buffer.len() + add.len()
            };
            self.buffer.extend(add.as_bytes());
            span
        }
    
        fn assert_span_table_equals(&self, expected: &str) {
            assert_eq!(expected, std::str::from_utf8(&self.st.contents(&self.buffer)).unwrap());
        }
    
        fn assert_spans_equal(&self, expected: &[&str]) {
            let spans = self.st.spans(&self.buffer);
            let spans: Vec<&str> = spans.into_iter().map(|x| std::str::from_utf8(x).unwrap()).collect();
            assert_eq!(spans, expected);
        }
    }
    
    #[test]
    fn test_insert() {
        let mut stb = SpanTableBuffer::default();

        let span = stb.span("hello");
        stb.st.insert_span(span, 0);
        stb.assert_span_table_equals("hello");

        let span = stb.span("world");
        stb.st.insert_span(span, 1);
        stb.assert_span_table_equals("helloworld");

        let span = stb.span("abc");
        stb.st.insert_span(span, 1);
        stb.assert_span_table_equals("helloabcworld");

        let span = stb.span("123");
        stb.st.insert_span(span, 0);
        stb.assert_span_table_equals("123helloabcworld");
    }

    
    #[test]
    fn test_remove() {
        let mut stb = SpanTableBuffer::default();

        let span = stb.span("hello");
        stb.st.insert_span(span, 0);
        let span = stb.span("world");
        stb.st.insert_span(span, 1);
        let span = stb.span("abc");
        stb.st.insert_span(span, 1);
        let span = stb.span("123");
        stb.st.insert_span(span, 0);

        stb.assert_span_table_equals("123helloabcworld");

        stb.st.remove_span(1);
        stb.assert_span_table_equals("123abcworld");

        stb.st.remove_span(0);
        stb.assert_span_table_equals("abcworld");

        stb.st.remove_span(1);
        stb.assert_span_table_equals("abc");

        stb.st.remove_span(0);
        stb.assert_span_table_equals("");
    }

    
    #[test]
    fn test_split() {
        let mut stb = SpanTableBuffer::default();

        let span = stb.span("123helloabcworld");
        stb.st.insert_span(span, 0);      

        stb.st.split_span(0, 3);
        stb.assert_spans_equal(&["123", "helloabcworld"]);

        stb.st.split_span(1, 5);
        stb.assert_spans_equal(&["123", "hello", "abcworld"]);

        stb.st.split_span(2, 3);
        stb.assert_spans_equal(&["123", "hello", "abc", "world"]);
    }

}