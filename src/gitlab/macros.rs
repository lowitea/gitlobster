// Copied from https://gitlab.kitware.com/utils/rust-gitlab/-/blob/master/src/macros.rs

macro_rules! impl_id {
    ( $name:ident, $doc:expr$(,)? ) => {
        #[derive(
            Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[doc = $doc]
        pub struct $name(u64);

        impl $name {
            /// Create a new id.
            pub const fn new(id: u64) -> Self {
                $name(id)
            }

            /// The value of the id.
            pub const fn value(&self) -> u64 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

pub(crate) use impl_id;
