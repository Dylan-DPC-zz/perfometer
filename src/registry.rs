use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use crate::Counter;
use std::error::Error;
use std::fmt::{Display, Formatter, Error as FmtError };

#[derive(Debug, Default)]
pub struct Registry (pub Arc<HashMap<String,Box<dyn Counter>>>);

impl Deref for Registry {
    type Target = HashMap<String, Box<dyn Counter>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Registry {
    pub fn create() -> Self {
         Registry::default()
    }

    pub fn from_entries(entries: HashMap<String, Box<dyn Counter>>) -> Self {
        let counters = entries.into_iter().map(|entry| entry).collect();

        Registry(Arc::new(counters))
    }

    pub fn count(&self, name: &str) -> Result<(), CounterNotFoundError>{
        let counter = self.0.get(name).ok_or_else(|| CounterNotFoundError {counter: name.to_owned()})?;

        Ok(())

    }

}

#[derive(Debug, Default)]
pub struct Entry(pub HashMap<String, Box<dyn Counter>>);

impl Deref for Entry {
    type Target = HashMap<String, Box<dyn Counter>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Entry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Entry {
    pub fn add_counter<T: Counter + Default + 'static>(&mut self, name: &str) {
        self.0.insert(name.to_owned(), Box::new(T::default()));
    }

    pub fn bind(self) -> Registry {
        Registry::from_entries(self.0)
    }

}

#[derive(Clone, Debug)]
pub struct CounterNotFoundError {
    counter: String
}

impl Error for CounterNotFoundError {}

impl Display for CounterNotFoundError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{} not found", self.counter)
    }
}