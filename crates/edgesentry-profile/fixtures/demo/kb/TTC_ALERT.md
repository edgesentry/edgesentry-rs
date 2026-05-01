Site Safety Procedure §3.2 — Time-to-Collision Emergency Stop

When the projected time-to-collision (TTC) between a powered industrial truck and any
person or stationary obstacle drops below 3 seconds, the operator must initiate an
emergency stop immediately.

TTC is computed as: TTC = current_distance / closing_speed

A TTC below 3 seconds indicates that, at current trajectories, a collision will occur
within the operator's reaction and braking distance. The vehicle must come to a full stop
before the situation can be re-assessed.

After a TTC_ALERT event:
  (a) the vehicle must not move again until the operator has confirmed the path is clear,
  (b) the event is logged as a near-miss and reviewed in the next safety debrief.

Threshold rationale: typical forklift braking distance at 5 km/h on a dry warehouse floor
is approximately 2–3 metres; a 3-second TTC provides a minimum 1-second reaction buffer.

Reference: BS EN 1175:2020 Industrial trucks — Safety requirements for electrical/electronic
           equipment; Site Safety Procedure §3.1.
