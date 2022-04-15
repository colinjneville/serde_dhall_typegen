#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementQuantity {
    Zero,
    Multiple,
}

pub trait IteratorSingle: Iterator {
    fn single(self) -> Result<<Self as Iterator>::Item, ElementQuantity>;
}

impl<T: Iterator> IteratorSingle for T { 
    fn single(mut self) -> Result<<Self as Iterator>::Item, ElementQuantity> {
        if let Some(first) = self.next() {
            if self.next().is_some() {
                Err(ElementQuantity::Multiple)
            } else {
                Ok(first)
            }
        } else {
            Err(ElementQuantity::Zero)
        }
    }
}