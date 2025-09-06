// This module contains booking-related components that are shared
// between the public calendar booking interface and the admin booking management

use leptos::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// Re-export from public_calendar for now since the main booking logic is there
pub use crate::components::public_calendar::{BookingInfo, TimeSlotInfo};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookingListItem {
    pub id: i32,
    pub calendar_name: String,
    pub guest_name: String,
    pub guest_email: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[component]
pub fn BookingCard(booking: BookingListItem) -> impl IntoView {
    let status_color = match booking.status.as_str() {
        "confirmed" => "bg-green-100 text-green-800",
        "pending" => "bg-yellow-100 text-yellow-800", 
        "cancelled" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    };

    view! {
        <div class="card hover:shadow-md transition-shadow duration-200">
            <div class="flex justify-between items-start">
                <div class="flex-1">
                    <div class="flex items-center space-x-2">
                        <h3 class="text-lg font-medium text-gray-900">
                            {booking.title}
                        </h3>
                        <span class={format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_color)}>
                            {booking.status.clone()}
                        </span>
                    </div>
                    
                    <div class="mt-2 space-y-1 text-sm text-gray-600">
                        <p class="flex items-center">
                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
                            </svg>
                            {booking.guest_name} " (" {booking.guest_email} ")"
                        </p>
                        
                        <p class="flex items-center">
                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            {booking.start_time.format("%B %d, %Y at %I:%M %p").to_string()}
                        </p>
                        
                        <p class="flex items-center">
                            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                            </svg>
                            {booking.calendar_name}
                        </p>
                    </div>
                </div>
                
                <div class="flex items-center space-x-2 ml-4">
                    {if booking.status == "confirmed" {
                        view! {
                            <button class="btn-danger text-sm">
                                "Cancel"
                            </button>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn BookingList(bookings: Vec<BookingListItem>) -> impl IntoView {
    if bookings.is_empty() {
        view! {
            <div class="text-center py-12">
                <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                </svg>
                <h3 class="mt-2 text-sm font-medium text-gray-900">"No bookings yet"</h3>
                <p class="mt-1 text-sm text-gray-500">"Your bookings will appear here."</p>
            </div>
        }.into_view()
    } else {
        view! {
            <div class="space-y-4">
                {bookings.into_iter().map(|booking| {
                    view! { <BookingCard booking=booking/> }
                }).collect::<Vec<_>>()}
            </div>
        }.into_view()
    }
}