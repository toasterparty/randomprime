
use std::{
    fmt,
    io,
    iter::{once, FromIterator},
    ops::{Deref, DerefMut},
    slice::Iter as SliceIter,
};

use crate::{
    reader::{Reader, Readable},
    writer::Writable,
    lcow::LCow,
};

pub trait DiffListSourceCursor
{
    type Item;
    type Source;

    /// `true` if the cursor was successfully advanced, `false` if not.
    fn next(&mut self) -> bool;
    fn get(&self) -> Self::Item;
    /// Returns the source of the cursor split in two. The current element goes into the
    /// right return value.
    fn split(self) -> (Option<Self::Source>, Self::Source);
    fn split_around(self) -> (Option<Self::Source>, Self::Item, Option<Self::Source>);
}

pub trait AsDiffListSourceCursor: Sized
{
    type Cursor: DiffListSourceCursor<Source=Self>;
    fn as_cursor(&self) -> Self::Cursor;
    fn len(&self) -> usize;
}


pub struct DiffList<A>
    where A: AsDiffListSourceCursor,
{
    list: Vec<DiffListElem<A>>,
}

impl<A> Clone for DiffList<A>
    where A: AsDiffListSourceCursor + Clone,
          <A::Cursor as DiffListSourceCursor>::Item: Clone,
{
    fn clone(&self) -> Self
    {
        DiffList {
            list: self.list.clone(),
        }
    }
}

impl<A> fmt::Debug for DiffList<A>
    where A: AsDiffListSourceCursor + fmt::Debug,
          <A::Cursor as DiffListSourceCursor>::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "DiffList {{ list: {:?} }}", self.list)
    }
}

pub enum DiffListElem<A>
    where A: AsDiffListSourceCursor,
{
    Array(A),
    Inst(<A::Cursor as DiffListSourceCursor>::Item),
}

impl<A> Clone for DiffListElem<A>
    where A: AsDiffListSourceCursor + Clone,
          <A::Cursor as DiffListSourceCursor>::Item: Clone,
{
    fn clone(&self) -> Self
    {
        match *self {
            DiffListElem::Array(ref a) => DiffListElem::Array(a.clone()),
            DiffListElem::Inst(ref i) => DiffListElem::Inst(i.clone()),
        }
    }
}


impl<A> fmt::Debug for DiffListElem<A>
    where A: AsDiffListSourceCursor + fmt::Debug,
          <A::Cursor as DiffListSourceCursor>::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match *self {
            DiffListElem::Array(ref a) => write!(f, "DiffListElem::Array({:?})", *a),
            DiffListElem::Inst(ref i) => write!(f, "DiffListElem::Inst({:?})", i),
        }
    }
}

impl<A> DiffList<A>
    where A: AsDiffListSourceCursor,
{
    pub fn cursor<'s>(&'s mut self) -> DiffListCursor<'s, A>
    {
        let inner_cursor = match self.list.get(0) {
                Some(&DiffListElem::Array(ref a)) => Some(a.as_cursor()),
                _ => None,
            };
        DiffListCursor {
            vec: &mut self.list,
            idx: 0,
            inner_cursor,
        }
    }

    pub fn iter<'s>(&'s self) -> DiffListIter<'s, A>
    {
        DiffListIter {
            list_iter: self.list.iter(),
            inner_cursor: None,
        }
    }

    pub fn elems_iter<'s>(&'s self) -> SliceIter<'s, DiffListElem<A>>
    {
        self.list.iter()
    }

    pub fn len(&self) -> usize
    {
        // TODO: It might make sense to cache this...
        self.list.iter().map(|elem| elem.len()).sum()
    }

    pub fn clear(&mut self)
    {
        self.list.clear()
    }
}

impl<A> DiffListElem<A>
    where A: AsDiffListSourceCursor,
{
    fn len(&self) -> usize
    {
        match *self {
            DiffListElem::Array(ref array) => array.len(),
            DiffListElem::Inst(_) => 1,
        }
    }
}


pub struct DiffListCursor<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    vec: &'list mut Vec<DiffListElem<A>>,
    idx: usize,
    inner_cursor: Option<A::Cursor>,
}

impl<'list, A> DiffListCursor<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    // TODO: Return value?
    pub fn next(&mut self)
    {
        let advance_cursor = self.inner_cursor.as_mut().map(|ic| !ic.next()).unwrap_or(true);
        if advance_cursor && !self.vec.get(self.idx).is_none() {
            self.inner_cursor = None;
            self.idx += 1;
            match self.vec.get(self.idx) {
                None => (),
                Some(&DiffListElem::Inst(_)) => (),
                Some(&DiffListElem::Array(ref a)) => {
                    self.inner_cursor = Some(a.as_cursor());
                },
            };
        };
    }

    // TODO: prev?

    /// Inserts the items yielded by `iter` into the list. The cursor will be
    /// positioned at the first inserted item.
    pub fn insert_before<I>(&mut self, iter: I)
        where I: Iterator<Item=<A::Cursor as DiffListSourceCursor>::Item>
    {
        let mut iter = iter.peekable();
        if iter.peek().is_none() {
            return;
        };

        // XXX This could probably be made more efficent by combining the insert with the splice,
        //     but it'd probably be even harder to understand...
        if let Some(ic) = self.inner_cursor.take() {
            let (left, right) = ic.split();
            if let Some(left) = left {
                self.vec.insert(self.idx, DiffListElem::Array(left));
                self.idx += 1
            };
            self.vec[self.idx] = DiffListElem::Array(right);
        };
        self.vec.splice(self.idx..self.idx, iter.map(DiffListElem::Inst));
        if let Some(DiffListElem::Array(a)) = self.vec.get(self.idx) {
            self.inner_cursor = Some(a.as_cursor());
        }
    }

    /// Inserts the items yielded by `iter` into the list. The cursor will be positioned after the
    /// last inserted item (the same item it was originally pointed to).
    pub fn insert_after<I>(&mut self, iter: I)
        where I: Iterator<Item=<A::Cursor as DiffListSourceCursor>::Item>
    {
        let mut iter = iter.peekable();
        if iter.peek().is_none() {
            return;
        };

        // XXX This could probably be made more efficent by combining the insert with the splice,
        //     but it'd probably be even harder to understand...
        let pre_len = self.vec.len();
        if let Some(ic) = self.inner_cursor.take() {
            let (left, right) = ic.split();
            if let Some(left) = left {
                self.vec.insert(self.idx, DiffListElem::Array(left));
                self.idx += 1
            };
            self.vec[self.idx] = DiffListElem::Array(right);
        };
        self.vec.splice(self.idx..self.idx, iter.map(DiffListElem::Inst));
        self.idx += self.vec.len() - pre_len;
        if let Some(DiffListElem::Array(a)) = self.vec.get(self.idx) {
            self.inner_cursor = Some(a.as_cursor());
        }
    }

    pub fn peek(&mut self) -> Option<LCow<<A::Cursor as DiffListSourceCursor>::Item>>
    {
        if let Some(ref ic) = self.inner_cursor {
            Some(LCow::Owned(ic.get()))
        } else {
            match self.vec.get(self.idx) {
                None => None,
                Some(&DiffListElem::Array(_)) => unreachable!(),
                Some(&DiffListElem::Inst(ref res)) => Some(LCow::Borrowed(res)),
            }
        }
    }

    pub fn value(&mut self) -> Option<&mut <A::Cursor as DiffListSourceCursor>::Item>
    {
        if let Some(ic) = self.inner_cursor.take() {
            let (left, elem, right) = ic.split_around();
            if let Some(right) = right {
                // There are elements to the right
                self.vec[self.idx] = DiffListElem::Array(right);
                self.vec.insert(self.idx, DiffListElem::Inst(elem));
            } else {
                // This was the last element.
                self.vec[self.idx] = DiffListElem::Inst(elem);
            };
            // self.cursor now points to the correct Inst
            if let Some(left) = left {
                // There are elements to the left.
                self.vec.insert(self.idx, DiffListElem::Array(left));
                self.idx += 1
            };
        };
        match self.vec.get_mut(self.idx) {
            Some(&mut DiffListElem::Inst(ref mut inst)) => Some(inst),
            Some(&mut DiffListElem::Array(_)) => unreachable!(),
            None => None,
        }
    }

    pub fn into_value(mut self) -> Option<&'list mut <A::Cursor as DiffListSourceCursor>::Item>
    {
        self.value();
        match self.vec.get_mut(self.idx) {
            Some(&mut DiffListElem::Inst(ref mut inst)) => Some(inst),
            Some(&mut DiffListElem::Array(_)) => unreachable!(),
            None => None,
        }
    }

    pub fn cursor_advancer<'a>(&'a mut self) -> DiffListCursorAdvancer<'a, 'list, A>
    {
        DiffListCursorAdvancer { cursor: self }
    }
}

#[derive(Clone)]
pub struct DiffListIter<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    list_iter: SliceIter<'list, DiffListElem<A>>,
    inner_cursor: Option<A::Cursor>,
}

impl<'list, A> Iterator for DiffListIter<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    type Item = LCow<'list, <A::Cursor as DiffListSourceCursor>::Item>;
    fn next(&mut self) -> Option<Self::Item>
    {
        if let Some(ref mut cursor) = self.inner_cursor {
            if cursor.next() {
                return Some(LCow::Owned(cursor.get()))
            }
        }
        match self.list_iter.next() {
            Some(&DiffListElem::Array(ref array)) => {
                let cursor = array.as_cursor();
                let res = cursor.get();
                self.inner_cursor = Some(cursor);
                Some(LCow::Owned(res))
            },
            Some(&DiffListElem::Inst(ref inst)) => Some(LCow::Borrowed(inst)),
            None => None,
        }
    }
}

impl<'r, A> Readable<'r> for DiffList<A>
    where A: AsDiffListSourceCursor,
          <A::Cursor as DiffListSourceCursor>::Item: Readable<'r>,
{
    type Args = A;
    fn read_from(reader: &mut Reader<'r>, args: A) -> Self
    {
        let res = DiffList {
            list: Vec::from_iter(once(DiffListElem::Array(args))),
        };
        reader.advance(res.size());
        res
    }

    fn size(&self) -> usize
    {
        <A::Cursor as DiffListSourceCursor>::Item::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<A> Writable for DiffList<A>
    where A: AsDiffListSourceCursor,
          <A::Cursor as DiffListSourceCursor>::Item: Writable,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut s = 0;
        for i in self.iter() {
            s += i.write_to(writer)?
        }
        Ok(s)
    }
}

impl<A> FromIterator<<A::Cursor as DiffListSourceCursor>::Item> for DiffList<A>
    where A: AsDiffListSourceCursor
{
    fn from_iter<I>(i: I) -> Self
        where I: IntoIterator<Item = <A::Cursor as DiffListSourceCursor>::Item>
    {
        DiffList {
            list: i.into_iter().map(|x| DiffListElem::Inst(x)).collect(),
        }
    }
}


/// Wraps a DiffListCursor and automatically advances it when it is dropped.
pub struct DiffListCursorAdvancer<'cursor, 'list: 'cursor, A>
    where A: AsDiffListSourceCursor + 'list
{
    cursor: &'cursor mut DiffListCursor<'list, A>,
}

impl<'cursor, 'list: 'cursor, A> Drop for DiffListCursorAdvancer<'cursor, 'list, A>
    where A: AsDiffListSourceCursor + 'list
{
    fn drop(&mut self)
    {
        self.cursor.next()
    }
}

impl<'cursor, 'list: 'cursor, A> Deref for DiffListCursorAdvancer<'cursor, 'list, A>
    where A: AsDiffListSourceCursor + 'list
{
    type Target = DiffListCursor<'list, A>;
    fn deref(&self) -> &Self::Target
    {
        &*self.cursor
    }
}

impl<'cursor, 'list: 'cursor, A> DerefMut for DiffListCursorAdvancer<'cursor, 'list, A>
    where A: AsDiffListSourceCursor + 'list
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut *self.cursor
    }
}

#[cfg(test)]
mod test
{
    struct Source<'a>(&'a [u8]);

    impl<'a> super::AsDiffListSourceCursor for Source<'a>
    {
        type Cursor = Cursor<'a>;
        fn as_cursor(&self) -> Self::Cursor
        {
            Cursor {
                array: self.0,
                idx: 0,
            }
        }

        fn len(&self) -> usize
        {
            self.0.len()
        }
    }

    struct Cursor<'a>
    {
        array: &'a [u8],
        idx: usize,
    }

    impl<'a> super::DiffListSourceCursor for Cursor<'a>
    {
        type Item = u8;
        type Source = Source<'a>;

        fn next(&mut self) -> bool
        {
            if self.idx == self.array.len() - 1 {
                false
            } else {
                self.idx += 1;
                true
            }
        }

        fn get(&self) -> Self::Item
        {
            self.array[self.idx]
        }

        fn split(self) -> (Option<Self::Source>, Self::Source)
        {
            if self.idx == 0 {
                (None, Source(self.array))
            } else {
                let (left, right) = self.array.split_at(self.idx);
                (Some(Source(left)), Source(right))
            }
        }

        fn split_around(self) -> (Option<Self::Source>, Self::Item, Option<Self::Source>)
        {
            let item = self.get();
            if self.array.len() == 1 {
                (None, item, None)
            } else if self.idx == 0 {
                (None, item, Some(Source(&self.array[1..])))
            } else if self.idx == self.array.len() - 1 {
                (Some(Source(&self.array[..self.idx])), item, None)
            } else {
                (Some(Source(&self.array[..self.idx])), item, Some(Source(&self.array[self.idx + 1..])))
            }

        }
    }

    #[test]
    fn test_diff_list_unmodified_iter()
    {
        let junk = &[0u8; 1024][..];
        let source = Source(&[1, 2, 3, 4, 5, 6]);
        let diff_list: super::DiffList<Source> = crate::Reader::new(junk).read(source);

        let v = diff_list.iter()
            .map(|i| i.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(v, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_diff_list_insertion_iter()
    {
        let junk = &[0u8; 1024][..];
        let source = Source(&[1, 2, 3, 4, 5, 6]);
        let mut diff_list: super::DiffList<Source> = crate::Reader::new(junk).read(source);

        {
            let mut cursor = diff_list.cursor();
            cursor.insert_before(std::iter::once(0));
            cursor.next();
            cursor.next();
            cursor.insert_after(std::iter::once(7));
        }

        let v = diff_list.iter()
            .map(|i| i.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(v, vec![0, 1, 7, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_diff_list_multiple_values()
    {
        let junk = &[0u8; 1024][..];
        let source = Source(&[1, 2, 3, 4, 5, 6]);
        let mut diff_list: super::DiffList<Source> = crate::Reader::new(junk).read(source);

        {
            let mut cursor = diff_list.cursor();
            while cursor.peek().is_some() {
                let mut cursor = cursor.cursor_advancer();
                cursor.insert_after(vec![9].into_iter());
                assert!(cursor.peek().is_some());
                assert!(cursor.value().is_some());
                assert!(cursor.peek().is_some());
            }
        }

        let v = diff_list.iter()
            .map(|i| i.into_owned())
            .collect::<Vec<_>>();
        assert_eq!(v, vec![9, 1, 9, 2, 9, 3, 9, 4, 9, 5, 9, 6]);
    }
}
