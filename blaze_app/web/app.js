/// Project Blaze — Dashboard client
/// Polls /api/state and /api/events, renders with CSS transitions


const POLL_MS = 300;
const TIMELINE_MIN_WIDTH_PX = 800;

const ZONE_ICONS = {
    EmergencyRoom: "\u{1F6A8}",
    PharmacyHall:  "\u{1F48A}",
    WardA:         "\u{1F6CF}\uFE0F",
    WardB:         "\u{1F6CF}\uFE0F",
    Lobby:         "\u{1F6AA}",
};

const STATE_ICONS = {
    Idle:        "\u{1F7E2}",
    Busy:        "\u{1F535}",
    WaitingZone: "\u{1F7E1}",
    Offline:     "\u{1F534}",
};

let previousRobotStates = {};
let eventsSince = 0;
let allEvents = [];
let seenEventIndexes = new Set();
let pollTimer = null;
let pollInFlight = false;
let lastQueueListSignature = "";


document.addEventListener("DOMContentLoaded", async () => {
    await loadScenarios();
    document.getElementById("start-btn").addEventListener("click", startScenario);
    document.getElementById("pause-btn").addEventListener("click", timePause);
    document.getElementById("step-btn").addEventListener("click", timeStep);
    document.getElementById("resume-btn").addEventListener("click", timeResume);
    startPolling();
});

async function loadScenarios() {
    try {
        const res = await fetch("/api/scenarios");
        const data = await res.json();
        const sel = document.getElementById("scenario-select");
        data.scenarios.forEach(s => {
            const opt = document.createElement("option");
            opt.value = s.id;
            opt.textContent = s.name;
            opt.title = s.description;
            sel.appendChild(opt);
        });
    } catch (_) {}
}

async function startScenario() {
    const id = document.getElementById("scenario-select").value;
    if (!id) return;
    const manual = document.getElementById("manual-mode-cb").checked;
    eventsSince = 0;
    allEvents = [];
    seenEventIndexes.clear();
    previousRobotStates = {};
    lastQueueListSignature = "";
    try {
        const url = "/api/scenario/start?id=" + encodeURIComponent(id) +
            (manual ? "&manual=1" : "");
        await fetch(url, { method: "POST" });
    } catch (_) {}
}

async function timePause() {
    try { await fetch("/api/time/pause", { method: "POST" }); } catch (_) {}
}

async function timeStep() {
    try { await fetch("/api/time/step", { method: "POST" }); } catch (_) {}
}

async function timeResume() {
    try { await fetch("/api/time/resume", { method: "POST" }); } catch (_) {}
}

function startPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(poll, POLL_MS);
    poll();
}

async function poll() {
    if (pollInFlight) return;
    pollInFlight = true;
    try {
        const [stateRes, eventsRes, timeRes] = await Promise.all([
            fetch("/api/state"),
            fetch("/api/events?since=" + eventsSince),
            fetch("/api/time/state"),
        ]);
        const state = await stateRes.json();
        const eventsData = await eventsRes.json();
        const timeState = await timeRes.json();
        renderState(state);
        if (Number(eventsData.total_count) < eventsSince) {
            // Event source was reset Reset local cache.
            eventsSince = 0;
            allEvents = [];
            seenEventIndexes.clear();
        }
        if (eventsData.events && eventsData.events.length > 0) {
            for (const ev of eventsData.events) {
                if (seenEventIndexes.has(ev.index)) continue;
                seenEventIndexes.add(ev.index);
                allEvents.push(ev);
            }
        }
        renderTimeline(allEvents, state.metrics);
        renderTimeControls(timeState, state.running);
        eventsSince = eventsData.total_count;
    } catch (_) {
    } finally {
        pollInFlight = false;
    }
}

function renderTimeControls(timeState, running) {
    const ctrl = document.getElementById("time-controls");
    const badge = document.getElementById("time-mode-badge");
    const pauseBtn = document.getElementById("pause-btn");
    const stepBtn = document.getElementById("step-btn");
    const resumeBtn = document.getElementById("resume-btn");
    const manual = timeState.manual_mode || false;
    const paused = timeState.paused || false;

    if (manual && running) {
        ctrl.classList.add("visible");
        badge.textContent = paused ? "Paused" : "Running";
        badge.className = "time-badge " + (paused ? "paused" : "manual");
        pauseBtn.disabled = paused;
        stepBtn.disabled = !paused;
        resumeBtn.disabled = !paused;
    } else {
        ctrl.classList.remove("visible");
        badge.textContent = "Auto";
        badge.className = "time-badge auto";
    }
}

// ---- Render state ----
function renderState(state) {
    renderStatus(state.running, state.scenario_name);
    renderQueue(state.queue, state.metrics, state.robots);
    renderZones(state.zones);
    renderRobots(state.robots);
    renderMetrics(state.metrics);
}

function renderStatus(running, name) {
    const dot = document.getElementById("status-indicator");
    const txt = document.getElementById("status-text");
    dot.className = "status-dot " + (running ? "running" : (name ? "finished" : "idle"));
    txt.textContent = running ? "Running: " + name : (name ? "Finished: " + name : "Idle");
}

function renderQueue(q, metrics, robots) {
    if (!q) return;
    const totalPushed = q.total_pushed || 0;
    const pending = q.total_count;
    const completed = metrics && metrics.completed_task_count != null ? Number(metrics.completed_task_count) : 0;
    const inProgress = Array.isArray(robots)
        ? robots.filter(r => r.current_task_id != null).length
        : Math.max(0, totalPushed - pending - completed);
    setTextIfChanged(document.getElementById("q-all"), totalPushed);
    setTextIfChanged(document.getElementById("q-total"), pending);
    setTextIfChanged(document.getElementById("q-progress"), inProgress);
    setTextIfChanged(document.getElementById("q-done"), completed);

    const listEl = document.getElementById("queue-task-list");
    const tasks = q.tasks || [];
    const nextSignature = tasks
        .map(t => `${t.id}:${t.priority}:${t.kind}:${t.zone}`)
        .join("|");

    if (tasks.length === 0) {
        if (lastQueueListSignature !== "") {
            listEl.innerHTML = "";
            lastQueueListSignature = "";
        }
        return;
    }
    if (nextSignature === lastQueueListSignature) {
        return; // Ignore unchanged queue list to avoid chip flicker.
    }

    listEl.innerHTML = tasks.map(t => {
        const pri = t.priority === "Urgent" ? "\u{1F534}" : "\u{1F535}";
        return '<span class="queue-task-chip ' + t.priority.toLowerCase() + '">' +
            pri + ' #' + t.id + ' ' + t.kind + ' \u{2192} ' + t.zone +
            '</span>';
    }).join("");
    lastQueueListSignature = nextSignature;
}

function renderZones(zones) {
    if (!zones || zones.length === 0) return;
    const container = document.getElementById("zones-container");
    const existingCards = container.querySelectorAll(".zone-card");
    if (existingCards.length !== zones.length) {
        container.innerHTML = "";
        zones.forEach(z => container.appendChild(createZoneCard(z)));
    } else {
        zones.forEach((z, i) => updateZoneCard(existingCards[i], z));
    }
}

function createZoneCard(z) {
    const div = document.createElement("div");
    div.className = "zone-card " + (z.occupant !== null ? "occupied" : "free");
    div.innerHTML = zoneCardHTML(z);
    return div;
}

function updateZoneCard(el, z) {
    const wasOccupied = el.classList.contains("occupied");
    const isOccupied = z.occupant !== null;
    el.className = "zone-card " + (isOccupied ? "occupied" : "free");
    el.innerHTML = zoneCardHTML(z);
}

function zoneCardHTML(z) {
    const icon = ZONE_ICONS[z.zone] || "\u{1F3E5}";
    const occupantHTML = z.occupant !== null
        ? '<span class="robot-badge">\u{1F916} R' + z.occupant + '</span>'
        : '<span class="free-label">Free</span>';
    let waitingHTML = '<span class="waiting-label">Wait:</span> ';
    if (z.waiting_robots.length === 0) {
        waitingHTML += "—";
    } else {
        waitingHTML += z.waiting_robots.map(id =>
            '<span class="waiting-badge">\u{1F916} R' + id + '</span>'
        ).join('<span class="waiting-arrow">\u{2192}</span>');
    }
    return '<div class="zone-header">' +
               '<span class="zone-icon">' + icon + '</span>' +
               '<span class="zone-name">' + z.zone + '</span>' +
           '</div>' +
           '<div class="zone-occupant">' + occupantHTML + '</div>' +
           '<div class="zone-waiting">' + waitingHTML + '</div>';
}

function renderRobots(robots) {
    if (!robots || robots.length === 0) return;
    const container = document.getElementById("robots-container");
    const existingCards = container.querySelectorAll(".robot-card");
    if (existingCards.length !== robots.length) {
        container.innerHTML = "";
        robots.forEach(r => container.appendChild(createRobotCard(r)));
    } else {
        robots.forEach((r, i) => updateRobotCard(existingCards[i], r));
    }
    const newStates = {};
    robots.forEach(r => { newStates[r.robot_id] = r.state; });
    previousRobotStates = newStates;
}

function createRobotCard(r) {
    const div = document.createElement("div");
    div.className = "robot-card state-" + r.state;
    div.innerHTML = robotCardHTML(r);
    return div;
}

function updateRobotCard(el, r) {
    const prev = previousRobotStates[r.robot_id];
    const changed = prev && prev !== r.state;
    el.className = "robot-card state-" + r.state + (changed ? " state-changed" : "");
    el.innerHTML = robotCardHTML(r);
}

function robotCardHTML(r) {
    const stateIcon = STATE_ICONS[r.state] || "";
    let detail = "";
    if (r.current_task_id !== null) detail += "Task #" + r.current_task_id;
    if (r.current_zone) detail += (detail ? " \u{2022} " : "") + r.current_zone;
    if (!detail) detail = "—";
    return '<span class="robot-icon">\u{1F916}</span>' +
           '<div class="robot-info">' +
               '<span class="robot-name">Robot ' + r.robot_id + '</span>' +
               '<span class="robot-detail">' + detail + '</span>' +
           '</div>' +
           '<span class="state-badge ' + r.state + '">' + stateIcon + ' ' + r.state + '</span>';
}

function renderMetrics(m) {
    if (!m) return;
    document.getElementById("m-completed").textContent = m.completed_task_count;
    document.getElementById("m-waits").textContent     = m.total_wait_count;
    document.getElementById("m-offline").textContent   = m.offline_count;
    document.getElementById("m-runtime").textContent   = m.runtime_ms !== null ? m.runtime_ms : "\u{2014}";
}

/// Render timeline 
const LANE_ORDER = ["ROBOT", "TASK", "ZONE", "HEALTH", "SYSTEM"];

function renderTimeline(events, metrics) {
    const container = document.getElementById("timeline-container");

    if (!events || events.length === 0) {
        container.innerHTML = '<p class="placeholder">No events yet</p>';
        return;
    }

    const maxMs = metrics && metrics.runtime_ms != null
        ? Math.max(Number(metrics.runtime_ms), 1)
        : Math.max(...events.map(e => Number(e.elapsed_ms)), 1);

    const widthPx = Math.max(TIMELINE_MIN_WIDTH_PX, Math.min(3000, maxMs / 12));
    container.style.minWidth = widthPx + "px";

    const lanes = {};
    LANE_ORDER.forEach(cat => { lanes[cat] = []; });
    events.forEach(ev => {
        const cat = (ev.category || "").trim() || "SYSTEM";
        if (lanes[cat]) lanes[cat].push(ev);
    });

    let html = '<div class="timeline-axis">';
    for (let i = 0; i <= 4; i++) {
        const ms = Math.round((i / 4) * maxMs);
        const pct = (i / 4) * 100;
        html += '<span class="timeline-axis-label" style="left:' + pct + '%">' + ms + 'ms</span>';
    }
    html += '</div><div class="timeline-lanes">';

    LANE_ORDER.forEach(cat => {
        const evs = lanes[cat] || [];
        if (evs.length === 0) return;
        html += '<div class="timeline-lane ' + cat + '">';
        html += '<span class="timeline-lane-label">' + cat + '</span>';
        html += '<div class="timeline-track">';
        evs.forEach(ev => {
            let pct = Math.min(100, (Number(ev.elapsed_ms) / maxMs) * 100);
            const msg = escapeHTML(ev.message);
            const tip = '[' + ev.elapsed_ms + 'ms] ' + msg;
            const sameTime = evs.filter(e => Number(e.elapsed_ms) === Number(ev.elapsed_ms));
            const myIdx = sameTime.indexOf(ev);
            const n = sameTime.length;
            const topOffset = n > 1 ? (myIdx - (n - 1) / 2) * 8 : 0;
            if (n > 1) {
                const spread = Math.min(2.5, 20 / n);
                pct = Math.max(0, Math.min(100, pct + (myIdx - (n - 1) / 2) * spread));
            }
            html += '<span class="timeline-marker-wrap" style="left:' + pct + '%;top:' + (50 + topOffset) + '%" ' +
                'data-tip="' + escapeAttr(tip) + '" title="' + escapeAttr(tip) + '">' +
                '<span class="timeline-marker"></span>' +
                '<span class="timeline-marker-label">' + msg + '</span>' +
                '</span>';
        });
        html += '</div></div>';
    });

    html += '</div>';
    container.innerHTML = html;

    container.querySelectorAll(".timeline-marker-wrap").forEach(el => {
        el.addEventListener("mouseenter", showTooltip);
        el.addEventListener("mouseleave", hideTooltip);
        el.addEventListener("mousemove", moveTooltip);
    });
}

let tooltipEl = null;

function showTooltip(e) {
    const wrap = e.target.closest(".timeline-marker-wrap");
    if (!wrap) return;
    if (tooltipEl) tooltipEl.remove();
    tooltipEl = document.createElement("div");
    tooltipEl.className = "timeline-tooltip";
    tooltipEl.textContent = wrap.getAttribute("data-tip") || wrap.title;
    document.body.appendChild(tooltipEl);
}

function hideTooltip() {
    if (tooltipEl) {
        tooltipEl.remove();
        tooltipEl = null;
    }
}

function moveTooltip(e) {
    if (!tooltipEl) return;
    tooltipEl.style.left = (e.clientX + 12) + "px";
    tooltipEl.style.top = (e.clientY + 12) + "px";
}

function escapeHTML(s) {
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
}

function escapeAttr(s) {
    return String(s)
        .replace(/&/g, "&amp;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;")
        .replace(/</g, "&lt;");
}

function setTextIfChanged(el, value) {
    const text = String(value);
    if (el.textContent !== text) {
        el.textContent = text;
    }
}
