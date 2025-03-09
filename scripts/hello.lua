-- Hello World widget example for swaydgets
-- This demonstrates the basic features of the Lua API

-- Log a message to show the script is running
log("Hello widget starting up...")

-- Create a window with title and dimensions
local window = create_window("Hello Widget", 200, 100)

-- Configure window margins
window:set_margin("top", 20)
window:set_margin("left", 20)

-- Create a vertical box container with 5px spacing
local box = window:add_box("vertical", 5)

-- Style the box with a semi-transparent black background and rounded corners
box:set_css([[
  box {
    background-color: rgba(0, 0, 0, 0.7);
    border-radius: 10px;
    padding: 10px;
  }
]])

-- Add a greeting label
local greeting = box:add_label("Hello, World!", 18)
greeting:set_css([[
  label {
    color: #ffffff;
    font-weight: bold;
  }
]])

-- Add a time label that will be updated
local time_label = box:add_label("Loading time...", 14)
time_label:set_css([[
  label {
    color: #aaaaff;
    font-style: italic;
  }
]])

-- Set update interval to 1 second
window:set_update_interval(1)

-- Function to update the time
local function update_time()
  local current_time = os.date("%H:%M:%S")
  time_label:set_text("Current time: " .. current_time)
  log("Updated time: " .. current_time)
  
  -- This function would be called by the schedule_update mechanism
  -- when it's properly implemented
end

-- Call it once to initialize
update_time()

-- Show the window
window:show()

-- Note: The schedule_update function is commented out in the Rust code
-- When implemented, you would call it like:
-- schedule_update(update_time)

log("Hello widget initialized!")