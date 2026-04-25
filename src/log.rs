use std::fmt;

pub trait Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

pub struct Wrapper<'a, T: Display>(&'a T);

impl<'a, T: Display> fmt::Display for Wrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub trait Ext: Display {
    fn log(&self) -> Wrapper<'_, Self>
    where
        Self: Sized,
    {
        Wrapper(self)
    }
}

impl<T: Display> Ext for T {}
