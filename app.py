from flask import Flask, jsonify, request, render_template
from controller import PumpController

app = Flask(__name__)
ctrl = PumpController()

WEEKDAYS = [
    (1, "Monday"), (2, "Tuesday"), (3, "Wednesday"),
    (4, "Thursday"), (5, "Friday"), (6, "Saturday"), (7, "Sunday"),
]


@app.route("/")
def index():
    return render_template("index.html", weekdays=WEEKDAYS, initial_status=ctrl.status)


@app.route("/api/status")
def api_status():
    return jsonify(ctrl.status)


@app.route("/api/schedules", methods=["GET"])
def api_get_schedules():
    return jsonify(ctrl.get_schedules())


@app.route("/api/schedules", methods=["POST"])
def api_add_schedule():
    data = request.get_json()
    if not data or "time" not in data:
        return jsonify({"error": "time required"}), 400
    ctrl.add_schedule(data)
    return jsonify(ctrl.get_schedules()), 201


@app.route("/api/schedules/<int:schedule_id>", methods=["DELETE"])
def api_remove_schedule(schedule_id):
    ok = ctrl.remove_schedule(schedule_id)
    return jsonify({"deleted": ok}), (200 if ok else 404)


@app.route("/api/manual", methods=["POST"])
def api_manual():
    data = request.get_json()
    state = data.get("state", False)
    duration = data.get("duration")
    if state:
        ctrl.set_manual(True, duration)
    else:
        ctrl.set_manual(False)
    return jsonify(ctrl.status)


@app.route("/api/manual", methods=["DELETE"])
def api_clear_manual():
    ctrl.clear_manual()
    return jsonify(ctrl.status)


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=5000, debug=False)
