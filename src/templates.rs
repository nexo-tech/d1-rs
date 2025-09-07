use tera::Tera;

pub fn init_templates() -> Tera {
    let mut tera = Tera::default();
    
    // Base layout template
    tera.add_raw_template("base.html", BASE_TEMPLATE).unwrap();
    
    // Page templates
    tera.add_raw_template("login.html", LOGIN_TEMPLATE).unwrap();
    tera.add_raw_template("register.html", REGISTER_TEMPLATE).unwrap();
    tera.add_raw_template("dashboard.html", DASHBOARD_TEMPLATE).unwrap();
    tera.add_raw_template("calendar_list.html", CALENDAR_LIST_TEMPLATE).unwrap();
    tera.add_raw_template("calendar_form.html", CALENDAR_FORM_TEMPLATE).unwrap();
    tera.add_raw_template("calendar_detail.html", CALENDAR_DETAIL_TEMPLATE).unwrap();
    tera.add_raw_template("public_calendar.html", PUBLIC_CALENDAR_TEMPLATE).unwrap();
    tera.add_raw_template("booking_success.html", BOOKING_SUCCESS_TEMPLATE).unwrap();
    tera.add_raw_template("working_hours.html", WORKING_HOURS_TEMPLATE).unwrap();
    
    tera
}

const BASE_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}Calendar Booking System{% endblock %}</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f5f5;
            color: #333;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        .nav {
            background: white;
            padding: 1rem 2rem;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
            margin-bottom: 2rem;
        }
        .nav-inner {
            max-width: 1200px;
            margin: 0 auto;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        .nav a {
            color: #333;
            text-decoration: none;
            margin: 0 1rem;
        }
        .nav a:hover {
            color: #4CAF50;
        }
        .card {
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 2rem;
            margin-bottom: 2rem;
        }
        .btn {
            display: inline-block;
            padding: 0.75rem 1.5rem;
            background: #4CAF50;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            text-decoration: none;
            font-size: 1rem;
        }
        .btn:hover {
            background: #45a049;
        }
        .btn-secondary {
            background: #6c757d;
        }
        .btn-secondary:hover {
            background: #5a6268;
        }
        .btn-danger {
            background: #dc3545;
        }
        .btn-danger:hover {
            background: #c82333;
        }
        .form-group {
            margin-bottom: 1.5rem;
        }
        label {
            display: block;
            margin-bottom: 0.5rem;
            font-weight: 500;
        }
        input, select, textarea {
            width: 100%;
            padding: 0.75rem;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 1rem;
        }
        input:focus, select:focus, textarea:focus {
            outline: none;
            border-color: #4CAF50;
        }
        .alert {
            padding: 1rem;
            border-radius: 4px;
            margin-bottom: 1rem;
        }
        .alert-success {
            background: #d4edda;
            color: #155724;
            border: 1px solid #c3e6cb;
        }
        .alert-error {
            background: #f8d7da;
            color: #721c24;
            border: 1px solid #f5c6cb;
        }
        .slot-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
            gap: 1rem;
            margin: 1rem 0;
        }
        .slot-btn {
            padding: 0.75rem;
            border: 2px solid #ddd;
            background: white;
            border-radius: 4px;
            cursor: pointer;
            text-align: center;
        }
        .slot-btn:hover {
            border-color: #4CAF50;
            background: #f0f8f0;
        }
        .slot-btn.selected {
            border-color: #4CAF50;
            background: #4CAF50;
            color: white;
        }
        table {
            width: 100%;
            border-collapse: collapse;
        }
        th, td {
            padding: 0.75rem;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background: #f8f9fa;
            font-weight: 600;
        }
    </style>
    {% block styles %}{% endblock %}
</head>
<body>
    {% if user %}
    <nav class="nav">
        <div class="nav-inner">
            <div>
                <a href="/dashboard">Dashboard</a>
                <a href="/dashboard/calendars">Calendars</a>
            </div>
            <div>
                <span>{{ user.email }}</span>
                <a href="/api/logout" onclick="return confirm('Are you sure you want to logout?')">Logout</a>
            </div>
        </div>
    </nav>
    {% endif %}
    
    <div class="container">
        {% block content %}{% endblock %}
    </div>
    
    {% block scripts %}{% endblock %}
</body>
</html>
"#;

const LOGIN_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Login - Calendar System{% endblock %}

{% block content %}
<div class="card" style="max-width: 400px; margin: 4rem auto;">
    <h1 style="margin-bottom: 2rem; text-align: center;">Login</h1>
    
    {% if error %}
    <div class="alert alert-error">{{ error }}</div>
    {% endif %}
    
    <form method="POST" action="/api/login">
        <div class="form-group">
            <label for="email">Email</label>
            <input type="email" id="email" name="email" required>
        </div>
        
        <div class="form-group">
            <label for="password">Password</label>
            <input type="password" id="password" name="password" required>
        </div>
        
        <button type="submit" class="btn" style="width: 100%;">Login</button>
    </form>
    
    <p style="text-align: center; margin-top: 1rem;">
        Don't have an account? <a href="/register">Register</a>
    </p>
</div>
{% endblock %}
"#;

const REGISTER_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Register - Calendar System{% endblock %}

{% block content %}
<div class="card" style="max-width: 400px; margin: 4rem auto;">
    <h1 style="margin-bottom: 2rem; text-align: center;">Register</h1>
    
    {% if error %}
    <div class="alert alert-error">{{ error }}</div>
    {% endif %}
    
    <form method="POST" action="/api/register">
        <div class="form-group">
            <label for="name">Name</label>
            <input type="text" id="name" name="name" required>
        </div>
        
        <div class="form-group">
            <label for="email">Email</label>
            <input type="email" id="email" name="email" required>
        </div>
        
        <div class="form-group">
            <label for="password">Password</label>
            <input type="password" id="password" name="password" required minlength="8">
        </div>
        
        <button type="submit" class="btn" style="width: 100%;">Register</button>
    </form>
    
    <p style="text-align: center; margin-top: 1rem;">
        Already have an account? <a href="/login">Login</a>
    </p>
</div>
{% endblock %}
"#;

const DASHBOARD_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Dashboard - Calendar System{% endblock %}

{% block content %}
<div class="card">
    <h1>Welcome to your Dashboard</h1>
    <p style="margin: 1rem 0;">Manage your calendars and bookings from here.</p>
    
    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 2rem; margin-top: 2rem;">
        <div class="card">
            <h2>Calendars</h2>
            <p>{{ calendar_count }} active calendars</p>
            <a href="/dashboard/calendars" class="btn" style="margin-top: 1rem;">Manage Calendars</a>
        </div>
        
        <div class="card">
            <h2>Bookings</h2>
            <p>{{ booking_count }} total bookings</p>
            <a href="/dashboard/bookings" class="btn" style="margin-top: 1rem;">View Bookings</a>
        </div>
    </div>
</div>
{% endblock %}
"#;

const CALENDAR_LIST_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Calendars - Calendar System{% endblock %}

{% block content %}
<div class="card">
    <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;">
        <h1>My Calendars</h1>
        <a href="/dashboard/calendars/new" class="btn">Create Calendar</a>
    </div>
    
    {% if calendars %}
    <table>
        <thead>
            <tr>
                <th>Name</th>
                <th>Slug</th>
                <th>Timezone</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
            {% for calendar in calendars %}
            <tr>
                <td>{{ calendar.name }}</td>
                <td>{{ calendar.slug }}</td>
                <td>{{ calendar.timezone }}</td>
                <td>
                    <a href="/dashboard/calendars/{{ calendar.id }}" class="btn btn-secondary" style="padding: 0.5rem 1rem; font-size: 0.875rem;">View</a>
                    <a href="/book/{{ calendar.slug }}" target="_blank" class="btn" style="padding: 0.5rem 1rem; font-size: 0.875rem;">Public Link</a>
                </td>
            </tr>
            {% endfor %}
        </tbody>
    </table>
    {% else %}
    <p>No calendars found. Create your first calendar to get started.</p>
    {% endif %}
</div>
{% endblock %}
"#;

const CALENDAR_FORM_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Create Calendar - Calendar System{% endblock %}

{% block content %}
<div class="card" style="max-width: 600px; margin: 0 auto;">
    <h1>Create New Calendar</h1>
    
    {% if error %}
    <div class="alert alert-error">{{ error }}</div>
    {% endif %}
    
    <form method="POST" action="/api/calendars">
        <div class="form-group">
            <label for="name">Calendar Name</label>
            <input type="text" id="name" name="name" required>
        </div>
        
        <div class="form-group">
            <label for="slug">URL Slug</label>
            <input type="text" id="slug" name="slug" pattern="[a-z0-9-]+" required>
            <small style="color: #666;">Lowercase letters, numbers, and hyphens only</small>
        </div>
        
        <div class="form-group">
            <label for="timezone">Timezone</label>
            <select id="timezone" name="timezone" required>
                <option value="America/New_York">America/New_York</option>
                <option value="America/Chicago">America/Chicago</option>
                <option value="America/Denver">America/Denver</option>
                <option value="America/Los_Angeles">America/Los_Angeles</option>
                <option value="Europe/London">Europe/London</option>
                <option value="Europe/Paris">Europe/Paris</option>
                <option value="Asia/Tokyo">Asia/Tokyo</option>
                <option value="Australia/Sydney">Australia/Sydney</option>
            </select>
        </div>
        
        <div class="form-group">
            <label for="duration">Appointment Duration (minutes)</label>
            <input type="number" id="duration" name="duration" value="30" min="15" max="480" required>
        </div>
        
        <div class="form-group">
            <label for="buffer">Buffer Time (minutes)</label>
            <input type="number" id="buffer" name="buffer" value="0" min="0" max="60">
        </div>
        
        <button type="submit" class="btn">Create Calendar</button>
        <a href="/dashboard/calendars" class="btn btn-secondary">Cancel</a>
    </form>
</div>
{% endblock %}
"#;

const CALENDAR_DETAIL_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}{{ calendar.name }} - Calendar System{% endblock %}

{% block content %}
<div class="card">
    <h1>{{ calendar.name }}</h1>
    
    <div style="margin: 2rem 0;">
        <p><strong>Slug:</strong> {{ calendar.slug }}</p>
        <p><strong>Timezone:</strong> {{ calendar.timezone }}</p>
        <p><strong>Duration:</strong> {{ calendar.duration }} minutes</p>
        <p><strong>Buffer:</strong> {{ calendar.buffer }} minutes</p>
        <p><strong>Public URL:</strong> <a href="/book/{{ calendar.slug }}" target="_blank">/book/{{ calendar.slug }}</a></p>
    </div>
    
    <div style="margin-top: 2rem;">
        <a href="/dashboard/calendars/{{ calendar.id }}/working-hours" class="btn">Set Working Hours</a>
        <a href="/dashboard/calendars/{{ calendar.id }}/edit" class="btn btn-secondary">Edit Calendar</a>
        <button onclick="deleteCalendar('{{ calendar.id }}')" class="btn btn-danger">Delete Calendar</button>
    </div>
</div>

<script>
function deleteCalendar(id) {
    if (confirm('Are you sure you want to delete this calendar?')) {
        fetch(`/api/calendars/${id}`, { method: 'DELETE' })
            .then(response => {
                if (response.ok) {
                    window.location.href = '/dashboard/calendars';
                } else {
                    alert('Failed to delete calendar');
                }
            });
    }
}
</script>
{% endblock %}
"#;

const PUBLIC_CALENDAR_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Book Appointment - {{ calendar.name }}</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .booking-container {
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            max-width: 600px;
            width: 100%;
            padding: 2rem;
        }
        h1 {
            color: #333;
            margin-bottom: 0.5rem;
        }
        .calendar-info {
            color: #666;
            margin-bottom: 2rem;
        }
        .date-picker {
            margin-bottom: 2rem;
        }
        label {
            display: block;
            margin-bottom: 0.5rem;
            font-weight: 500;
            color: #333;
        }
        input, select {
            width: 100%;
            padding: 0.75rem;
            border: 2px solid #e0e0e0;
            border-radius: 8px;
            font-size: 1rem;
            transition: border-color 0.3s;
        }
        input:focus, select:focus {
            outline: none;
            border-color: #667eea;
        }
        .slots-container {
            margin: 2rem 0;
        }
        .slots-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
            gap: 0.75rem;
        }
        .slot-btn {
            padding: 0.75rem;
            border: 2px solid #e0e0e0;
            background: white;
            border-radius: 8px;
            cursor: pointer;
            transition: all 0.3s;
            font-size: 0.95rem;
        }
        .slot-btn:hover {
            border-color: #667eea;
            background: #f5f3ff;
        }
        .slot-btn.selected {
            border-color: #667eea;
            background: #667eea;
            color: white;
        }
        .booking-form {
            margin-top: 2rem;
            padding-top: 2rem;
            border-top: 2px solid #e0e0e0;
        }
        .form-group {
            margin-bottom: 1.5rem;
        }
        .btn-book {
            width: 100%;
            padding: 1rem;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 1.1rem;
            font-weight: 600;
            cursor: pointer;
            transition: transform 0.3s;
        }
        .btn-book:hover {
            transform: translateY(-2px);
        }
        .btn-book:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }
    </style>
</head>
<body>
    <div class="booking-container">
        <h1>{{ calendar.name }}</h1>
        <p class="calendar-info">{{ calendar.duration }} minute appointment</p>
        
        <div class="date-picker">
            <label for="date">Select Date</label>
            <input type="date" id="date" min="{{ today }}" max="{{ max_date }}" value="{{ today }}">
        </div>
        
        <div class="slots-container">
            <label>Available Times</label>
            <div id="slots" class="slots-grid">
                <p style="color: #666;">Select a date to see available times</p>
            </div>
        </div>
        
        <div class="booking-form" style="display: none;" id="bookingForm">
            <h2 style="margin-bottom: 1rem;">Your Information</h2>
            <form id="bookForm">
                <div class="form-group">
                    <label for="name">Name</label>
                    <input type="text" id="name" name="name" required>
                </div>
                
                <div class="form-group">
                    <label for="email">Email</label>
                    <input type="email" id="email" name="email" required>
                </div>
                
                <div class="form-group">
                    <label for="notes">Notes (optional)</label>
                    <input type="text" id="notes" name="notes">
                </div>
                
                <button type="submit" class="btn-book">Book Appointment</button>
            </form>
        </div>
    </div>
    
    <script>
        const calendarSlug = '{{ calendar.slug }}';
        let selectedSlot = null;
        
        document.getElementById('date').addEventListener('change', loadSlots);
        
        async function loadSlots() {
            const date = document.getElementById('date').value;
            const response = await fetch(`/api/slots/${calendarSlug}?date=${date}`);
            const slots = await response.json();
            
            const slotsContainer = document.getElementById('slots');
            if (slots.length === 0) {
                slotsContainer.innerHTML = '<p style="color: #666;">No available times for this date</p>';
                document.getElementById('bookingForm').style.display = 'none';
                return;
            }
            
            slotsContainer.innerHTML = slots.map(slot => 
                `<button class="slot-btn" onclick="selectSlot('${slot}')">${formatTime(slot)}</button>`
            ).join('');
        }
        
        function selectSlot(slot) {
            selectedSlot = slot;
            document.querySelectorAll('.slot-btn').forEach(btn => {
                btn.classList.remove('selected');
                if (btn.textContent === formatTime(slot)) {
                    btn.classList.add('selected');
                }
            });
            document.getElementById('bookingForm').style.display = 'block';
        }
        
        function formatTime(datetime) {
            const date = new Date(datetime);
            return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
        }
        
        document.getElementById('bookForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const formData = {
                name: document.getElementById('name').value,
                email: document.getElementById('email').value,
                notes: document.getElementById('notes').value,
                slot: selectedSlot
            };
            
            const response = await fetch(`/api/book/${calendarSlug}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(formData)
            });
            
            if (response.ok) {
                const result = await response.json();
                window.location.href = `/booking/${result.id}/success`;
            } else {
                alert('Failed to book appointment. Please try again.');
            }
        });
        
        // Load initial slots
        loadSlots();
    </script>
</body>
</html>
"#;

const BOOKING_SUCCESS_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Booking Confirmed</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .success-container {
            background: white;
            border-radius: 12px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            max-width: 500px;
            width: 100%;
            padding: 3rem;
            text-align: center;
        }
        .success-icon {
            width: 80px;
            height: 80px;
            background: #4CAF50;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 2rem;
        }
        .success-icon::after {
            content: '✓';
            color: white;
            font-size: 3rem;
        }
        h1 {
            color: #333;
            margin-bottom: 1rem;
        }
        .booking-details {
            background: #f8f9fa;
            border-radius: 8px;
            padding: 1.5rem;
            margin: 2rem 0;
            text-align: left;
        }
        .detail-row {
            display: flex;
            justify-content: space-between;
            margin-bottom: 0.75rem;
        }
        .detail-label {
            color: #666;
        }
        .detail-value {
            font-weight: 600;
            color: #333;
        }
    </style>
</head>
<body>
    <div class="success-container">
        <div class="success-icon"></div>
        <h1>Booking Confirmed!</h1>
        <p>Your appointment has been successfully booked.</p>
        
        <div class="booking-details">
            <div class="detail-row">
                <span class="detail-label">Date & Time:</span>
                <span class="detail-value">{{ booking.datetime }}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Duration:</span>
                <span class="detail-value">{{ booking.duration }} minutes</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Name:</span>
                <span class="detail-value">{{ booking.name }}</span>
            </div>
            <div class="detail-row">
                <span class="detail-label">Email:</span>
                <span class="detail-value">{{ booking.email }}</span>
            </div>
        </div>
        
        <p style="color: #666;">A confirmation email has been sent to your email address.</p>
    </div>
</body>
</html>
"#;

const WORKING_HOURS_TEMPLATE: &str = r#"
{% extends "base.html" %}

{% block title %}Working Hours - {{ calendar.name }}{% endblock %}

{% block content %}
<div class="card" style="max-width: 600px; margin: 0 auto;">
    <h1>Working Hours for {{ calendar.name }}</h1>
    
    <form method="POST" action="/api/calendars/{{ calendar.id }}/working-hours">
        {% for day in ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"] %}
        <div class="form-group">
            <label>
                <input type="checkbox" name="days" value="{{ loop.index0 }}" 
                    {% if working_hours[loop.index0].enabled %}checked{% endif %}>
                {{ day }}
            </label>
            <div style="display: flex; gap: 1rem; margin-top: 0.5rem;">
                <input type="time" name="start_{{ loop.index0 }}" 
                    value="{{ working_hours[loop.index0].start }}" style="width: auto;">
                <input type="time" name="end_{{ loop.index0 }}" 
                    value="{{ working_hours[loop.index0].end }}" style="width: auto;">
            </div>
        </div>
        {% endfor %}
        
        <button type="submit" class="btn">Save Working Hours</button>
        <a href="/dashboard/calendars/{{ calendar.id }}" class="btn btn-secondary">Cancel</a>
    </form>
</div>
{% endblock %}
"#;