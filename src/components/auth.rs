use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use web_sys::SubmitEvent;

use crate::components::app::UserInfo;

#[component]
pub fn LoginView() -> impl IntoView {
    let (login_error, set_login_error) = create_signal(None::<String>);
    let (is_loading, set_is_loading) = create_signal(false);
    
    let login_action = create_server_action::<LoginUser>();
    let value = login_action.value();
    
    create_effect(move |_| {
        if let Some(Ok(user)) = value.get() {
            // Store user in context and navigate to dashboard
            provide_context(create_signal(Some(user.clone())).0);
            let navigate = use_navigate();
            navigate("/dashboard", Default::default());
        } else if let Some(Err(e)) = value.get() {
            set_login_error(Some(e.to_string()));
            set_is_loading(false);
        }
    });

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        set_is_loading(true);
        set_login_error(None);
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        "Sign in to your account"
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        "Or "
                        <A href="/register" class="font-medium text-primary-600 hover:text-primary-500">
                            "create a new account"
                        </A>
                    </p>
                </div>
                
                <ActionForm action=login_action on:submit=on_submit class="mt-8 space-y-6">
                    {move || {
                        login_error.get().map(|err| view! {
                            <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md">
                                {err}
                            </div>
                        })
                    }}
                    
                    <div class="space-y-4">
                        <div class="form-group">
                            <label for="email" class="form-label">
                                "Email address"
                            </label>
                            <input
                                id="email"
                                name="email"
                                type="email"
                                autocomplete="email"
                                required
                                class="input-field"
                                placeholder="you@example.com"
                            />
                        </div>
                        
                        <div class="form-group">
                            <label for="password" class="form-label">
                                "Password"
                            </label>
                            <input
                                id="password"
                                name="password"
                                type="password"
                                autocomplete="current-password"
                                required
                                class="input-field"
                                placeholder="••••••••"
                            />
                        </div>
                    </div>

                    <div>
                        <button
                            type="submit"
                            class="group relative w-full btn-primary"
                            disabled=move || is_loading.get()
                        >
                            {move || if is_loading.get() {
                                view! { <span>"Signing in..."</span> }
                            } else {
                                view! { <span>"Sign in"</span> }
                            }}
                        </button>
                    </div>
                </ActionForm>
            </div>
        </div>
    }
}

#[component]
pub fn RegisterView() -> impl IntoView {
    let (register_error, set_register_error) = create_signal(None::<String>);
    let (is_loading, set_is_loading) = create_signal(false);
    
    let register_action = create_server_action::<RegisterUser>();
    let value = register_action.value();
    
    create_effect(move |_| {
        if let Some(Ok(user)) = value.get() {
            // Store user in context and navigate to dashboard
            provide_context(create_signal(Some(user.clone())).0);
            let navigate = use_navigate();
            navigate("/dashboard", Default::default());
        } else if let Some(Err(e)) = value.get() {
            set_register_error(Some(e.to_string()));
            set_is_loading(false);
        }
    });

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        set_is_loading(true);
        set_register_error(None);
    };

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        "Create your account"
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        "Or "
                        <A href="/login" class="font-medium text-primary-600 hover:text-primary-500">
                            "sign in to existing account"
                        </A>
                    </p>
                </div>
                
                <ActionForm action=register_action on:submit=on_submit class="mt-8 space-y-6">
                    {move || {
                        register_error.get().map(|err| view! {
                            <div class="bg-red-50 border border-red-200 text-red-600 px-4 py-3 rounded-md">
                                {err}
                            </div>
                        })
                    }}
                    
                    <div class="space-y-4">
                        <div class="form-group">
                            <label for="full_name" class="form-label">
                                "Full Name"
                            </label>
                            <input
                                id="full_name"
                                name="full_name"
                                type="text"
                                required
                                class="input-field"
                                placeholder="John Doe"
                            />
                        </div>
                        
                        <div class="form-group">
                            <label for="email" class="form-label">
                                "Email address"
                            </label>
                            <input
                                id="email"
                                name="email"
                                type="email"
                                autocomplete="email"
                                required
                                class="input-field"
                                placeholder="you@example.com"
                            />
                        </div>
                        
                        <div class="form-group">
                            <label for="company_name" class="form-label">
                                "Company Name (optional)"
                            </label>
                            <input
                                id="company_name"
                                name="company_name"
                                type="text"
                                class="input-field"
                                placeholder="Acme Inc."
                            />
                        </div>
                        
                        <div class="form-group">
                            <label for="password" class="form-label">
                                "Password"
                            </label>
                            <input
                                id="password"
                                name="password"
                                type="password"
                                autocomplete="new-password"
                                required
                                class="input-field"
                                placeholder="••••••••"
                            />
                        </div>
                    </div>

                    <div>
                        <button
                            type="submit"
                            class="group relative w-full btn-primary"
                            disabled=move || is_loading.get()
                        >
                            {move || if is_loading.get() {
                                view! { <span>"Creating account..."</span> }
                            } else {
                                view! { <span>"Create account"</span> }
                            }}
                        </button>
                    </div>
                </ActionForm>
            </div>
        </div>
    }
}

#[component]
pub fn ProtectedRoute(children: Children) -> impl IntoView {
    let user = use_context::<Signal<Option<UserInfo>>>().unwrap_or_default();
    
    create_effect(move |_| {
        if user.get().is_none() {
            let navigate = use_navigate();
            navigate("/login", Default::default());
        }
    });
    
    move || {
        if user.get().is_some() {
            children().into_view()
        } else {
            view! {
                <div class="flex items-center justify-center min-h-screen">
                    <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-primary-600"></div>
                </div>
            }.into_view()
        }
    }
}

#[component]
pub fn AuthCallbackView() -> impl IntoView {
    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50">
            <div class="max-w-md w-full space-y-8 text-center">
                <div>
                    <h2 class="mt-6 text-3xl font-extrabold text-gray-900">
                        "Connecting your calendar..."
                    </h2>
                    <div class="mt-4">
                        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
                    </div>
                </div>
            </div>
        </div>
    }
}

// Server Actions
#[server(LoginUser, "/api")]
pub async fn login_user(email: String, password: String) -> Result<UserInfo, ServerFnError> {
    use crate::server::{use_app_state, AuthService};
    
    let state = use_app_state();
    
    let login_req = crate::server::auth::LoginRequest {
        email,
        password,
    };
    
    match state.auth_service.login(login_req).await {
        Ok(auth_response) => {
            // Convert server UserInfo to component UserInfo
            Ok(UserInfo {
                id: auth_response.user.id,
                email: auth_response.user.email,
                full_name: auth_response.user.full_name,
                company_name: auth_response.user.company_name,
                is_verified: auth_response.user.is_verified,
            })
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

#[server(RegisterUser, "/api")]
pub async fn register_user(
    full_name: String,
    email: String, 
    company_name: Option<String>,
    password: String
) -> Result<UserInfo, ServerFnError> {
    use crate::server::{use_app_state, AuthService};
    
    let state = use_app_state();
    
    let register_req = crate::server::auth::RegisterRequest {
        email,
        password,
        full_name,
        company_name,
    };
    
    match state.auth_service.register(register_req).await {
        Ok(auth_response) => {
            // Convert server UserInfo to component UserInfo
            Ok(UserInfo {
                id: auth_response.user.id,
                email: auth_response.user.email,
                full_name: auth_response.user.full_name,
                company_name: auth_response.user.company_name,
                is_verified: auth_response.user.is_verified,
            })
        },
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}