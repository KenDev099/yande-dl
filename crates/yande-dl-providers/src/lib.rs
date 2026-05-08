//! Image board providers for yande-dl. Currently only [`moebooru::MoebooruProvider`]
//! (used for Yande.re and Konachan).

pub mod moebooru;

pub use moebooru::MoebooruProvider;
