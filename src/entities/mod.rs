// Re-export all entity modules
pub mod user;
pub mod calendar;
pub mod calendar_working_hours;
pub mod booking;

pub use user::Entity as User;
pub use calendar::Entity as Calendar;
pub use calendar_working_hours::Entity as CalendarWorkingHours;
pub use booking::Entity as Booking;