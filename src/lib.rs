//! Nanoscope: a small rig for testing lossless image compression ideas honestly.
pub mod canon;
pub mod codec;
pub mod corpus;
pub mod envinfo;
pub mod ext;
pub mod gblend;
pub mod gdir;
pub mod gramp;
pub mod pkon2;
pub mod pkon2x;
pub mod pkonlearn;
pub mod pkonrel;
pub mod lisa;
/// Compatibility alias for the pre-rename module path `nanoscope::praw` (PRAW was the former name of LISA).
pub use lisa as praw;
pub mod profile;
pub mod pkonfile;
pub mod verify;
pub mod lisa_cloud;
pub mod lisa_mixer;
pub mod lisa_rp;
