pub mod user;
pub mod session;
pub mod calendar;
pub mod calendar_token;
pub mod calendar_working_hours;
pub mod booking;
// Legacy models for backward compatibility
pub mod token;
pub mod working_hours;

pub use user::Entity as User;
pub use session::Entity as Session;
pub use calendar::Entity as Calendar;
pub use calendar_token::Entity as CalendarToken;
pub use calendar_working_hours::Entity as CalendarWorkingHours;
pub use booking::Entity as Booking;
// Legacy exports
pub use token::Entity as Token;
pub use working_hours::Entity as WorkingHours;