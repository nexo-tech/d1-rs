use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

#[component]
pub fn PublicCalendarView() -> impl IntoView {
    let params = use_params_map();
    let calendar_slug = move || {
        params.with(|params| params.get("slug").cloned())
    };

    let calendar_resource = create_local_resource(
        calendar_slug,
        |slug| async move {
            if let Some(slug) = slug {
                get_public_calendar(slug).await
            } else {
                Err(ServerFnError::new("No calendar slug provided"))
            }
        }
    );

    view! {
        <div class="min-h-screen bg-gray-50">
            <Suspense fallback=move || view! { 
                <div class="flex items-center justify-center min-h-screen">
                    <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-primary-600"></div>
                </div>
            }>
                {move || {
                    calendar_resource.get().map(|calendar| {
                        match calendar {
                            Ok(cal) => view! { <PublicCalendarContent calendar=cal/> }.into_view(),
                            Err(e) => view! { 
                                <div class="flex items-center justify-center min-h-screen">
                                    <div class="max-w-md w-full space-y-8 text-center">
                                        <div>
                                            <h2 class="mt-6 text-3xl font-extrabold text-gray-900">
                                                "Calendar Not Found"
                                            </h2>
                                            <p class="mt-2 text-sm text-gray-600">
                                                {e.to_string()}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            }.into_view(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn PublicCalendarContent(calendar: PublicCalendarInfo) -> impl IntoView {
    let (selected_date, set_selected_date) = create_signal(chrono::Utc::now().date_naive());
    let (selected_slot, set_selected_slot) = create_signal(None::<TimeSlotInfo>);
    let (step, set_step) = create_signal(BookingStep::SelectDate);

    let slots_resource = create_local_resource(
        move || (calendar.slug.clone(), selected_date.get()),
        |(slug, date)| async move {
            get_available_slots(slug, date, None).await
        }
    );

    view! {
        <div class="max-w-4xl mx-auto py-8 px-4 sm:px-6 lg:px-8">
            // Header
            <div class="text-center mb-8">
                <h1 class="text-3xl font-bold text-gray-900">{calendar.name.clone()}</h1>
                {calendar.description.clone().map(|desc| view! {
                    <p class="mt-2 text-lg text-gray-600">{desc}</p>
                })}
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-8">
                // Calendar/Date Selection
                <div class="space-y-6">
                    <div class="card">
                        <div class="card-header">
                            <h3 class="text-lg font-medium text-gray-900">"Select a Date"</h3>
                        </div>
                        <DatePicker 
                            selected_date=selected_date
                            on_date_change=set_selected_date
                        />
                    </div>

                    // Available time slots
                    {move || {
                        if step.get() >= BookingStep::SelectTime {
                            view! {
                                <div class="card">
                                    <div class="card-header">
                                        <h3 class="text-lg font-medium text-gray-900">"Available Times"</h3>
                                    </div>
                                    <Suspense fallback=move || view! { 
                                        <div class="space-y-2">
                                            {(0..6).map(|_| view! {
                                                <div class="h-10 bg-gray-200 rounded animate-pulse"></div>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }>
                                        {move || {
                                            slots_resource.get().map(|slots| {
                                                match slots {
                                                    Ok(slots) => {
                                                        if slots.is_empty() {
                                                            view! {
                                                                <div class="text-center py-8">
                                                                    <p class="text-gray-500">"No available times for this date."</p>
                                                                </div>
                                                            }.into_view()
                                                        } else {
                                                            view! {
                                                                <TimeSlotGrid 
                                                                    slots=slots
                                                                    selected_slot=selected_slot
                                                                    on_slot_select=move |slot| {
                                                                        set_selected_slot(Some(slot));
                                                                        set_step(BookingStep::EnterDetails);
                                                                    }
                                                                />
                                                            }.into_view()
                                                        }
                                                    },
                                                    Err(e) => view! {
                                                        <div class="text-center py-8 text-red-600">
                                                            "Error loading times: " {e.to_string()}
                                                        </div>
                                                    }.into_view(),
                                                }
                                            })
                                        }}
                                    </Suspense>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                </div>

                // Booking form
                <div class="space-y-6">
                    {move || {
                        match step.get() {
                            BookingStep::SelectDate => view! {
                                <div class="card">
                                    <div class="text-center py-8">
                                        <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                                        </svg>
                                        <h3 class="mt-2 text-lg font-medium text-gray-900">"Select a date to continue"</h3>
                                        <p class="mt-1 text-sm text-gray-500">"Choose a date from the calendar to see available times."</p>
                                    </div>
                                </div>
                            }.into_view(),
                            BookingStep::SelectTime => view! {
                                <div class="card">
                                    <div class="text-center py-8">
                                        <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                        </svg>
                                        <h3 class="mt-2 text-lg font-medium text-gray-900">"Select a time slot"</h3>
                                        <p class="mt-1 text-sm text-gray-500">"Choose an available time from the options on the left."</p>
                                    </div>
                                </div>
                            }.into_view(),
                            BookingStep::EnterDetails => view! {
                                <BookingForm 
                                    calendar_slug=calendar.slug.clone()
                                    selected_slot=selected_slot
                                    on_booking_complete=move |booking_id| {
                                        let navigate = use_navigate();
                                        navigate(&format!("/booking/{}/success", booking_id), Default::default());
                                    }
                                />
                            }.into_view(),
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn DatePicker(
    selected_date: ReadSignal<NaiveDate>,
    on_date_change: WriteSignal<NaiveDate>,
) -> impl IntoView {
    let today = chrono::Utc::now().date_naive();
    let (current_month, set_current_month) = create_signal(today.with_day(1).unwrap());
    
    view! {
        <div class="space-y-4">
            // Month navigation
            <div class="flex items-center justify-between">
                <button
                    class="p-2 hover:bg-gray-100 rounded-md"
                    on:click=move |_| {
                        let prev = current_month.get()
                            .checked_sub_months(chrono::Months::new(1))
                            .unwrap_or(current_month.get());
                        set_current_month(prev);
                    }
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                </button>
                <h4 class="text-lg font-medium text-gray-900">
                    {move || current_month.get().format("%B %Y").to_string()}
                </h4>
                <button
                    class="p-2 hover:bg-gray-100 rounded-md"
                    on:click=move |_| {
                        let next = current_month.get()
                            .checked_add_months(chrono::Months::new(1))
                            .unwrap_or(current_month.get());
                        set_current_month(next);
                    }
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                    </svg>
                </button>
            </div>
            
            // Calendar grid
            <div class="grid grid-cols-7 gap-1 text-center text-sm">
                // Day headers
                {["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].iter().map(|day| {
                    view! { <div class="py-2 font-medium text-gray-500">{*day}</div> }
                }).collect::<Vec<_>>()}
                
                // Calendar days (simplified - would need proper calendar logic)
                {move || {
                    let month_start = current_month.get();
                    let days_in_month = match month_start.month() {
                        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                        4 | 6 | 9 | 11 => 30,
                        2 => if month_start.year() % 4 == 0 { 29 } else { 28 },
                        _ => 30,
                    };
                    
                    (1..=days_in_month).map(|day| {
                        let date = month_start.with_day(day).unwrap();
                        let is_past = date < today;
                        let is_selected = date == selected_date.get();
                        
                        view! {
                            <button
                                class={format!("p-2 rounded-md transition-colors duration-200 {}",
                                    if is_past { "text-gray-300 cursor-not-allowed" }
                                    else if is_selected { "bg-primary-600 text-white" }
                                    else { "hover:bg-primary-50 text-gray-900" }
                                )}
                                disabled=is_past
                                on:click=move |_| {
                                    if !is_past {
                                        on_date_change(date);
                                    }
                                }
                            >
                                {day}
                            </button>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}

#[component]
fn TimeSlotGrid(
    slots: Vec<TimeSlotInfo>,
    selected_slot: ReadSignal<Option<TimeSlotInfo>>,
    on_slot_select: impl Fn(TimeSlotInfo) + 'static,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 gap-2">
            {slots.into_iter().map(|slot| {
                let slot_clone = slot.clone();
                let is_selected = move || {
                    selected_slot.get().map_or(false, |s| s.start_time == slot.start_time)
                };
                
                view! {
                    <button
                        class={move || format!("p-3 text-sm rounded-md border transition-all duration-200 {}",
                            if !slot.available { "bg-gray-100 text-gray-400 cursor-not-allowed border-gray-200" }
                            else if is_selected() { "bg-primary-600 text-white border-primary-600" }
                            else { "bg-white text-gray-900 border-gray-300 hover:border-primary-300 hover:shadow-md" }
                        )}
                        disabled=!slot.available
                        on:click={
                            let slot_for_click = slot_clone.clone();
                            move |_| {
                                if slot.available {
                                    on_slot_select(slot_for_click.clone());
                                }
                            }
                        }
                    >
                        {slot.start_time.format("%I:%M %p").to_string()}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

#[component]
fn BookingForm(
    calendar_slug: String,
    selected_slot: ReadSignal<Option<TimeSlotInfo>>,
    on_booking_complete: impl Fn(i32) + 'static,
) -> impl IntoView {
    let (booking_error, set_booking_error) = create_signal(None::<String>);
    let (is_loading, set_is_loading) = create_signal(false);
    
    let create_booking_action = create_server_action::<CreateBooking>();
    let value = create_booking_action.value();
    
    create_effect(move |_| {
        if let Some(Ok(booking)) = value.get() {
            on_booking_complete(booking.id);
        } else if let Some(Err(e)) = value.get() {
            set_booking_error(Some(e.to_string()));
            set_is_loading(false);
        }
    });

    view! {
        <div class="card">
            <div class="card-header">
                <h3 class="text-lg font-medium text-gray-900">"Booking Details"</h3>
                {move || {
                    selected_slot.get().map(|slot| view! {
                        <p class="text-sm text-gray-600 mt-1">
                            {slot.start_time.format("%A, %B %d at %I:%M %p").to_string()}
                            " - " 
                            {slot.end_time.format("%I:%M %p").to_string()}
                        </p>
                    })
                }}
            </div>
            
            <ActionForm action=create_booking_action class="space-y-4">
                <input type="hidden" name="calendar_slug" value=calendar_slug/>
                {move || {
                    selected_slot.get().map(|slot| view! {
                        <>
                            <input type="hidden" name="start_time" value=slot.start_time.to_rfc3339()/>
                            <input type="hidden" name="end_time" value=slot.end_time.to_rfc3339()/>
                        </>
                    })
                }}
                
                {move || {
                    booking_error.get().map(|err| view! {
                        <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md">
                            {err}
                        </div>
                    })
                }}
                
                <div class="form-group">
                    <label for="guest_name" class="form-label">
                        "Your Name"
                    </label>
                    <input
                        id="guest_name"
                        name="guest_name"
                        type="text"
                        required
                        class="input-field"
                        placeholder="John Doe"
                    />
                </div>
                
                <div class="form-group">
                    <label for="guest_email" class="form-label">
                        "Email Address"
                    </label>
                    <input
                        id="guest_email"
                        name="guest_email"
                        type="email"
                        required
                        class="input-field"
                        placeholder="john@example.com"
                    />
                </div>
                
                <div class="form-group">
                    <label for="title" class="form-label">
                        "Meeting Title"
                    </label>
                    <input
                        id="title"
                        name="title"
                        type="text"
                        required
                        class="input-field"
                        placeholder="30-minute meeting"
                    />
                </div>
                
                <div class="form-group">
                    <label for="description" class="form-label">
                        "Additional Notes (optional)"
                    </label>
                    <textarea
                        id="description"
                        name="description"
                        rows="3"
                        class="input-field"
                        placeholder="Any additional information..."
                    ></textarea>
                </div>
                
                <button
                    type="submit"
                    class="w-full btn-primary"
                    disabled=move || is_loading.get()
                >
                    {move || if is_loading.get() {
                        "Booking..."
                    } else {
                        "Book Appointment"
                    }}
                </button>
            </ActionForm>
        </div>
    }
}

#[component]
pub fn BookingSuccessView() -> impl IntoView {
    let params = use_params_map();
    let booking_id = move || {
        params.with(|params| {
            params.get("id")
                .and_then(|id| id.parse::<i32>().ok())
        })
    };

    view! {
        <div class="min-h-screen bg-gray-50 flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8 text-center">
                <div>
                    <div class="mx-auto flex items-center justify-center h-16 w-16 rounded-full bg-green-100">
                        <svg class="h-8 w-8 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                        </svg>
                    </div>
                    <h2 class="mt-6 text-3xl font-extrabold text-gray-900">
                        "Booking Confirmed!"
                    </h2>
                    <p class="mt-2 text-sm text-gray-600">
                        "You should receive a calendar invitation shortly."
                    </p>
                    {move || {
                        booking_id().map(|id| view! {
                            <p class="mt-2 text-xs text-gray-500">
                                "Booking ID: " {id}
                            </p>
                        })
                    }}
                </div>
                <div class="space-y-4">
                    <p class="text-sm text-gray-600">
                        "If you need to make changes or cancel this booking, please contact the organizer directly."
                    </p>
                </div>
            </div>
        </div>
    }
}

// Types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicCalendarInfo {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub timezone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeSlotInfo {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookingInfo {
    pub id: i32,
    pub guest_name: String,
    pub guest_email: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum BookingStep {
    SelectDate = 1,
    SelectTime = 2,
    EnterDetails = 3,
}

// Server Actions
#[server(GetPublicCalendar, "/api")]
pub async fn get_public_calendar(slug: String) -> Result<PublicCalendarInfo, ServerFnError> {
    use crate::server::use_app_state;
    
    let state = use_app_state();
    
    match state.calendar_service.get_calendar_by_slug(&slug).await {
        Ok(calendar) => {
            Ok(PublicCalendarInfo {
                name: calendar.name,
                slug: calendar.slug,
                description: calendar.description,
                timezone: calendar.timezone,
            })
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

#[server(GetAvailableSlots, "/api")]
pub async fn get_available_slots(
    slug: String, 
    date: NaiveDate, 
    timezone: Option<String>
) -> Result<Vec<TimeSlotInfo>, ServerFnError> {
    use crate::server::use_app_state;
    use crate::server::booking_service::AvailabilityRequest;
    
    let state = use_app_state();
    
    let availability_req = AvailabilityRequest {
        date,
        timezone,
        duration_minutes: Some(30), // Default 30-minute slots
    };
    
    match state.booking_service.get_available_slots(&slug, availability_req).await {
        Ok(slots) => {
            let slot_infos: Vec<TimeSlotInfo> = slots
                .into_iter()
                .map(|slot| TimeSlotInfo {
                    start_time: DateTime::from_utc(slot.start_time.naive_utc(), Utc),
                    end_time: DateTime::from_utc(slot.end_time.naive_utc(), Utc),
                    available: slot.available,
                })
                .collect();
            Ok(slot_infos)
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

#[server(CreateBooking, "/api")]
pub async fn create_booking(
    calendar_slug: String,
    guest_name: String,
    guest_email: String,
    title: String,
    description: Option<String>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<BookingInfo, ServerFnError> {
    use crate::server::use_app_state;
    use crate::server::booking_service::CreateBookingRequest;
    
    let state = use_app_state();
    
    let booking_req = CreateBookingRequest {
        guest_name: guest_name.clone(),
        guest_email: guest_email.clone(),
        title: title.clone(),
        description,
        start_time,
        end_time,
        timezone: None,
    };
    
    match state.booking_service.create_booking(&calendar_slug, booking_req).await {
        Ok(booking) => {
            Ok(BookingInfo {
                id: booking.id,
                guest_name: booking.guest_name,
                guest_email: booking.guest_email,
                title: booking.title,
                start_time: booking.start_time,
                end_time: booking.end_time,
            })
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}