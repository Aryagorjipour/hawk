# Smart Hawk — Persian UI

bot-name = اسمارت هاوک
welcome-back = دوباره سلام { $name }! آماده‌ای شکار وب؟
welcome-new = سلام! من اسمارت هاوک 🦅 هستم — شکارچی هوشمند وب.
mission-aborted = مأموریت لغو شد. هنوز قهوه داغه.
main-menu-hint = از منوی پایین یه بال انتخاب کن.

btn-crawl = 🕊 کراول
btn-schedule = ⏰ زمان‌بندی
btn-settings = ⚙️ تنظیمات
btn-history = 📜 تاریخچه
btn-usage = 📊 مصرف
btn-about = ℹ️ درباره
btn-cancel = لغو
btn-back = « برگشت
btn-confirm = تأیید
btn-delete = حذف
btn-use-default = پیش‌فرض تلگرام: { $name }

usage-title = 📊 بودجه لونه
usage-body =
    کراول رایگان امروز: { $free_used }/{ $free_max } مصرف‌شده ({ $free_left } باقی)
    اعتبار جایزه کراول: { $bonus }
    زمان‌بندی: { $active_sched } فعال / { $max_sched } حداکثر (+{ $bonus_slots } اسلات بسته)
    تقریبی شکارهای باقی‌مانده: ~{ $total }

    AI: { $provider } · { $model } · { $verified }
usage-ai-none = هنوز تنظیم نشده
usage-verified = تأیید شده ✅
usage-unverified = تأیید نشده

onboarding-ask-username = چه صدایی صدات کنم؟
    (پیش‌فرض رو بزن یا یه اسم بنویس.)
onboarding-choose-provider = کدوم مغز هوش مصنوعی رو وصل کنیم؟
onboarding-ask-base-url = برای سفارشی، آدرس پایه لازم داریم (https + نسخه API مثل https://host/v1):
onboarding-ask-api-key = کلید API رو بفرست. رمزنگاریش می‌کنم و سعی می‌کنم پیامت رو پاک کنم.
    تو گروه کلید نفرست!
onboarding-key-stored = کلید رسید و رفت تو گاوصندوق 🔐
onboarding-loading-models = دارم از باغ‌وحش مدل‌ها می‌پرسم کی خونه است…
onboarding-pick-model = یه مدل انتخاب کن (صفحه { $page }/{ $pages }):
onboarding-verifying = دارم با احتیاط به API نوک می‌زنم…
onboarding-success = همه چراغ‌ها سبزن!
    کلیدت رمزنگاری می‌مونه — فضولی نمی‌کنیم.

    چطور پرواز کنی:
    • کراول — لینک + چیزی که می‌خوای
    • زمان‌بندی — ست کن و برو
    • تاریخچه — شکارهای قبلی
    • تنظیمات — اسم، ایمیل، AI، حذف کامل
    • درباره — اعتبار و بسته ستاره

    رایگان: { $free_crawls } کراول/روز، { $free_schedules } زمان‌بندی فعال.

onboarding-auth-failed = اون کلید در رو باز نکرد (خطای احراز). کلید دیگه امتحان کن.
onboarding-provider-failed = مشکل از ارائه‌دهنده: { $detail }
    دوباره ارائه‌دهنده رو انتخاب کنیم.

crawl-ask-url = یه لینک بده. ترجیحاً https.
crawl-ask-prompt = از اون صفحه چی برات شکار کنم؟
crawl-started = بال‌ها باز. ممکنه یه کم طول بکشه…
crawl-busy = هر بار فقط یه شکار، ناخدا.
crawl-quota = کراول رایگان و اعتبار جایزه‌ات تموم شد. از درباره بسته ستاره بگیر.
crawl-need-onboarding = اول راه‌اندازی رو تموم کن — به تنظیم AI نیاز دارم.
crawl-done-footer = صفحات: { $pages } · بودجه تقریبی امروز: ~{ $budget }
crawl-failed = شکار شکست خورد: { $detail }
crawl-unable = مدل شونه بالا انداخت: { $reason }

error-llm-auth = کلید API هوش مصنوعی رد شد. از تنظیمات → ارائه‌دهنده AI کلید معتبر بگذار.
error-llm-quota = اعتبار/کوتای ارائه‌دهنده AI برای این درخواست کافی نیست. موجودی را شارژ کن (یا ارائه‌دهنده دیگر) و دوباره امتحان کن.
error-llm-rate = ارائه‌دهنده AI محدودت کرده (rate limit). یه دقیقه صبر کن و دوباره بزن.
error-llm-model = این مدل در دسترس نیست یا نامعتبره. از تنظیمات مدل دیگری انتخاب کن.
error-llm-network = به ارائه‌دهنده AI وصل نشدیم (شبکه/تایم‌اوت). کمی بعد دوباره امتحان کن.
error-llm-bad-response = پاسخ ارائه‌دهنده قابل فهم نبود. مدل دیگری امتحان کن.
error-llm-unknown = ارائه‌دهنده AI خطا داد. کلید، موجودی و مدل را در تنظیمات چک کن.
error-fetch = صفحه دریافت نشد: { $detail }

schedule-hub = 📋 زمان‌بندی‌ها ({ $used }/{ $max } اسلات)
schedule-empty = هنوز زمان‌بندی نداری. + جدید بزن.
schedule-new = + زمان‌بندی جدید
schedule-ask-url = آدرس برای زمان‌بندی:
schedule-ask-prompt = هر بار چی استخراج بشه؟
schedule-recurrence = هر چند وقت؟
schedule-interval = بازه‌ای
schedule-daily = روزانه
schedule-weekly = هفتگی
schedule-pick-interval = بازه رو انتخاب کن:
schedule-ask-time = ساعت (HH:MM) به وقت منطقه تو ({ $tz }):
schedule-pick-days = روزهای هفته رو بزن، بعد ساعت رو HH:MM بفرست
schedule-delivery = نحوه ارسال:
schedule-created = زمان‌بندی قفل شد. اجرای بعدی: { $next }
schedule-slot-full = اسلات خالی نداری. بسته بخر یا یکی رو غیرفعال کن.
schedule-deleted = زمان‌بندی رفت تو خلا.
schedule-toggled = وضعیت الان: { $state }.
schedule-trigger = ⏱ زمان‌بندی در حال اجرا: { $label }
schedule-trigger-now = ▶ همین الان اجرا
schedule-triggered = بال‌ها باز برای این زمان‌بندی — نتیجه همین‌جا می‌آد.
schedule-activate = فعال‌سازی
schedule-deactivate = غیرفعال

settings-hub = تنظیمات — لونه رو مرتب کن.
settings-name = نام نمایشی
settings-email = ایمیل
settings-timezone = منطقه زمانی
settings-ai = ارائه‌دهنده AI
settings-language = زبان
settings-delete = حذف همه داده‌های من
settings-delete-confirm = این کار همه چیز رو کامل پاک می‌کنه. برای تأیید بنویس DELETE
settings-deleted = همه داده‌هات پرید. با /start دوباره شروع کن.
settings-saved = ذخیره شد.

history-title = 📜 لاگ شکار (جدیدترین اول)
history-empty = هنوز شکاری نداری. یه چیزی بدرخشون.
history-item = { $time } { $status } { $url } — { $prompt }

about-body =
    اسمارت هاوک با کلید AI خودت صفحه می‌خونه و نتیجه ساخت‌یافته می‌آره.

    🌐 لندینگ: { $landing }
    🐙 گیت‌هاب: { $github }

    رایگان: { $free_crawls } کراول/روز · { $free_schedules } زمان‌بندی
    بسته‌های ستاره:
    • ۲۵★ → ۲۵ کراول +۱ اسلات
    • ۱۰۰★ → ۱۲۰ کراول +۵ اسلات
    • ۲۵۰★ → ۳۵۰ کراول +۱۲ اسلات

about-buy = خرید اعتبار با ستاره
about-tip = مرسی که به هاوک سوخت می‌دی!

inline-help-crawl = یه URL بنویس (سؤال اختیاری) تا کراول شروع بشه. نتیجه دایرکت میاد.
inline-help-history = با پیشوند h نتیجه قبلی انتخاب کن.
inline-need-setup = اول تو چت خصوصی با من راه‌اندازی رو تموم کن.
inline-started = کراول شروع شد — نتیجه‌ش رو دایرکت می‌فرستم.
inline-quota = اعتبار تموم — تو دایرکت About رو باز کن.

error-generic = یه چیزی کج پرید: { $detail }
error-ssrf = این آدرس مجاز نیست (شبکه خصوصی/محلی).
error-invalid-url = این شبیه URL سالم نیست: { $detail }
error-validation = وایسا: { $detail }

provider-openai = OpenAI
provider-anthropic = Anthropic
provider-gemini = Gemini
provider-grok = Grok
provider-openrouter = OpenRouter
provider-custom = سفارشی (سازگار با OpenAI)
