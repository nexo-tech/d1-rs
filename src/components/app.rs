use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::{auth::*, booking::*, calendar_management::*, public_calendar::*};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html lang="en"/>
        <Title text="Calendar App"/>
        <Meta name="description" content="Multi-tenant calendar booking system"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        
        <Router>
            <div class="min-h-screen bg-gray-50">
                <Routes>
                    // Public calendar booking routes
                    <Route path="/book/:slug" view=PublicCalendarView/>
                    <Route path="/booking/:id/success" view=BookingSuccessView/>
                    
                    // Authentication routes  
                    <Route path="/login" view=LoginView/>
                    <Route path="/register" view=RegisterView/>
                    <Route path="/auth/callback" view=AuthCallbackView/>
                    
                    // Protected backoffice routes
                    <Route path="/dashboard" view=move || view! {
                        <ProtectedRoute>
                            <DashboardLayout>
                                <DashboardHome/>
                            </DashboardLayout>
                        </ProtectedRoute>
                    }/>
                    
                    <Route path="/dashboard/calendars" view=move || view! {
                        <ProtectedRoute>
                            <DashboardLayout>
                                <CalendarList/>
                            </DashboardLayout>
                        </ProtectedRoute>
                    }/>
                    
                    <Route path="/dashboard/calendars/new" view=move || view! {
                        <ProtectedRoute>
                            <DashboardLayout>
                                <CreateCalendar/>
                            </DashboardLayout>
                        </ProtectedRoute>
                    }/>
                    
                    <Route path="/dashboard/calendars/:id" view=move || view! {
                        <ProtectedRoute>
                            <DashboardLayout>
                                <CalendarDetail/>
                            </DashboardLayout>
                        </ProtectedRoute>
                    }/>
                    
                    <Route path="/dashboard/bookings" view=move || view! {
                        <ProtectedRoute>
                            <DashboardLayout>
                                <BookingManagement/>
                            </DashboardLayout>
                        </ProtectedRoute>
                    }/>
                    
                    // Default route redirects to login or dashboard
                    <Route path="/" view=HomeView/>
                    
                    // 404 page
                    <Route path="/*any" view=NotFoundView/>
                </Routes>
            </div>
        </Router>
    }
}

#[component]
fn HomeView() -> impl IntoView {
    let user = use_context::<Signal<Option<UserInfo>>>().unwrap_or_default();
    
    create_effect(move |_| {
        if let Some(_) = user.get() {
            let navigate = use_navigate();
            navigate("/dashboard", Default::default());
        } else {
            let navigate = use_navigate();
            navigate("/login", Default::default());
        }
    });

    view! {
        <div class="flex items-center justify-center min-h-screen">
            <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-primary-600"></div>
        </div>
    }
}

#[component]
fn NotFoundView() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50">
            <div class="max-w-md w-full space-y-8 text-center">
                <div>
                    <h2 class="mt-6 text-3xl font-extrabold text-gray-900">
                        "404 - Page Not Found"
                    </h2>
                    <p class="mt-2 text-sm text-gray-600">
                        "The page you're looking for doesn't exist."
                    </p>
                </div>
                <div>
                    <A href="/" class="btn-primary">
                        "Go Home"
                    </A>
                </div>
            </div>
        </div>
    }
}

#[derive(Clone, Debug)]
pub struct UserInfo {
    pub id: i32,
    pub email: String,
    pub full_name: String,
    pub company_name: Option<String>,
    pub is_verified: bool,
}