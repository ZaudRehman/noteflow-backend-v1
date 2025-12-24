// src/services/mod.rs
pub mod auth_service;
pub mod collaboration_service;
pub mod note_service;
pub mod tag_service;
pub mod user_service;

pub use auth_service::AuthService;
pub use collaboration_service::CollaborationService;
pub use note_service::NoteService;
pub use tag_service::TagService;
pub use user_service::UserService;
