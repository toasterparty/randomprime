use auto_struct_macros::auto_struct;
use reader_writer::FixedArray;
use reader_writer::typenum::U300;

#[derive(Debug, Clone)]
#[auto_struct(Readable, Writable)]
pub struct Dumb<'r> {
    pub data: FixedArray<u32, U300>,
    phantom: core::marker::PhantomData<&'r mut [u8]>
}
