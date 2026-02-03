import logging
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))
from data.scraper import scrape_rta_timeslots_parallel

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


def load_env_file(env_path: Path) -> dict:
    """Load .env file and return as dict."""
    env_vars = {}
    if env_path.exists():
        with open(env_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, value = line.split("=", 1)
                    env_vars[key.strip()] = value.strip()
    return env_vars


def main():
    env_path = Path(__file__).parent.parent / ".env"
    env_vars = load_env_file(env_path)
    
    username = env_vars.get("USERNAME", "")
    password = env_vars.get("PASSWORD", "")
    
    if not username or not password:
        print(f"ERROR: Missing USERNAME or PASSWORD in {env_path}")
        return
    
    headless = False
    have_booking = False
    
    locations = ["141", "35", "34"]
    
    timeout_ms = 30000
    polling_ms = 500
    
    masked_user = f"{username[:4]}{'*' * (len(username) - 4)}"
    print(f"\n{'='*60}\nRTA Scraper Test\n{'='*60}")
    print(f"Username: {masked_user} | Headless: {headless} | Locations: {locations}\n")
    
    proxies_path = Path(__file__).parent.parent / "data" / "proxies.env"
    proxies = []
    if proxies_path.exists():
        with open(proxies_path) as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith("#"):
                    proxies.append(line)
    
    if not proxies:
        print(f"ERROR: No proxies found in {proxies_path}")
        return
    
    print(f"Loaded {len(proxies)} proxies")
    
    try:
        results = scrape_rta_timeslots_parallel(
            locations=locations,
            headless=headless,
            username=username,
            password=password,
            have_booking=have_booking,
            timeout_ms=timeout_ms,
            proxies=proxies,
            parallel_browsers=1,
        )
        
        print(f"\n{'='*60}\nRESULTS\n{'='*60}")
        
        if not results:
            print("No results returned. Check the logs for errors.")
        
        for location, data in results.items():
            slots = data.get("slots", [])
            available = [s for s in slots if s.get("availability")]
            times = [s.get('startTime', '?') for s in available[:5]]
            extra = f" (+{len(available)-5} more)" if len(available) > 5 else ""
            
            print(f"\n📍 {location}: {data.get('next_available_date', 'N/A')}")
            print(f"   Slots: {len(available)}/{len(slots)} available")
            if times:
                print(f"   Times: {', '.join(times)}{extra}")
            
    except Exception as e:
        logger.exception("Scraper failed")
        print(f"\n❌ Error: {e}")
        print("Make sure Chrome is installed and run: pip install undetected-chromedriver selenium")

if __name__ == "__main__":
    main()
