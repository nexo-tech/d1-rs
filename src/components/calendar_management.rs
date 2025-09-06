use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};

use crate::components::app::UserInfo;

#[component]
pub fn DashboardLayout(children: Children) -> impl IntoView {
    let user = use_context::<Signal<Option<UserInfo>>>().unwrap_or_default();
    let location = use_location();
    
    let logout_action = create_server_action::<LogoutUser>();
    
    // Handle logout
    create_effect(move |_| {
        if let Some(Ok(_)) = logout_action.value().get() {
            provide_context(create_signal(None::<UserInfo>).0);
            let navigate = use_navigate();
            navigate("/login", Default::default());
        }
    });

    view! {
        <div class="min-h-screen bg-gray-50">
            // Navigation
            <nav class="bg-white shadow-sm border-b border-gray-200">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="flex justify-between h-16">
                        <div class="flex">
                            <div class="flex-shrink-0 flex items-center">
                                <A href="/dashboard" class="text-xl font-bold text-gray-900">
                                    "Calendar App"
                                </A>
                            </div>
                            <div class="hidden sm:ml-6 sm:flex sm:space-x-8">
                                <A 
                                    href="/dashboard" 
                                    class=move || {
                                        if location.pathname.get() == "/dashboard" {
                                            "nav-link-active"
                                        } else {
                                            "nav-link"
                                        }
                                    }
                                >
                                    "Dashboard"
                                </A>
                                <A 
                                    href="/dashboard/calendars" 
                                    class=move || {
                                        if location.pathname.get().starts_with("/dashboard/calendars") {
                                            "nav-link-active"
                                        } else {
                                            "nav-link"
                                        }
                                    }
                                >
                                    "Calendars"
                                </A>
                                <A 
                                    href="/dashboard/bookings" 
                                    class=move || {
                                        if location.pathname.get() == "/dashboard/bookings" {
                                            "nav-link-active"
                                        } else {
                                            "nav-link"
                                        }
                                    }
                                >
                                    "Bookings"
                                </A>
                            </div>
                        </div>
                        <div class="flex items-center space-x-4">
                            {move || {
                                user.get().map(|u| view! {
                                    <div class="flex items-center space-x-4">
                                        <span class="text-sm text-gray-700">{u.full_name}</span>
                                        <ActionForm action=logout_action>
                                            <button
                                                type="submit"
                                                class="text-sm text-gray-500 hover:text-gray-700 transition-colors duration-200"
                                            >
                                                "Sign out"
                                            </button>
                                        </ActionForm>
                                    </div>
                                })
                            }}
                        </div>
                    </div>
                </div>
            </nav>
            
            // Main content
            <main class="py-6">
                <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    {children()}
                </div>
            </main>
        </div>
    }
}

#[component]
pub fn DashboardHome() -> impl IntoView {
    let user = use_context::<Signal<Option<UserInfo>>>().unwrap_or_default();
    let calendars_resource = create_local_resource(
        move || user.get(),
        |user| async move {
            if user.is_some() {
                get_user_calendars().await
            } else {
                Ok(vec![])
            }
        }
    );

    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">
                    "Dashboard"
                </h1>
                <p class="mt-1 text-sm text-gray-600">
                    "Welcome back! Here's an overview of your calendars."
                </p>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                // Quick stats cards
                <div class="card">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-primary-100 rounded-lg flex items-center justify-center">
                                <svg class="w-5 h-5 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                                </svg>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-600">"Active Calendars"</p>
                            <Suspense fallback=move || view! { <div class="h-6 bg-gray-200 rounded animate-pulse"></div> }>
                                {move || {
                                    calendars_resource.get().map(|calendars| {
                                        match calendars {
                                            Ok(cals) => view! { <p class="text-2xl font-semibold text-gray-900">{cals.len()}</p> }.into_view(),
                                            Err(_) => view! { <p class="text-2xl font-semibold text-gray-900">"--"</p> }.into_view(),
                                        }
                                    })
                                }}
                            </Suspense>
                        </div>
                    </div>
                </div>

                // Recent bookings card would go here
                <div class="card">
                    <div class="flex items-center">
                        <div class="flex-shrink-0">
                            <div class="w-8 h-8 bg-green-100 rounded-lg flex items-center justify-center">
                                <svg class="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/>
                                </svg>
                            </div>
                        </div>
                        <div class="ml-4">
                            <p class="text-sm font-medium text-gray-600">"Total Bookings"</p>
                            <p class="text-2xl font-semibold text-gray-900">"--"</p>
                        </div>
                    </div>
                </div>
            </div>

            // Quick actions
            <div class="card">
                <div class="card-header">
                    <h3 class="text-lg font-medium text-gray-900">"Quick Actions"</h3>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                    <A href="/dashboard/calendars/new" class="block p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:shadow-md transition-all duration-200">
                        <div class="flex items-center space-x-3">
                            <div class="flex-shrink-0">
                                <svg class="w-6 h-6 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                                </svg>
                            </div>
                            <div>
                                <p class="text-sm font-medium text-gray-900">"Create Calendar"</p>
                                <p class="text-sm text-gray-500">"Set up a new booking calendar"</p>
                            </div>
                        </div>
                    </A>
                    
                    <A href="/dashboard/calendars" class="block p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:shadow-md transition-all duration-200">
                        <div class="flex items-center space-x-3">
                            <div class="flex-shrink-0">
                                <svg class="w-6 h-6 text-primary-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                                </svg>
                            </div>
                            <div>
                                <p class="text-sm font-medium text-gray-900">"Manage Calendars"</p>
                                <p class="text-sm text-gray-500">"View and edit your calendars"</p>
                            </div>
                        </div>
                    </A>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn CalendarList() -> impl IntoView {
    let user = use_context::<Signal<Option<UserInfo>>>().unwrap_or_default();
    let calendars_resource = create_local_resource(
        move || user.get(),
        |user| async move {
            if user.is_some() {
                get_user_calendars().await
            } else {
                Ok(vec![])
            }
        }
    );

    view! {
        <div class="space-y-6">
            <div class="flex justify-between items-center">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">"My Calendars"</h1>
                    <p class="mt-1 text-sm text-gray-600">
                        "Manage your booking calendars and settings."
                    </p>
                </div>
                <A href="/dashboard/calendars/new" class="btn-primary">
                    "Create Calendar"
                </A>
            </div>

            <Suspense fallback=move || view! { 
                <div class="space-y-4">
                    {(0..3).map(|_| view! {
                        <div class="card animate-pulse">
                            <div class="h-4 bg-gray-200 rounded w-1/4 mb-2"></div>
                            <div class="h-3 bg-gray-200 rounded w-3/4"></div>
                        </div>
                    }).collect::<Vec<_>>()}
                </div>
            }>
                {move || {
                    calendars_resource.get().map(|calendars| {
                        match calendars {
                            Ok(cals) => {
                                if cals.is_empty() {
                                    view! {
                                        <div class="text-center py-12">
                                            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                                            </svg>
                                            <h3 class="mt-2 text-sm font-medium text-gray-900">"No calendars yet"</h3>
                                            <p class="mt-1 text-sm text-gray-500">"Get started by creating your first calendar."</p>
                                            <div class="mt-6">
                                                <A href="/dashboard/calendars/new" class="btn-primary">
                                                    "Create Calendar"
                                                </A>
                                            </div>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="space-y-4">
                                            {cals.into_iter().map(|calendar| view! {
                                                <CalendarCard calendar=calendar/>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_view()
                                }
                            },
                            Err(e) => view! {
                                <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md">
                                    "Failed to load calendars: " {e.to_string()}
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
fn CalendarCard(calendar: CalendarInfo) -> impl IntoView {
    view! {
        <div class="card hover:shadow-md transition-shadow duration-200">
            <div class="flex justify-between items-start">
                <div class="flex-1">
                    <div class="flex items-center space-x-2">
                        <h3 class="text-lg font-medium text-gray-900">
                            {calendar.name.clone()}
                        </h3>
                        <span class={format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                            if calendar.is_active { "bg-green-100 text-green-800" } else { "bg-gray-100 text-gray-800" }
                        )}>
                            {if calendar.is_active { "Active" } else { "Inactive" }}
                        </span>
                    </div>
                    {calendar.description.map(|desc| view! {
                        <p class="mt-1 text-sm text-gray-600">{desc}</p>
                    })}
                    <div class="mt-2 flex items-center space-x-4 text-sm text-gray-500">
                        <span class="flex items-center">
                            <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.102m0 0l4-4a4 4 0 105.656-5.656l-1.102 1.102m-4 4l-4 4"/>
                            </svg>
                            <code class="text-xs">{format!("/book/{}", calendar.slug)}</code>
                        </span>
                        <span class="flex items-center">
                            <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            {calendar.timezone}
                        </span>
                    </div>
                </div>
                <div class="flex items-center space-x-2">
                    <A href={format!("/dashboard/calendars/{}", calendar.id)} class="btn-secondary text-sm">
                        "Edit"
                    </A>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn CreateCalendar() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">"Create New Calendar"</h1>
                <p class="mt-1 text-sm text-gray-600">
                    "Set up a new booking calendar for your clients."
                </p>
            </div>

            <div class="max-w-2xl">
                <CalendarForm calendar=None/>
            </div>
        </div>
    }
}

#[component]
pub fn CalendarDetail() -> impl IntoView {
    let params = use_params_map();
    let calendar_id = move || {
        params.with(|params| {
            params.get("id")
                .and_then(|id| id.parse::<i32>().ok())
        })
    };

    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">"Calendar Settings"</h1>
                <p class="mt-1 text-sm text-gray-600">
                    "Manage your calendar settings and working hours."
                </p>
            </div>

            {move || {
                calendar_id().map(|id| view! {
                    <div>"Calendar ID: " {id} " - Implementation coming soon"</div>
                })
            }}
        </div>
    }
}

#[component] 
pub fn CalendarForm(calendar: Option<CalendarInfo>) -> impl IntoView {
    view! {
        <div class="card">
            <form class="space-y-6">
                <div class="form-group">
                    <label for="name" class="form-label">
                        "Calendar Name"
                    </label>
                    <input
                        id="name"
                        name="name"
                        type="text"
                        required
                        class="input-field"
                        placeholder="My Booking Calendar"
                        value={calendar.as_ref().map(|c| c.name.clone()).unwrap_or_default()}
                    />
                </div>

                <div class="form-group">
                    <label for="description" class="form-label">
                        "Description"
                    </label>
                    <textarea
                        id="description"
                        name="description"
                        rows="3"
                        class="input-field"
                        placeholder="Book a meeting with me..."
                    >{calendar.as_ref().and_then(|c| c.description.clone()).unwrap_or_default()}</textarea>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div class="form-group">
                        <label for="timezone" class="form-label">
                            "Timezone"
                        </label>
                        <select id="timezone" name="timezone" class="input-field">
                            <option value="UTC">"UTC"</option>
                            <option value="America/New_York">"Eastern Time"</option>
                            <option value="America/Chicago">"Central Time"</option>
                            <option value="America/Denver">"Mountain Time"</option>
                            <option value="America/Los_Angeles">"Pacific Time"</option>
                            <option value="Europe/London">"London"</option>
                            <option value="Europe/Paris">"Paris"</option>
                            <option value="Asia/Tokyo">"Tokyo"</option>
                        </select>
                    </div>

                    <div class="form-group">
                        <label for="booking_buffer" class="form-label">
                            "Buffer Between Bookings (minutes)"
                        </label>
                        <input
                            id="booking_buffer"
                            name="booking_buffer"
                            type="number"
                            min="0"
                            max="120"
                            class="input-field"
                            value={calendar.as_ref().map(|c| c.booking_buffer_minutes.to_string()).unwrap_or_else(|| "15".to_string())}
                        />
                    </div>
                </div>

                <div class="flex justify-end space-x-4">
                    <A href="/dashboard/calendars" class="btn-secondary">
                        "Cancel"
                    </A>
                    <button type="submit" class="btn-primary">
                        {if calendar.is_some() { "Update Calendar" } else { "Create Calendar" }}
                    </button>
                </div>
            </form>
        </div>
    }
}

#[component]
pub fn BookingManagement() -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold text-gray-900">"Booking Management"</h1>
                <p class="mt-1 text-sm text-gray-600">
                    "View and manage all your calendar bookings."
                </p>
            </div>

            <div class="card">
                <div class="text-center py-12">
                    <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/>
                    </svg>
                    <h3 class="mt-2 text-sm font-medium text-gray-900">"Booking management coming soon"</h3>
                    <p class="mt-1 text-sm text-gray-500">"This feature will be available in the next update."</p>
                </div>
            </div>
        </div>
    }
}

// Types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarInfo {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub timezone: String,
    pub is_active: bool,
    pub booking_buffer_minutes: i32,
    pub max_booking_days_ahead: i32,
    pub min_booking_notice_hours: i32,
}

// Server Actions
#[server(LogoutUser, "/api")]
pub async fn logout_user() -> Result<(), ServerFnError> {
    // TODO: Clear session cookie
    Ok(())
}

#[server(GetUserCalendars, "/api")]
pub async fn get_user_calendars() -> Result<Vec<CalendarInfo>, ServerFnError> {
    use crate::server::{use_app_state, state::get_current_user_id};
    
    let user_id = get_current_user_id()
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    
    let state = use_app_state();
    
    match state.calendar_service.get_user_calendars(user_id).await {
        Ok(calendars) => {
            let calendar_infos: Vec<CalendarInfo> = calendars
                .into_iter()
                .map(|cal| CalendarInfo {
                    id: cal.id,
                    name: cal.name,
                    slug: cal.slug,
                    description: cal.description,
                    timezone: cal.timezone,
                    is_active: cal.is_active,
                    booking_buffer_minutes: cal.booking_buffer_minutes,
                    max_booking_days_ahead: cal.max_booking_days_ahead,
                    min_booking_notice_hours: cal.min_booking_notice_hours,
                })
                .collect();
            Ok(calendar_infos)
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}