pub trait Concat<T> {
    type Output;

    fn concat(self, other: T) -> Self::Output;
}
