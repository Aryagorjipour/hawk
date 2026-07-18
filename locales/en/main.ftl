# Smart Hawk — English UI

bot-name = Smart Hawk
welcome-back = Welcome back, { $name }! Ready to hunt the web?
welcome-new = Hey! I'm Smart Hawk 🦅 — your AI-powered web snatcher.
mission-aborted = Mission aborted. Coffee still hot though.
main-menu-hint = Pick a wing from the menu below.

btn-crawl = 🕊 Crawl
btn-schedule = ⏰ Schedule
btn-settings = ⚙️ Settings
btn-history = 📜 History
btn-usage = 📊 Usage
btn-about = ℹ️ About
btn-cancel = Cancel
btn-back = « Back
btn-confirm = Confirm
btn-delete = Delete
btn-use-default = Use Telegram default: { $name }

usage-title = 📊 Your nest budget
usage-body =
    Free crawls today: { $free_used }/{ $free_max } used ({ $free_left } left)
    Bonus crawl credits: { $bonus }
    Schedules: { $active_sched } active / { $max_sched } max (+{ $bonus_slots } pack slots)
    Approx. hunts you can still start: ~{ $total }

    AI: { $provider } · { $model } · { $verified }
usage-ai-none = not set up yet
usage-verified = verified ✅
usage-unverified = not verified

onboarding-ask-username = What should I call you?
    (Tap the default or type a nickname.)
onboarding-choose-provider = Which AI brain are we plugging in?
onboarding-ask-base-url = Custom nest needs a base URL (https + API version, e.g. https://host/v1):
onboarding-ask-api-key = Paste your API key. I'll encrypt it and try to delete your message after.
    Never share keys in group chats!
onboarding-key-stored = Key received & locked in the vault 🔐
onboarding-loading-models = Asking the model zoo who's home…
onboarding-pick-model = Pick a model (page { $page }/{ $pages }):
onboarding-verifying = Poking the API with a stick (gently)…
onboarding-success = Green lights everywhere!
    Your key stays encrypted — we don't peep.

    How to fly:
    • Crawl — drop a URL + what you want
    • Schedule — set it and forget it
    • History — past hunts
    • Settings — name, email, AI, nuclear delete
    • About — credits & Stars packs

    Free tier: { $free_crawls } crawls/day, { $free_schedules } active schedules.

onboarding-auth-failed = That key didn't open the door (auth failed). Try another key.
onboarding-provider-failed = Provider hiccup: { $detail }
    Let's pick a provider again.

crawl-ask-url = Drop a link. Make it https-y.
crawl-ask-prompt = What should the hawk snatch from that page?
crawl-started = Wings out. This may take a bit…
crawl-busy = One hunt at a time, captain.
crawl-quota = You're out of free crawls and bonus credits. Grab a Stars pack in About.
crawl-need-onboarding = Finish onboarding first — I need your AI setup.
crawl-done-footer = Pages fetched: { $pages } · Budget left today: ~{ $budget }
crawl-failed = Hunt failed: { $detail }
crawl-unable = The model shrugged: { $reason }

error-llm-auth = Your AI API key was rejected. Open Settings → AI provider and paste a valid key.
error-llm-quota = Your AI provider doesn't have enough credit for this request. Top up balance there (or pick another provider), then try again.
error-llm-rate = Your AI provider is rate-limiting you. Wait a minute and try again.
error-llm-model = That model is unavailable or invalid. Pick another model in Settings → AI provider.
error-llm-network = Could not reach your AI provider (network/timeout). Try again in a moment.
error-llm-bad-response = Your AI provider returned a response we could not understand. Try another model.
error-llm-unknown = Your AI provider returned an error. Check key, balance, and model in Settings.
error-fetch = Could not fetch the page: { $detail }

schedule-hub = 📋 Your schedules ({ $used }/{ $max } slots)
schedule-empty = No schedules yet. Tap + New schedule.
schedule-new = + New schedule
schedule-ask-url = Schedule URL:
schedule-ask-prompt = What should each run extract?
schedule-recurrence = How often?
schedule-interval = Interval
schedule-daily = Daily
schedule-weekly = Weekly
schedule-pick-interval = Choose interval:
schedule-ask-time = Time (HH:MM) in your timezone ({ $tz }):
schedule-pick-days = Pick weekdays (tap to toggle), then Send time as HH:MM
schedule-delivery = Delivery options:
schedule-created = Schedule locked in. Next run: { $next }
schedule-slot-full = No free schedule slots. Buy a pack or deactivate one.
schedule-deleted = Schedule yeeted into the void.
schedule-toggled = Schedule is now { $state }.
schedule-trigger = ⏱ Schedule firing: { $label }
schedule-trigger-now = ▶ Trigger now
schedule-triggered = Wings out for this schedule — result lands here when ready.
schedule-activate = Activate
schedule-deactivate = Deactivate

settings-hub = Settings — tweak the nest.
settings-name = Display name
settings-email = Email
settings-timezone = Timezone
settings-ai = AI provider
settings-language = Language
settings-delete = Delete all my data
settings-delete-confirm = This HARD-DELETES everything. Type DELETE to confirm.
settings-deleted = All your data is gone. Poof. Start with /start anytime.
settings-saved = Saved.

history-title = 📜 Hunt log (newest first)
history-empty = No hunts yet. Go crawl something shiny.
history-item = { $time } { $status } { $url } — { $prompt }

about-body =
    Smart Hawk crawls pages with YOUR AI key and brings back structured loot.

    🌐 Landing: { $landing }
    🐙 GitHub: { $github }

    Free: { $free_crawls } crawls/day · { $free_schedules } schedules
    Stars packs:
    • 25★ → 25 crawls +1 schedule slot
    • 100★ → 120 crawls +5 slots
    • 250★ → 350 crawls +12 slots

about-buy = Buy credits with Stars
about-tip = Thanks for fueling the hawk!

inline-help-crawl = Type a URL (optional ask…) to start a crawl. Result DMs you.
inline-help-history = Prefix with h to pick a past result.
inline-need-setup = Finish setup in a private chat with me first.
inline-started = Crawl started — I'll DM you the loot.
inline-quota = Out of credits — open About in DM.

error-generic = Something flapped sideways: { $detail }
error-ssrf = That address is off-limits (private/local network).
error-invalid-url = That doesn't look like a healthy URL: { $detail }
error-validation = Hold up: { $detail }

provider-openai = OpenAI
provider-anthropic = Anthropic
provider-gemini = Gemini
provider-grok = Grok
provider-openrouter = OpenRouter
provider-custom = Custom (OpenAI-compatible)
