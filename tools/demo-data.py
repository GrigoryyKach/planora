#!/usr/bin/env python3
"""Writes a demo data.json (English, relative to today) for screenshots and manual testing.

Usage:  python tools/demo-data.py <folder>
Then:   set PLANORA_DATA_DIR=<folder> and start Planora.exe. Never point this at your real %APPDATA%\Planora.
"""
import json, sys, os, datetime as dt

folder = sys.argv[1] if len(sys.argv) > 1 else "demo-data"
os.makedirs(folder, exist_ok=True)
today = dt.date.today()
now = dt.datetime.now().replace(microsecond=0).isoformat()
n = 0


def uid():
    global n
    n += 1
    return f"{n:016x}"


def day(offset):
    return (today + dt.timedelta(days=offset)).isoformat()


projects = []


def project(profile, name, color, description=""):
    p = {"id": uid(), "profile": profile, "name": name, "description": description, "color": color, "archived": False, "created": now}
    projects.append(p)
    return p["id"]


tasks = []


def task(profile, title, project_id=None, **kw):
    t = {"id": uid(), "profile": profile, "project_id": project_id, "title": title, "description": "", "links": [],
         "status": None, "priority": None, "deadline": None, "scheduled": None, "time": None, "remind": "default",
         "recurrence": None, "done_dates": [], "checklist": [], "created": now, "updated": now, "completed_at": None}
    t.update(kw)
    tasks.append(t)
    return t


reno = project("personal", "Home renovation", 0x10b981, "Kitchen and hallway refresh")
spanish = project("personal", "Learn Spanish", 0xec4899, "Twenty minutes a day")
web = project("work", "Website launch", 0x4f6bff, "New marketing site, go live in December")
plan = project("work", "Q4 planning", 0xf59e0b, "Roadmap and budget")

chk = lambda *items: [{"text": t, "done": d} for t, d in items]

task("personal", "Buy paint samples", reno, status="done", priority="low", scheduled=day(-3))
task("personal", "Book the electrician", reno, status="in_progress", priority="high", deadline=day(2),
     checklist=chk(("Compare three quotes", True), ("Check reviews", True), ("Call and confirm", False)),
     links=[{"url": "https://example.com/electricians", "title": "Shortlist"}])
task("personal", "Choose kitchen tiles", reno, status="todo", priority="medium", deadline=day(9), scheduled=day(5))
task("personal", "Measure the hallway", reno, deadline=day(-2), priority="medium", status="todo")
task("personal", "Spanish lesson", spanish, scheduled=day(0), time="18:30:00", remind={"min": 10}, priority="medium",
     recurrence={"kind": "weekly", "interval": 1, "until": None}, done_dates=[day(-7)])
task("personal", "Review 50 new words", spanish, status="todo", scheduled=day(1))
task("personal", "Call mom", None, scheduled=day(0), time="20:00:00", remind={"min": 5}, description="Ask about the weekend.")
task("personal", "Renew gym membership", None, deadline=day(6), priority="low")
task("personal", "Dentist appointment", None, scheduled=day(4), time="09:30:00", remind={"min": 30}, priority="high")
task("personal", "Plan weekend trip", None, scheduled=day(11), status="todo")
task("personal", "Water the plants", None, scheduled=day(-1), recurrence={"kind": "days", "interval": 3, "until": None})

task("work", "Design review", web, scheduled=day(0), time="11:00:00", remind={"min": 15}, priority="high",
     description="Walk through the homepage mockups with the team.", checklist=chk(("Prepare slides", True), ("Share the Figma link", False)))
task("work", "Write landing page copy", web, status="in_progress", priority="urgent", deadline=day(1))
task("work", "Set up analytics", web, status="todo", priority="medium", deadline=day(7), scheduled=day(3))
task("work", "Cross-browser testing", web, priority="high", deadline=day(12))
task("work", "Launch announcement", web, deadline=day(18), priority="urgent", scheduled=day(18))
task("work", "Draft Q4 roadmap", plan, status="in_progress", priority="high", deadline=day(4),
     checklist=chk(("Collect team input", True), ("Prioritise", True), ("Estimate", False), ("Send for review", False)))
task("work", "Budget meeting", plan, scheduled=day(2), time="14:00:00", remind={"min": 10}, priority="medium")
task("work", "Hiring plan", plan, status="todo", deadline=day(-1), priority="medium")
task("work", "Team stand-up", None, scheduled=day(-2), time="10:00:00", remind="off",
     recurrence={"kind": "weekdays", "interval": 1, "until": None})
task("work", "Send weekly report", None, scheduled=day(3), time="16:30:00", priority="low",
     recurrence={"kind": "weekly", "interval": 1, "until": None})
task("work", "Invoice the client", None, deadline=day(0), priority="high", status="todo")
task("work", "Update the wiki", None, scheduled=day(5), status="cancelled")
task("work", "Quarterly review", None, scheduled=day(14), time="15:00:00")

for t in tasks:
    if t["status"] == "done":
        t["completed_at"] = now

with open(os.path.join(folder, "data.json"), "w", encoding="utf-8") as f:
    json.dump({"version": 1, "projects": projects, "tasks": tasks}, f, indent=2)
print("wrote", os.path.join(folder, "data.json"))
