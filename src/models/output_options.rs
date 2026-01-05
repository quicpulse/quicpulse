//! What to print (H/B/h/b/m)

/// Output options determining what parts of the message to print
pub struct OutputOptions {
    pub headers: bool,
    pub body: bool,
    pub meta: bool,
}

impl OutputOptions {
    pub fn any(&self) -> bool {
        self.headers || self.body || self.meta
    }
}
