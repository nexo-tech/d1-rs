pub fn home_page() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>Time Forge - Calendar Booking System</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 0; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; justify-content: center; align-items: center; min-height: 100vh; }
        .container { background: white; border-radius: 10px; padding: 40px; box-shadow: 0 20px 60px rgba(0,0,0,0.3); text-align: center; max-width: 400px; }
        h1 { color: #333; margin-bottom: 10px; }
        p { color: #666; margin-bottom: 30px; }
        .button { display: inline-block; background: #4CAF50; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; font-weight: 500; transition: background 0.3s; }
        .button:hover { background: #45a049; }
        .info { margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .info h3 { color: #333; margin-bottom: 15px; }
        .info p { font-size: 14px; color: #666; line-height: 1.6; }
    </style>
</head>
<body>
    <div class="container">
        <h1>📅 Time Forge</h1>
        <p>Simple calendar booking system with Google Calendar integration</p>
        <a href="/auth/google" class="button">Sign in with Google Calendar</a>
        
        <div class="info">
            <h3>How it works</h3>
            <p>1. Connect your Google Calendar<br>
               2. Set your working hours<br>
               3. Share your booking link<br>
               4. Let people book time with you</p>
        </div>
    </div>
</body>
</html>
    "#.to_string()
}

pub fn auth_error_page() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
    <title>Authentication Error</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; display: flex; justify-content: center; align-items: center; min-height: 100vh; }
        .container { background: white; border-radius: 10px; padding: 40px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); text-align: center; max-width: 400px; }
        h1 { color: #e74c3c; }
        a { color: #4CAF50; text-decoration: none; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Authentication Error</h1>
        <p>Failed to generate authentication URL. Please check your configuration.</p>
        <p><a href="/">← Back to Home</a></p>
    </div>
</body>
</html>
    "#.to_string()
}

pub fn auth_success_page(email: &str) -> String {
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Success - Calendar Connected</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); display: flex; justify-content: center; align-items: center; min-height: 100vh; }}
        .container {{ background: white; border-radius: 10px; padding: 40px; box-shadow: 0 20px 60px rgba(0,0,0,0.3); text-align: center; max-width: 500px; }}
        h1 {{ color: #4CAF50; margin-bottom: 20px; }}
        .email {{ background: #f0f9ff; padding: 10px 20px; border-radius: 5px; display: inline-block; font-weight: 500; color: #333; margin: 10px 0; }}
        .button-group {{ display: flex; gap: 10px; justify-content: center; margin-top: 30px; }}
        .button {{ display: inline-block; background: #4CAF50; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; font-weight: 500; transition: background 0.3s; }}
        .button:hover {{ background: #45a049; }}
        .button-secondary {{ background: #6c757d; }}
        .button-secondary:hover {{ background: #5a6268; }}
        .info {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }}
        .share-link {{ background: #f8f9fa; padding: 15px; border-radius: 5px; margin-top: 20px; word-break: break-all; }}
        .share-link code {{ color: #e83e8c; font-size: 14px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>✅ Calendar Connected Successfully!</h1>
        <p>Your Google Calendar has been connected for:</p>
        <div class="email">{}</div>
        
        <div class="info">
            <h3>🔗 Your Booking Link</h3>
            <p>Share this link with people who want to book time with you:</p>
            <div class="share-link">
                <code>/calendar/{}</code>
            </div>
        </div>
        
        <div class="button-group">
            <a href="/dashboard" class="button">Go to Dashboard</a>
            <a href="/working-hours" class="button button-secondary">Set Working Hours</a>
        </div>
    </div>
</body>
</html>
    "#, email, email)
}

pub fn dashboard_page(emails: &[String]) -> String {
    let email_list = if emails.is_empty() {
        "<p>No calendars connected yet.</p>".to_string()
    } else {
        emails.iter()
            .map(|email| format!(r#"
                <div class="calendar-item">
                    <div class="calendar-email">{}</div>
                    <div class="calendar-link">
                        <strong>Booking link:</strong> 
                        <a href="/calendar/{}" target="_blank">/calendar/{}</a>
                    </div>
                </div>
            "#, email, email, email))
            .collect::<Vec<_>>()
            .join("")
    };
    
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Dashboard - Time Forge</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 800px; margin: 0 auto; }}
        h1 {{ color: #333; margin-bottom: 30px; }}
        .section {{ background: white; border-radius: 10px; padding: 20px; margin-bottom: 20px; box-shadow: 0 2px 5px rgba(0,0,0,0.1); }}
        .button {{ display: inline-block; background: #4CAF50; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; margin-right: 10px; margin-bottom: 10px; }}
        .button:hover {{ background: #45a049; }}
        .calendar-item {{ background: #f8f9fa; padding: 15px; border-radius: 5px; margin-bottom: 10px; }}
        .calendar-email {{ font-weight: 500; color: #333; margin-bottom: 5px; }}
        .calendar-link {{ color: #666; font-size: 14px; }}
        .calendar-link a {{ color: #4CAF50; text-decoration: none; font-weight: 500; }}
        .calendar-link a:hover {{ color: #45a049; text-decoration: underline; }}
        .calendar-link code {{ color: #e83e8c; background: #fff; padding: 2px 6px; border-radius: 3px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>📊 Dashboard</h1>
        
        <div class="section">
            <h2>Quick Actions</h2>
            <a href="/working-hours" class="button">⚙️ Configure Working Hours</a>
            <a href="/auth/google" class="button">➕ Connect Another Calendar</a>
        </div>
        
        <div class="section">
            <h2>Connected Calendars</h2>
            {}
        </div>
    </div>
</body>
</html>
    "#, email_list)
}

pub fn working_hours_form(start_time: &str, end_time: &str, timezone: &str, days: Vec<&str>) -> String {
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Working Hours Configuration</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; border-radius: 10px; padding: 30px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; margin-bottom: 30px; text-align: center; }}
        .form-group {{ margin-bottom: 20px; }}
        label {{ display: block; margin-bottom: 5px; color: #555; font-weight: 500; }}
        input[type="time"], select {{ width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 5px; font-size: 16px; }}
        .checkbox-group {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }}
        .checkbox-item {{ display: flex; align-items: center; padding: 10px; background: #f8f9fa; border-radius: 5px; }}
        .checkbox-item input {{ margin-right: 8px; }}
        .button {{ background: #4CAF50; color: white; padding: 12px 30px; border: none; border-radius: 5px; cursor: pointer; font-size: 16px; width: 100%; }}
        .button:hover {{ background: #45a049; }}
        .result {{ margin-top: 20px; padding: 15px; border-radius: 5px; display: none; }}
        .result.success {{ background: #d4edda; color: #155724; border: 1px solid #c3e6cb; display: block; }}
        .result.error {{ background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; display: block; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>⚙️ Configure Working Hours</h1>
        
        <form id="workingHoursForm">
            <div class="form-group">
                <label for="start_time">Start Time</label>
                <input type="time" id="start_time" name="start_time" value="{}" required>
            </div>
            
            <div class="form-group">
                <label for="end_time">End Time</label>
                <input type="time" id="end_time" name="end_time" value="{}" required>
            </div>
            
            <div class="form-group">
                <label for="timezone">Timezone</label>
                <select id="timezone" name="timezone" required>
                    <option value="UTC"{}>UTC</option>
                    <option value="America/New_York"{}>America/New_York</option>
                    <option value="America/Chicago"{}>America/Chicago</option>
                    <option value="America/Denver"{}>America/Denver</option>
                    <option value="America/Los_Angeles"{}>America/Los_Angeles</option>
                    <option value="Europe/London"{}>Europe/London</option>
                    <option value="Europe/Paris"{}>Europe/Paris</option>
                    <option value="Europe/Zagreb"{}>Europe/Zagreb</option>
                    <option value="Asia/Tokyo"{}>Asia/Tokyo</option>
                </select>
            </div>
            
            <div class="form-group">
                <label>Working Days</label>
                <div class="checkbox-group">
                    <div class="checkbox-item">
                        <input type="checkbox" id="monday" name="days" value="monday"{}> 
                        <label for="monday">Monday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="tuesday" name="days" value="tuesday"{}> 
                        <label for="tuesday">Tuesday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="wednesday" name="days" value="wednesday"{}> 
                        <label for="wednesday">Wednesday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="thursday" name="days" value="thursday"{}> 
                        <label for="thursday">Thursday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="friday" name="days" value="friday"{}> 
                        <label for="friday">Friday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="saturday" name="days" value="saturday"{}> 
                        <label for="saturday">Saturday</label>
                    </div>
                    <div class="checkbox-item">
                        <input type="checkbox" id="sunday" name="days" value="sunday"{}> 
                        <label for="sunday">Sunday</label>
                    </div>
                </div>
            </div>
            
            <button type="submit" class="button">Save Working Hours</button>
        </form>
        
        <div id="result" class="result"></div>
        
        <div style="text-align: center; margin-top: 30px;">
            <a href="/dashboard" style="color: #4CAF50; text-decoration: none;">← Back to Dashboard</a>
        </div>
    </div>
    
    <script>
        document.getElementById('workingHoursForm').addEventListener('submit', async function(e) {{
            e.preventDefault();
            
            const resultDiv = document.getElementById('result');
            const submitBtn = e.target.querySelector('button');
            
            submitBtn.disabled = true;
            submitBtn.textContent = 'Saving...';
            resultDiv.style.display = 'block';
            resultDiv.className = 'result';
            resultDiv.innerHTML = '⏳ Saving working hours...';
            
            try {{
                const formData = new FormData(e.target);
                const checkedDays = Array.from(formData.getAll('days'));
                
                const workingHoursData = {{
                    start_time: formData.get('start_time'),
                    end_time: formData.get('end_time'),
                    timezone: formData.get('timezone'),
                    working_days: checkedDays.join(',')
                }};
                
                const response = await fetch('/api/working-hours', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify(workingHoursData)
                }});
                
                const result = await response.json();
                
                if (result.success) {{
                    resultDiv.className = 'result success';
                    resultDiv.innerHTML = '✅ <strong>Working hours saved successfully!</strong>';
                }} else {{
                    throw new Error(result.error || 'Failed to save working hours');
                }}
            }} catch (error) {{
                resultDiv.className = 'result error';
                resultDiv.innerHTML = `❌ <strong>Error:</strong> ${{error.message}}`;
            }} finally {{
                submitBtn.disabled = false;
                submitBtn.textContent = 'Save Working Hours';
            }}
        }});
    </script>
</body>
</html>
    "#, 
        start_time, end_time,
        if timezone == "UTC" { " selected" } else { "" },
        if timezone == "America/New_York" { " selected" } else { "" },
        if timezone == "America/Chicago" { " selected" } else { "" },
        if timezone == "America/Denver" { " selected" } else { "" },
        if timezone == "America/Los_Angeles" { " selected" } else { "" },
        if timezone == "Europe/London" { " selected" } else { "" },
        if timezone == "Europe/Paris" { " selected" } else { "" },
        if timezone == "Europe/Zagreb" { " selected" } else { "" },
        if timezone == "Asia/Tokyo" { " selected" } else { "" },
        if days.contains(&"monday") { " checked" } else { "" },
        if days.contains(&"tuesday") { " checked" } else { "" },
        if days.contains(&"wednesday") { " checked" } else { "" },
        if days.contains(&"thursday") { " checked" } else { "" },
        if days.contains(&"friday") { " checked" } else { "" },
        if days.contains(&"saturday") { " checked" } else { "" },
        if days.contains(&"sunday") { " checked" } else { "" }
    )
}

pub fn public_calendar_page(email: &str) -> String {
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Book with {} - Calendar</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); overflow: hidden; }}
        .header {{ background: #4CAF50; color: white; padding: 20px; text-align: center; }}
        .calendar-container {{ display: flex; min-height: 600px; }}
        .calendar {{ flex: 1; padding: 20px; }}
        .slots-panel {{ width: 350px; border-left: 1px solid #ddd; padding: 20px; background: #fafafa; }}
        .month-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }}
        .month-nav {{ background: #4CAF50; color: white; border: none; padding: 8px 12px; border-radius: 5px; cursor: pointer; }}
        .calendar-grid {{ display: grid; grid-template-columns: repeat(7, 1fr); gap: 1px; background: #ddd; border: 1px solid #ddd; border-radius: 8px; overflow: hidden; }}
        .calendar-header {{ background: #f0f0f0; padding: 15px 10px; text-align: center; font-weight: bold; }}
        .calendar-day {{ background: white; padding: 15px 10px; text-align: center; cursor: pointer; min-height: 50px; display: flex; align-items: center; justify-content: center; }}
        .calendar-day:hover {{ background: #e8f5e9; }}
        .calendar-day.selected {{ background: #4CAF50; color: white; }}
        .calendar-day.today {{ background: #e3f2fd; border: 2px solid #2196F3; font-weight: bold; }}
        .calendar-day.today.selected {{ background: #4CAF50; color: white; border-color: #4CAF50; }}
        .calendar-day.past {{ opacity: 0.3; cursor: not-allowed; color: #ccc; }}
        .calendar-day.past:hover {{ background: white; }}
        .timezone-info {{ background: #e3f2fd; padding: 10px; border-radius: 5px; margin-bottom: 20px; }}
        .slot-item {{ background: white; margin-bottom: 10px; padding: 15px; border-radius: 5px; border-left: 4px solid #4CAF50; cursor: pointer; }}
        .slot-item:hover {{ background: #f9f9f9; }}
        .slot-item.unavailable {{ border-left-color: #ccc; background: #f5f5f5; cursor: not-allowed; opacity: 0.6; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>📅 Book with {}</h1>
            <p>Select a time slot to book a meeting</p>
        </div>
        <div class="calendar-container">
            <div class="calendar">
                <div class="month-header">
                    <button class="month-nav" onclick="previousMonth()">&lt;</button>
                    <h2 id="monthYear"></h2>
                    <button class="month-nav" onclick="nextMonth()">&gt;</button>
                </div>
                <div id="calendar-grid" class="calendar-grid">
                    <div class="calendar-header">Sun</div>
                    <div class="calendar-header">Mon</div>
                    <div class="calendar-header">Tue</div>
                    <div class="calendar-header">Wed</div>
                    <div class="calendar-header">Thu</div>
                    <div class="calendar-header">Fri</div>
                    <div class="calendar-header">Sat</div>
                </div>
            </div>
            <div class="slots-panel">
                <div class="timezone-info">
                    Times shown in: <span id="user-timezone"></span>
                </div>
                <h3>Available Time Slots</h3>
                <div style="margin-bottom: 15px;">
                    <label for="slot-type" style="display: block; margin-bottom: 5px; font-weight: 500; color: #555;">Slot Duration:</label>
                    <select id="slot-type" onchange="filterSlotsByType()" style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 5px; background: white;">
                        <option value="30">30 minutes</option>
                        <option value="60">60 minutes</option>
                    </select>
                </div>
                <div id="selected-date">Select a date to view available slots</div>
                <div id="time-slots"></div>
            </div>
        </div>
    </div>

    <script>
        const calendarEmail = '{}';
        let currentMonth = new Date().getMonth();
        let currentYear = new Date().getFullYear();
        let selectedDate = null;
        let userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        let allSlots = []; // Store all fetched slots for filtering

        document.getElementById('user-timezone').textContent = userTimezone;

        function renderCalendar() {{
            const monthNames = ["January", "February", "March", "April", "May", "June",
                "July", "August", "September", "October", "November", "December"];
            
            document.getElementById('monthYear').textContent = monthNames[currentMonth] + ' ' + currentYear;
            
            const firstDay = new Date(currentYear, currentMonth, 1).getDay();
            const daysInMonth = new Date(currentYear, currentMonth + 1, 0).getDate();
            const today = new Date();
            const todayStr = today.getFullYear() + '-' + String(today.getMonth() + 1).padStart(2, '0') + '-' + String(today.getDate()).padStart(2, '0');
            
            let daysHTML = '';
            
            // Add empty cells for days before month starts
            for (let i = 0; i < firstDay; i++) {{
                daysHTML += '<div class="calendar-day" style="visibility: hidden;"></div>';
            }}
            
            for (let day = 1; day <= daysInMonth; day++) {{
                const dateStr = currentYear + '-' + String(currentMonth + 1).padStart(2, '0') + '-' + String(day).padStart(2, '0');
                const currentDate = new Date(currentYear, currentMonth, day);
                const isToday = dateStr === todayStr;
                const isPast = currentDate < new Date(today.getFullYear(), today.getMonth(), today.getDate());
                
                let classes = 'calendar-day';
                let onclick = '';
                let style = '';
                
                if (isToday) {{
                    classes += ' today';
                    onclick = `onclick="selectDate('${{dateStr}}')"`; // Today is selectable
                }} else if (isPast) {{
                    classes += ' past';
                    style = 'opacity: 0.3; cursor: not-allowed; color: #ccc;'; // Past dates are disabled
                }} else {{
                    onclick = `onclick="selectDate('${{dateStr}}')"`; // Future dates are selectable
                }}
                
                daysHTML += `<div class="${{classes}}" ${{onclick}} data-date="${{dateStr}}" style="${{style}}">${{day}}</div>`;
            }}
            
            const calendarGrid = document.getElementById('calendar-grid');
            const existingDays = calendarGrid.querySelectorAll('.calendar-day');
            existingDays.forEach(day => day.remove());
            
            calendarGrid.insertAdjacentHTML('beforeend', daysHTML);
            
            // Auto-select today's date if viewing current month
            if (currentMonth === today.getMonth() && currentYear === today.getFullYear()) {{
                selectDate(todayStr);
            }}
        }}

        function selectDate(dateStr) {{
            selectedDate = dateStr;
            
            // Update selected visual state
            document.querySelectorAll('.calendar-day').forEach(day => {{
                day.classList.remove('selected');
                if (day.dataset.date === dateStr) {{
                    day.classList.add('selected');
                }}
            }});
            
            // Update selected date display
            const dateParts = dateStr.split('-');
            const monthNames = ["January", "February", "March", "April", "May", "June",
                "July", "August", "September", "October", "November", "December"];
            document.getElementById('selected-date').innerHTML = 
                '<strong>' + monthNames[parseInt(dateParts[1]) - 1] + ' ' + parseInt(dateParts[2]) + ', ' + dateParts[0] + '</strong>';
            
            // Fetch available slots
            fetchAvailableSlots(dateStr);
        }}

        async function fetchAvailableSlots(date) {{
            const slotsDiv = document.getElementById('time-slots');
            slotsDiv.innerHTML = '<p>Loading available times...</p>';
            
            try {{
                const slotType = document.getElementById('slot-type').value;
                const response = await fetch(`/api/slots/${{calendarEmail}}?date=${{date}}&timezone=${{userTimezone}}&slot_type=${{slotType}}`);
                const data = await response.json();
                
                if (data.slots && data.slots.length > 0) {{
                    let slotsHTML = '';
                    data.slots.forEach(slot => {{
                        const isAvailable = slot.available;
                        const className = isAvailable ? 'slot-item' : 'slot-item unavailable';
                        const onclick = isAvailable ? `onclick="bookSlot('${{slot.start}}', '${{slot.end}}', '${{slot.display}}')"` : '';
                        slotsHTML += `<div class="${{className}}" ${{onclick}}>
                            <strong>${{slot.display}}</strong>
                            ${{!isAvailable ? '<br><small>Unavailable</small>' : ''}}
                        </div>`;
                    }});
                    slotsDiv.innerHTML = slotsHTML;
                }} else {{
                    slotsDiv.innerHTML = '<p>No available time slots for this date.</p>';
                }}
            }} catch (error) {{
                slotsDiv.innerHTML = '<p>Error loading available times. Please try again.</p>';
            }}
        }}

        function filterSlotsByType() {{
            // When user changes slot type, refetch slots from backend with new type
            if (selectedDate) {{
                fetchAvailableSlots(selectedDate);
            }}
        }}


        function bookSlot(startTime, endTime, displayTime) {{
            const formHTML = `
                <div style="position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); display: flex; justify-content: center; align-items: center; z-index: 1000;">
                    <div style="background: white; padding: 30px; border-radius: 10px; max-width: 400px; width: 90%;">
                        <h2>Book Appointment</h2>
                        <p><strong>Time:</strong> ${{displayTime}}</p>
                        <form id="bookingForm">
                            <div style="margin-bottom: 15px;">
                                <label style="display: block; margin-bottom: 5px;">Your Name *</label>
                                <input type="text" name="name" required style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 5px;">
                            </div>
                            <div style="margin-bottom: 15px;">
                                <label style="display: block; margin-bottom: 5px;">Your Email *</label>
                                <input type="email" name="email" required style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 5px;">
                            </div>
                            <div style="margin-bottom: 15px;">
                                <label style="display: block; margin-bottom: 5px;">Meeting Title *</label>
                                <input type="text" name="title" required style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 5px;">
                            </div>
                            <div style="margin-bottom: 15px;">
                                <label style="display: block; margin-bottom: 5px;">Notes (optional)</label>
                                <textarea name="notes" style="width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 5px; min-height: 60px;"></textarea>
                            </div>
                            <div style="display: flex; gap: 10px;">
                                <button type="submit" style="flex: 1; background: #4CAF50; color: white; padding: 10px; border: none; border-radius: 5px; cursor: pointer;">Book Now</button>
                                <button type="button" onclick="closeBookingForm()" style="flex: 1; background: #6c757d; color: white; padding: 10px; border: none; border-radius: 5px; cursor: pointer;">Cancel</button>
                            </div>
                        </form>
                    </div>
                </div>
            `;
            
            document.body.insertAdjacentHTML('beforeend', formHTML);
            
            document.getElementById('bookingForm').addEventListener('submit', async function(e) {{
                e.preventDefault();
                const formData = new FormData(e.target);
                
                const bookingData = {{
                    calendar_email: calendarEmail,
                    guest_name: formData.get('name'),
                    guest_email: formData.get('email'),
                    title: formData.get('title'),
                    notes: formData.get('notes'),
                    start_time: startTime,
                    end_time: endTime,
                    timezone: userTimezone
                }};
                
                try {{
                    const response = await fetch('/api/book', {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify(bookingData)
                    }});
                    
                    const result = await response.json();
                    
                    if (result.success) {{
                        alert('Booking confirmed! You will receive a calendar invitation via email.');
                        closeBookingForm();
                        fetchAvailableSlots(selectedDate); // Refresh slots
                    }} else {{
                        alert('Booking failed: ' + (result.error || 'Unknown error'));
                    }}
                }} catch (error) {{
                    alert('Error creating booking. Please try again.');
                }}
            }});
        }}

        function closeBookingForm() {{
            const modal = document.querySelector('[style*="position: fixed"]');
            if (modal) modal.remove();
        }}

        function previousMonth() {{
            currentMonth--;
            if (currentMonth < 0) {{
                currentMonth = 11;
                currentYear--;
            }}
            renderCalendar();
        }}

        function nextMonth() {{
            currentMonth++;
            if (currentMonth > 11) {{
                currentMonth = 0;
                currentYear++;
            }}
            renderCalendar();
        }}

        // Initialize calendar
        renderCalendar();
    </script>
</body>
</html>
    "#, email, email, email)
}

pub fn calendar_not_found_page(email: &str) -> String {
    format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Calendar Not Found</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; display: flex; justify-content: center; align-items: center; min-height: 100vh; }}
        .container {{ background: white; border-radius: 10px; padding: 40px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); text-align: center; max-width: 400px; }}
        h1 {{ color: #e74c3c; }}
        .email {{ background: #f8d7da; color: #721c24; padding: 10px 20px; border-radius: 5px; display: inline-block; font-family: monospace; margin: 10px 0; }}
        a {{ color: #4CAF50; text-decoration: none; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>📅 Calendar Not Found</h1>
        <p>No calendar is connected for:</p>
        <div class="email">{}</div>
        <p>The owner needs to connect their Google Calendar first.</p>
        <p style="margin-top: 30px;"><a href="/">← Go to Home</a></p>
    </div>
</body>
</html>
    "#, email)
}