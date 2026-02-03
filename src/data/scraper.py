import asyncio
import json
import logging
import random
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

from scrapling import StealthyFetcher

logging.getLogger().setLevel(0)
logging.getLogger("scrapling").setLevel(logging.WARNING)

async def _wait_and_click(page, selector: str, timeout_ms: int, min_delay: int, max_delay: int):
    await page.wait_for_timeout(random.randint(min_delay, max_delay))
    element = await page.wait_for_selector(selector, timeout=timeout_ms)
    await element.click()


async def _wait_and_select(page, selector: str, value: str, timeout_ms: int, min_delay: int, max_delay: int):
    await page.wait_for_timeout(random.randint(min_delay, max_delay))
    await page.wait_for_selector(selector, timeout=timeout_ms)
    await page.select_option(selector, value=value)


async def _type_like_human(page, selector: str, text: str, min_delay: int = 30, max_delay: int = 120):
    element = await page.wait_for_selector(selector, timeout=30000)
    await element.click()
    for char in text:
        await page.keyboard.type(char)
        await page.wait_for_timeout(random.randint(min_delay, max_delay))


async def _scrape_with_page(
    page,
    locations: list,
    username: str,
    password: str,
    have_booking: bool,
    timeout_ms: int,
    group_idx: int,
) -> dict:
    location_bookings = {}
    
    await page.wait_for_timeout(random.randint(1000, 2000))

    await _type_like_human(page, "#widget_cardNumber", username)
    await page.wait_for_timeout(random.randint(300, 700))

    await _type_like_human(page, "#widget_password", password)

    await _wait_and_click(page, "#nextButton", timeout_ms, 400, 800)
    
    if have_booking:
        await _wait_and_click(page, "//*[text()=\"Manage booking\"]", timeout_ms, 2000, 4000)
        await _wait_and_click(page, "#changeLocationButton", timeout_ms, 1500, 2500)
    else:
        await _wait_and_click(page, "text=Book test", timeout_ms, 2000, 4000)
        await _wait_and_click(page, "#CAR", timeout_ms, 1500, 2500)
        await _wait_and_click(page, "fieldset#DC span.rms_testItemResult", timeout_ms, 500, 1000)
        await _wait_and_click(page, "#nextButton", timeout_ms, 500, 1000)
        await _wait_and_click(page, "#checkTerms", timeout_ms, 1500, 2500)
        await _wait_and_click(page, "#nextButton", timeout_ms, 500, 1000)

    for location in locations:
        try:
            await _wait_and_click(page, "#rms_batLocLocSel", timeout_ms, 1000, 2000)
            await _wait_and_select(page, "#rms_batLocationSelect2", location, timeout_ms, 500, 1000)
            await _wait_and_click(page, "#nextButton", timeout_ms, 2500, 4000)

            try:
                earliest_btn = await page.query_selector("#getEarliestTime")
                if earliest_btn and await earliest_btn.is_visible():
                    await earliest_btn.click()
                    await page.wait_for_timeout(random.randint(2500, 4500))
            except:
                await page.wait_for_timeout(random.randint(500, 1000))
            
            await page.wait_for_timeout(random.randint(1500, 3000))

            # use chrome dev protocol to extract timeslots,
            # something about pywright not executing evaluate calls in the same javascript world as what selenium does
            cdp = await page.context.new_cdp_session(page)
            result = await cdp.send("Runtime.evaluate", {"expression": "JSON.stringify(timeslots)", "returnByValue": True})
            await cdp.detach()
            timeslots_str = result.get("result", {}).get("value")
            timeslots = json.loads(timeslots_str) if timeslots_str and timeslots_str != "undefined" else None
            
            next_available_date = None
            slots = []
            
            if timeslots:
                ajax = timeslots.get("ajaxresult", {})
                slots_data = ajax.get("slots", {})
                next_available_date = slots_data.get("nextAvailableDate")
                list_timeslots = slots_data.get("listTimeSlot", [])
                    
                for slot in list_timeslots:
                    slots.append({
                        "availability": slot.get("availability", False),
                        "slot_number": slot.get("slotNumber"),
                        "startTime": slot.get("startTime", ""),
                    })
            
            logging.debug(f"Group {group_idx}: Parsed {len(slots)} slots for {location}. Next available: {next_available_date}")
            
            location_bookings[location] = {
                "location": location,
                "slots": slots,
                "next_available_date": next_available_date,
            }
            
            await _wait_and_click(page, "#anotherLocationLink", timeout_ms, 1500, 3000)
            
        except Exception as e:
            logging.error(f"Group {group_idx}: Failed processing location {location}: {e}")

            try:
                another_link = await page.query_selector("#anotherLocationLink")
                if another_link and await another_link.is_visible():
                    await another_link.click()
                    logging.info(f"Group {group_idx}: Recovery click succeeded.")
            except:
                logging.warning(f"Group {group_idx}: Recovery failed.")
            await page.wait_for_timeout(random.randint(2000, 3000))
            continue
        
        await page.wait_for_timeout(random.randint(1500, 3000))
    
    logging.debug(f"Group {group_idx}: Finished scraping {len(location_bookings)} locations.")
    
    return location_bookings


def _scrape_single_group(
    locations: list,
    headless: bool,
    username: str,
    password: str,
    have_booking: bool,
    timeout_ms: int,
    proxy: str,
    group_idx: int,
) -> dict:
    logging.debug(f"Group {group_idx}: Starting browser with proxy {proxy} for {len(locations)} locations")
    
    time.sleep(random.uniform(1.0, 3.0) * group_idx)
    
    result_holder = {"bookings": {}}
    
    async def page_action(page):
        result_holder["bookings"] = await _scrape_with_page(
            page, locations, username, password, have_booking, timeout_ms, group_idx)
    
    async def run():
        proxy_config = {"server": f"http://{proxy}"} if proxy else None
        response = await StealthyFetcher.async_fetch(
            "https://www.myrta.com/wps/portal/extvp/myrta/login/",
            headless=headless, 
            network_idle=True, 
            proxy=proxy_config, 
            page_action=page_action
        )
        
        if response and getattr(response, 'status', None) == 403:
            body = getattr(response, 'text', None) or getattr(response, 'body', '') or ''
            return {"bookings": {}, "blocked": {"proxy": proxy, "status_code": 403, "response_body": str(body)[:2000]}}
        return {"bookings": result_holder["bookings"], "blocked": None}
    
    return asyncio.run(run())


def scrape_rta_timeslots_parallel(
    locations: list,
    headless: bool,
    username: str,
    password: str,
    have_booking: bool,
    timeout_ms: int,
    proxies: list,
    parallel_browsers: int,
) -> dict:
    if not locations:
        return {"bookings": {}, "blocked_proxies": []}
    
    if not proxies:
        logging.error("No proxies provided")
        return {"bookings": {}, "blocked_proxies": []}
    
    shuffled_locations = locations.copy()
    random.shuffle(shuffled_locations)
    
    num_groups = min(parallel_browsers, len(proxies), len(locations))
    location_groups = [[] for _ in range(num_groups)]
    for i, loc in enumerate(shuffled_locations):
        location_groups[i % num_groups].append(loc)
    
    active_proxies = proxies[:num_groups]
    
    all_bookings = {}
    blocked_proxies = []
    
    logging.info(f"Starting parallel scrape with {num_groups} browsers for {len(locations)} locations. Proxies: {active_proxies}")
    
    with ThreadPoolExecutor(max_workers=num_groups) as executor:
        futures = {}
        
        for group_idx, (group_locations, proxy) in enumerate(zip(location_groups, active_proxies)):
            if not group_locations:
                continue
                
            future = executor.submit(
                _scrape_single_group,
                group_locations,
                headless,
                username,
                password,
                have_booking,
                timeout_ms,
                proxy,
                group_idx,
            )
            futures[future] = (group_idx, proxy)
        
        for future in as_completed(futures):
            group_idx, proxy = futures[future]
            try:
                result = future.result()
                
                # Check if this proxy was blocked
                if result.get("blocked"):
                    blocked_proxies.append(result["blocked"])
                    logging.warning(f"Group {group_idx} with proxy {proxy} was blocked (403)")
                else:
                    bookings = result.get("bookings", {})
                    all_bookings.update(bookings)
                    logging.debug(f"Group {group_idx} with proxy {proxy} completed. Got {len(bookings)} locations.")
            except Exception as e:
                logging.error(f"Group {group_idx} with proxy {proxy} failed: {e}")
    
    logging.info(f"Parallel scrape complete: {len(all_bookings)}/{len(locations)} locations scraped. {len(blocked_proxies)} proxies blocked.")
    
    return {"bookings": all_bookings, "blocked_proxies": blocked_proxies}
