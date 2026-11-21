"""Turns broadcast signals of opendbc into Caruspex packs for listening.

opendbc (github.com/commaai/opendbc, MIT licence, comma.ai) describes the
CAN frames many cars send by themselves all the time: wheel speeds, engine
speed, steering angle, pedals, temperatures. Caruspex can listen to such
frames (ATMA/STMA with a filter, see crates/caruspex-net/src/can_signals.rs)
without asking anything, like the Tesla pack in data/pids does.

    py -I tools/gen_opendbc.py --fetch   download the needed files of the pinned commit into .cache/opendbc/
    py -I tools/gen_opendbc.py           write data/opendbc/opendbc-*.csv from the cache

The cache (about 1.6 MB) is not committed; CARUSPEX_CACHE moves it from
.cache/ to another folder (it gets an opendbc/ inside). The CSV files are, and
tools/gen_pids.py turns them into Rust together with data/pids; the CI does
that step offline. To refresh: set COMMIT, --fetch, convert, run the tests,
commit data/opendbc/.

What is taken: for every pack below a fixed list of value kinds (KINDS) with
the message and signal that holds it in that platform's DBC file. Start bit,
length, byte order, sign, factor and offset are copied exactly; a factor of
SCALE only converts the unit (fraction to %, mph to km/h) and is applied to
factor and offset alike. The generator stops when a named message or signal
is missing, when a message id does not fit into 11 bits or when a chosen
signal is multiplexed. Nothing is guessed: a kind a platform does not have
is left out. Values whose meaning opendbc leaves open (raw pedal counts,
gear numbers without a table, odometers without a sure unit) are left out
on purpose.

DBC files of opendbc's generator directory are joined the way
opendbc/dbc/generator/generator.py does it: every 'CM_ "IMPORT x.dbc";'
line pulls that file in front of the rest.

Matching: the cars of each pack come from the platform lists in
opendbc/car/<brand>/values.py (the CarDocs names with their model years),
mapped to the brand and model of the vehicle catalog (data/vehicles/) by
name, by the catalog's search aliases or through ALIASES below. Names that
fit no catalog model are listed and left out.

Honest note, written into every pack and shown in the app: opendbc was made
for driver assistance and records the buses behind the camera or the radar.
On many cars these buses are not wired to the OBD socket; the gateway sits
in between and lets only diagnosis through, and then the values stay silent
at the socket even with a working adapter.
"""

from __future__ import annotations

import csv
import io
import os
import re
import sys
import unicodedata
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CACHE = os.path.join(os.environ.get("CARUSPEX_CACHE") or os.path.join(ROOT, ".cache"), "opendbc")
OUT = os.path.join(ROOT, "data", "opendbc")
VEHICLES = os.path.join(ROOT, "data", "vehicles")
COMMIT = "c81a2155976b27d73f2fc72026f8bd4b083ce280"
RAW = f"https://raw.githubusercontent.com/commaai/opendbc/{COMMIT}/"

GENERATOR = {
    "toyota": ["_community.dbc", "_toyota_2017.dbc", "_toyota_adas_standard.dbc", "toyota_nodsu_pt.dbc",
               "toyota_tnga_k_pt.dbc", "toyota_new_mc_pt.dbc"],
    "honda": ["_community.dbc", "_honda_common.dbc", "_nidec_common.dbc", "_bosch_2018.dbc",
              "_gearbox_common.dbc", "_lkas_hud_5byte.dbc", "_lkas_hud_8byte.dbc",
              "_nidec_scm_group_a.dbc", "_nidec_scm_group_b.dbc", "_steering_control_a.dbc",
              "_steering_control_b.dbc", "_steering_control_c.dbc", "_steering_sensors_a.dbc",
              "_steering_sensors_b.dbc", "_steering_sensors_c.dbc", "_bosch_radar_acc.dbc",
              "_bosch_standstill.dbc", "honda_civic_touring_2016_can.dbc",
              "honda_civic_hatchback_ex_2017_can.dbc", "honda_crv_touring_2016_can.dbc",
              "honda_odyssey_exl_2018.dbc", "acura_ilx_2016_can.dbc"],
    "hyundai": ["hyundai_can.dbc"],
    "gm": ["_community.dbc", "gm_global_a_powertrain.dbc"],
    "nissan": ["_nissan_common.dbc", "nissan_x_trail_2017.dbc", "nissan_leaf_2018.dbc"],
    "subaru": ["_subaru_global.dbc", "subaru_global_2017.dbc", "_subaru_preglobal_2015.dbc",
               "subaru_outback_2015.dbc", "subaru_forester_2017.dbc"],
}
PLAIN = ["vw_mqb.dbc", "vw_pq.dbc", "tesla_can.dbc", "tesla_powertrain.dbc", "ford_lincoln_base_pt.dbc",
         "byd_atto3.dbc", "tesla_model3_party.dbc"]
VALUES = ["toyota", "honda", "hyundai", "gm", "ford", "volkswagen", "nissan", "subaru", "tesla", "byd"]

# Value kinds: English name, German name, English and German short name,
# unit, gauge range. The same kind has the same names in every pack.
KINDS = {
    "speed": ("Vehicle speed", "Geschwindigkeit", "Speed", "Tempo", "km/h", 0, 250),
    "wheel_fl": ("Wheel speed front left", "Raddrehzahl vorne links", "Wheel FL", "Rad VL", "km/h", 0, 250),
    "wheel_fr": ("Wheel speed front right", "Raddrehzahl vorne rechts", "Wheel FR", "Rad VR", "km/h", 0, 250),
    "wheel_rl": ("Wheel speed rear left", "Raddrehzahl hinten links", "Wheel RL", "Rad HL", "km/h", 0, 250),
    "wheel_rr": ("Wheel speed rear right", "Raddrehzahl hinten rechts", "Wheel RR", "Rad HR", "km/h", 0, 250),
    "rpm": ("Engine speed", "Motordrehzahl", "RPM", "Drehzahl", "rpm", 0, 8000),
    "motor_rpm": ("Drive motor speed", "Drehzahl des Antriebsmotors", "Motor rpm", "E-Motor", "rpm", -17000, 17000),
    "motor_torque": ("Drive motor torque", "Drehmoment des Antriebsmotors", "Motor Nm", "E-Motor Nm", "Nm", -750, 750),
    "steer_angle": ("Steering wheel angle", "Lenkradwinkel", "Steering", "Lenkung", "°", -600, 600),
    "steer_rate": ("Steering wheel speed", "Lenkgeschwindigkeit", "Steer rate", "Lenkrate", "°/s", -1000, 1000),
    "pedal": ("Accelerator pedal", "Gaspedal", "Pedal", "Pedal", "%", 0, 100),
    "brake": ("Brake pedal pressed", "Bremspedal getreten", "Brake", "Bremse", "", 0, 1),
    "brake_bar": ("Brake pressure", "Bremsdruck", "Brake bar", "Bremsdruck", "bar", 0, 200),
    "lat_accel": ("Lateral acceleration", "Querbeschleunigung", "Lat acc", "Quer", "m/s²", -15, 15),
    "long_accel": ("Longitudinal acceleration", "Längsbeschleunigung", "Long acc", "Längs", "m/s²", -15, 15),
    "yaw": ("Yaw rate", "Gierrate", "Yaw", "Gieren", "°/s", -100, 100),
    "coolant": ("Coolant temperature", "Kühlmitteltemperatur", "Coolant", "Kühlmittel", "°C", -40, 130),
    "oil": ("Engine oil temperature", "Motoröltemperatur", "Oil", "Öl", "°C", -40, 150),
    "intake": ("Intake air temperature", "Ansauglufttemperatur", "Intake", "Ansaugluft", "°C", -40, 100),
    "outside": ("Ambient air temperature", "Umgebungstemperatur", "Outside", "Außen", "°C", -40, 60),
    "gearbox_temp": ("Transmission fluid temperature", "Getriebeöltemperatur", "Gearbox", "Getriebe", "°C", -40, 150),
    "fuel_pct": ("Fuel tank level", "Tankfüllstand", "Fuel", "Tank", "%", 0, 100),
    "fuel_l": ("Fuel in the tank", "Kraftstoff im Tank", "Fuel l", "Tank l", "l", 0, 100),
    "battery_soc": ("12 V battery charge", "Ladezustand der 12-V-Batterie", "12V SOC", "12V SOC", "%", 0, 100),
    "door_driver": ("Driver door open", "Fahrertür offen", "Door", "Tür", "", 0, 1),
    "belt_driver": ("Driver seat belt open", "Fahrergurt offen", "Belt", "Gurt", "", 0, 1),
    "parking_brake": ("Parking brake on", "Feststellbremse an", "Handbrake", "Handbremse", "", 0, 1),
    "hv_volts": ("Battery voltage", "Akkuspannung", "Batt V", "Batt V", "V", 0, 500),
    "hv_amps": ("Battery current", "Akkustrom", "Batt A", "Batt A", "A", -600, 600),
}

MPH = 1.609344

# The packs. dbc names the generator file (without .dbc) or a plain file in
# opendbc/dbc; cars names the DBC files in values.py whose platforms the pack
# stands for (with the _generated suffix where opendbc writes it). Every
# value is (kind, message, signal[, scale]).
PACKS = [
    {
        "id": "opendbc-toyota", "dbc": "toyota/toyota_nodsu_pt",
        "cars": ["toyota_nodsu_pt_generated", "toyota_tnga_k_pt_generated", "toyota_new_mc_pt_generated"],
        "title_de": "Toyota und Lexus ab etwa 2016 (opendbc, Mithören)",
        "title_en": "Toyota and Lexus from about 2016 (opendbc, listening)",
        "values": [
            ("speed", "SPEED", "SPEED"), ("wheel_fl", "WHEEL_SPEEDS", "WHEEL_SPEED_FL"),
            ("wheel_fr", "WHEEL_SPEEDS", "WHEEL_SPEED_FR"), ("wheel_rl", "WHEEL_SPEEDS", "WHEEL_SPEED_RL"),
            ("wheel_rr", "WHEEL_SPEEDS", "WHEEL_SPEED_RR"), ("rpm", "ENGINE_RPM", "RPM"),
            ("steer_angle", "STEER_ANGLE_SENSOR", "STEER_ANGLE"), ("steer_rate", "STEER_ANGLE_SENSOR", "STEER_RATE"),
            ("yaw", "KINEMATICS", "YAW_RATE"), ("lat_accel", "KINEMATICS", "ACCEL_Y"),
            ("long_accel", "KINEMATICS", "ACCEL_X"), ("brake", "BRAKE_2", "BRAKE_PRESSED"),
            ("door_driver", "BODY_CONTROL_STATE", "DOOR_OPEN_FL"),
            ("belt_driver", "BODY_CONTROL_STATE", "SEATBELT_DRIVER_UNLATCHED"),
            ("parking_brake", "BODY_CONTROL_STATE", "PARKING_BRAKE"),
        ],
    },
    {
        "id": "opendbc-honda", "dbc": "honda/honda_civic_touring_2016_can",
        "cars": ["honda_civic_touring_2016_can_generated", "honda_civic_hatchback_ex_2017_can_generated",
                 "honda_crv_touring_2016_can_generated", "honda_odyssey_exl_2018_generated",
                 "acura_ilx_2016_can_generated"],
        "title_de": "Honda und Acura ab etwa 2016 (opendbc, Mithören)",
        "title_en": "Honda and Acura from about 2016 (opendbc, listening)",
        "values": [
            ("speed", "CAR_SPEED", "CAR_SPEED"), ("wheel_fl", "WHEEL_SPEEDS", "WHEEL_SPEED_FL"),
            ("wheel_fr", "WHEEL_SPEEDS", "WHEEL_SPEED_FR"), ("wheel_rl", "WHEEL_SPEEDS", "WHEEL_SPEED_RL"),
            ("wheel_rr", "WHEEL_SPEEDS", "WHEEL_SPEED_RR"), ("rpm", "POWERTRAIN_DATA", "ENGINE_RPM"),
            ("brake", "POWERTRAIN_DATA", "BRAKE_PRESSED"), ("lat_accel", "KINEMATICS", "LAT_ACCEL"),
            ("long_accel", "KINEMATICS", "LONG_ACCEL"), ("door_driver", "DOORS_STATUS", "DOOR_OPEN_FL"),
            ("belt_driver", "SEATBELT_STATUS", "SEATBELT_DRIVER_UNLATCHED"),
        ],
    },
    {
        "id": "opendbc-hyundai-kia", "dbc": "hyundai/hyundai_can", "cars": ["hyundai_can_generated"],
        "title_de": "Hyundai, Kia und Genesis mit klassischem CAN (opendbc, Mithören)",
        "title_en": "Hyundai, Kia and Genesis with classic CAN (opendbc, listening)",
        "values": [
            ("wheel_fl", "WHL_SPD11", "WHL_SPD_FL"), ("wheel_fr", "WHL_SPD11", "WHL_SPD_FR"),
            ("wheel_rl", "WHL_SPD11", "WHL_SPD_RL"), ("wheel_rr", "WHL_SPD11", "WHL_SPD_RR"),
            ("rpm", "EMS11", "N"), ("pedal", "EMS12", "PV_AV_CAN"), ("coolant", "EMS12", "TEMP_ENG"),
            ("intake", "EMS15", "IntAirTemp"), ("steer_angle", "SAS11", "SAS_Angle"),
            ("lat_accel", "ESP12", "LAT_ACCEL"), ("long_accel", "ESP12", "LONG_ACCEL"),
            ("yaw", "ESP12", "YAW_RATE"), ("outside", "DATC11", "CR_Datc_OutTempC"),
            ("gearbox_temp", "TCU11", "TEMP_AT"), ("battery_soc", "BAT11", "BAT_SOC"),
        ],
    },
    {
        "id": "opendbc-gm", "dbc": "gm/gm_global_a_powertrain", "cars": ["gm_global_a_powertrain_generated"],
        "title_de": "GM (Chevrolet, GMC, Buick, Cadillac), Global A (opendbc, Mithören)",
        "title_en": "GM (Chevrolet, GMC, Buick, Cadillac), Global A (opendbc, listening)",
        "values": [
            ("speed", "ECMVehicleSpeed", "VehicleSpeed", MPH), ("wheel_fl", "EBCMWheelSpdFront", "FLWheelSpd"),
            ("wheel_fr", "EBCMWheelSpdFront", "FRWheelSpd"), ("wheel_rl", "EBCMWheelSpdRear", "RLWheelSpd"),
            ("wheel_rr", "EBCMWheelSpdRear", "RRWheelSpd"), ("rpm", "ECMEngineStatus", "EngineRPM"),
            ("brake", "ECMEngineStatus", "BrakePressed"), ("coolant", "ECMEngineCoolantTemp", "EngineCoolantTemp"),
            ("steer_angle", "PSCMSteeringAngle", "SteeringWheelAngle"),
            ("steer_rate", "PSCMSteeringAngle", "SteeringWheelRate"),
            ("lat_accel", "EBCMVehicleDynamic", "LateralAcceleration"),
            ("door_driver", "BCMDoorBeltStatus", "FrontLeftDoor"),
            ("hv_volts", "BECMBatteryVoltageCurrent", "HVBatteryVoltage"),
            ("hv_amps", "BECMBatteryVoltageCurrent", "HVBatteryCurrent"),
        ],
    },
    {
        "id": "opendbc-ford", "dbc": "ford_lincoln_base_pt", "cars": ["ford_lincoln_base_pt"],
        "title_de": "Ford und Lincoln ab etwa 2018 (opendbc, Mithören)",
        "title_en": "Ford and Lincoln from about 2018 (opendbc, listening)",
        "values": [
            ("speed", "EngVehicleSpThrottle2", "Veh_V_ActlEng"), ("rpm", "EngVehicleSpThrottle", "EngAout_N_Actl"),
            ("pedal", "EngVehicleSpThrottle", "ApedPos_Pc_ActlArb"), ("coolant", "EngineData_6", "EngClnt_Te_Actl"),
            ("oil", "EngineData_6", "EngOil_Te_Actl"), ("outside", "EngineClimateData", "AirAmb_Te_Actl"),
            ("fuel_pct", "Powertrain_Data_4", "FuelLvl_Pc_DsplyEng"),
            ("steer_angle", "SteeringPinion_Data", "StePinComp_An_Est"),
        ],
    },
    {
        "id": "opendbc-vw-mqb", "dbc": "vw_mqb", "cars": ["vw_mqb"],
        "title_de": "Volkswagen-Konzern, Plattform MQB (opendbc, Mithören)",
        "title_en": "Volkswagen group, MQB platform (opendbc, listening)",
        "values": [
            ("speed", "ESP_21", "ESP_v_Signal"), ("wheel_fl", "ESP_19", "ESP_VL_Radgeschw_02"),
            ("wheel_fr", "ESP_19", "ESP_VR_Radgeschw_02"), ("wheel_rl", "ESP_19", "ESP_HL_Radgeschw_02"),
            ("wheel_rr", "ESP_19", "ESP_HR_Radgeschw_02"), ("rpm", "Motor_12", "MO_Drehzahl_01"),
            ("pedal", "Motor_20", "MO_Fahrpedalrohwert_01"), ("coolant", "Motor_07", "MO_Kuehlmittel_Temp"),
            ("brake", "ESP_05", "ESP_Fahrer_bremst"), ("brake_bar", "ESP_05", "ESP_Bremsdruck"),
            ("long_accel", "ESP_02", "ESP_Laengsbeschl"), ("gearbox_temp", "Getriebe_14", "GE_Sumpftemperatur"),
            ("fuel_pct", "Kombi_03", "KBI_Tankfuellstand_Prozent"), ("fuel_l", "Kombi_02", "KBI_Inhalt_Tank"),
        ],
    },
    {
        "id": "opendbc-vw-pq", "dbc": "vw_pq", "cars": ["vw_pq"],
        "title_de": "Volkswagen-Konzern, Plattform PQ (opendbc, Mithören)",
        "title_en": "Volkswagen group, PQ platform (opendbc, listening)",
        "values": [
            ("speed", "Kombi_1", "Geschwindigkeit__Kombi_1_"), ("wheel_fl", "Bremse_3", "Radgeschw__VL_4_1"),
            ("wheel_fr", "Bremse_3", "Radgeschw__VR_4_1"), ("wheel_rl", "Bremse_3", "Radgeschw__HL_4_1"),
            ("wheel_rr", "Bremse_3", "Radgeschw__HR_4_1"), ("rpm", "Motor_1", "Motordrehzahl"),
            ("pedal", "Motor_1", "Fahrpedalwert_oder_Drosselklapp"),
            ("coolant", "Kombi_2", "Kuehlmitteltemp__4_1__Kombi_2_"), ("oil", "Kombi_2", "Oeltemperatur_4_1"),
            ("outside", "Kombi_2", "Aussentemperatur_gefiltert"), ("fuel_l", "Kombi_1", "Tankinhalt"),
            ("brake_bar", "Bremse_5", "BR5_Bremsdruck"), ("long_accel", "Bremse_8", "BR8_Laengsbeschl"),
        ],
    },
    {
        "id": "opendbc-nissan", "dbc": "nissan/nissan_x_trail_2017", "cars": ["nissan_x_trail_2017_generated"],
        "title_de": "Nissan X-Trail, Rogue und Altima (opendbc, Mithören)",
        "title_en": "Nissan X-Trail, Rogue and Altima (opendbc, listening)",
        "values": [
            ("wheel_fl", "WHEEL_SPEEDS_FRONT", "WHEEL_SPEED_FL"), ("wheel_fr", "WHEEL_SPEEDS_FRONT", "WHEEL_SPEED_FR"),
            ("wheel_rl", "WHEEL_SPEEDS_REAR", "WHEEL_SPEED_RL"), ("wheel_rr", "WHEEL_SPEEDS_REAR", "WHEEL_SPEED_RR"),
            ("steer_angle", "STEER_ANGLE_SENSOR", "STEER_ANGLE"), ("door_driver", "DOORS_LIGHTS", "DOOR_OPEN_FL"),
            ("brake", "DOORS_LIGHTS", "USER_BRAKE_PRESSED"),
        ],
    },
    {
        "id": "opendbc-nissan-leaf", "dbc": "nissan/nissan_leaf_2018", "cars": ["nissan_leaf_2018_generated"],
        "title_de": "Nissan Leaf ab 2018 (opendbc, Mithören)",
        "title_en": "Nissan Leaf from 2018 (opendbc, listening)",
        "values": [
            ("wheel_fl", "WHEEL_SPEEDS_FRONT", "WHEEL_SPEED_FL"), ("wheel_fr", "WHEEL_SPEEDS_FRONT", "WHEEL_SPEED_FR"),
            ("wheel_rl", "WHEEL_SPEEDS_REAR", "WHEEL_SPEED_RL"), ("wheel_rr", "WHEEL_SPEEDS_REAR", "WHEEL_SPEED_RR"),
            ("steer_angle", "STEER_ANGLE_SENSOR", "STEER_ANGLE"), ("door_driver", "DOORS_LIGHTS", "DOOR_OPEN_FL"),
            ("belt_driver", "SEATBELT", "SEATBELT_DRIVER_UNLATCHED"),
        ],
    },
    {
        "id": "opendbc-subaru", "dbc": "subaru/subaru_global_2017", "cars": ["subaru_global_2017_generated"],
        "title_de": "Subaru mit Global-Plattform ab 2017 (opendbc, Mithören)",
        "title_en": "Subaru global platform from 2017 (opendbc, listening)",
        "values": [
            ("speed", "Brake_Pedal", "Speed"), ("wheel_fl", "Wheel_Speeds", "FL"), ("wheel_fr", "Wheel_Speeds", "FR"),
            ("wheel_rl", "Wheel_Speeds", "RL"), ("wheel_rr", "Wheel_Speeds", "RR"), ("rpm", "Throttle", "Engine_RPM"),
            ("steer_angle", "Steering", "Steering_Angle"), ("lat_accel", "G_Sensor", "Lateral"),
            ("door_driver", "BodyInfo", "DOOR_OPEN_FL"),
        ],
    },
    {
        "id": "opendbc-subaru-preglobal", "dbc": "subaru/subaru_outback_2015",
        "cars": ["subaru_outback_2015_generated", "subaru_forester_2017_generated"],
        "title_de": "Subaru vor der Global-Plattform, 2015 bis 2019 (opendbc, Mithören)",
        "title_en": "Subaru before the global platform, 2015 to 2019 (opendbc, listening)",
        "values": [
            ("speed", "Brake_Pedal", "Speed"), ("wheel_fl", "Wheel_Speeds", "FL"), ("wheel_fr", "Wheel_Speeds", "FR"),
            ("wheel_rl", "Wheel_Speeds", "RL"), ("wheel_rr", "Wheel_Speeds", "RR"), ("rpm", "Throttle", "Engine_RPM"),
            ("pedal", "Throttle", "Throttle_Pedal"), ("steer_angle", "Steering", "Steering_Angle"),
            ("coolant", "Engine_Temp", "Coolant_Temp"), ("oil", "Engine_Temp", "Oil_Temp"),
            ("door_driver", "BodyInfo", "DOOR_OPEN_FL"),
        ],
    },
    {
        "id": "opendbc-tesla-s-x", "dbc": "tesla_can", "cars": [],
        "title_de": "Tesla Model S und Model X, frühe Baujahre (opendbc, Mithören)",
        "title_en": "Tesla Model S and Model X, early years (opendbc, listening)",
        # values.py lists only the Model 3 and Y today; tesla_can.dbc is the
        # chassis bus of the Model S of the first Autopilot generation. The
        # Model X shares that platform; single values may differ there.
        "manual_cars": [("Tesla", "Model S", 2012, 2020), ("Tesla", "Model X", 2015, 2020)],
        "values": [
            ("speed", "ESP_B", "ESP_vehicleSpeed"), ("motor_rpm", "DI_torque1", "DI_motorRPM"),
            ("motor_torque", "DI_torque1", "DI_torqueMotor"), ("pedal", "DI_torque1", "DI_pedalPos"),
            ("brake", "DI_torque2", "DI_brakePedal"), ("steer_angle", "STW_ANGLHP_STAT", "StW_AnglHP"),
            ("steer_rate", "STW_ANGLHP_STAT", "StW_AnglHP_Spd"), ("outside", "BODY_R1", "AirTemp_Outsd"),
        ],
    },
    {
        "id": "opendbc-tesla-model-x-hw4", "dbc": "tesla_model3_party", "cars": ["tesla_model3_party"],
        # values.py puts the Model X with HW4 on the party bus of the Model 3
        # and Y. Those two have their own pack from model3dbc in data/pids,
        # so this one is for the Model X only. Left out: the steering angle
        # and rate (carstate.py turns their sign around) and the brake
        # (ESP_driverBrakeApply is a two-bit state).
        "only": [("Tesla", "Model X")],
        "title_de": "Tesla Model X mit HW4 ab 2024 (opendbc, Mithören)",
        "title_en": "Tesla Model X with HW4 from 2024 (opendbc, listening)",
        "values": [
            ("speed", "DI_speed", "DI_vehicleSpeed"), ("wheel_fl", "ESP_wheelSpeeds", "ESP_wheelSpeedFrL"),
            ("wheel_fr", "ESP_wheelSpeeds", "ESP_wheelSpeedFrR"), ("wheel_rl", "ESP_wheelSpeeds", "ESP_wheelSpeedReL"),
            ("wheel_rr", "ESP_wheelSpeeds", "ESP_wheelSpeedReR"), ("pedal", "DI_systemStatus", "DI_accelPedalPos"),
        ],
    },
    {
        "id": "opendbc-byd-atto3", "dbc": "byd_atto3", "cars": ["byd_atto3"],
        "title_de": "BYD Atto 3 (opendbc, Mithören)",
        "title_en": "BYD Atto 3 (opendbc, listening)",
        # The signals opendbc/car/byd/carstate.py reads with a plain meaning.
        # Left out: PEDAL.GAS_PEDAL (0 to 2.55 without a unit, carstate uses
        # RAW_THROTTLE only as "pressed"), the doors (which one is the
        # driver's depends on the market) and SEATBELT_DRIVER (1 is buckled,
        # the opposite of the belt_driver kind).
        "values": [
            ("speed", "WHEELSPEED_CLEAN", "WHEELSPEED_CLEAN"), ("wheel_fl", "WHEEL_SPEED", "WHEELSPEED_FL"),
            ("wheel_fr", "WHEEL_SPEED", "WHEELSPEED_FR"), ("wheel_rl", "WHEEL_SPEED", "WHEELSPEED_BL"),
            ("wheel_rr", "WHEEL_SPEED", "WHEELSPEED_BR"), ("steer_angle", "STEER_MODULE_2", "STEER_ANGLE_2"),
            ("brake", "DRIVE_STATE", "BRAKE_PRESSED"),
        ],
    },
]

# Range checks of the chosen signals: a signal whose 0/1 meaning is turned
# around in opendbc would be a "pressed" that reads 1 when released. Each
# entry here was checked against opendbc/car/<brand>/carstate.py by name.
BOOLEAN_KINDS = {"brake", "door_driver", "belt_driver", "parking_brake"}

# CarDocs names that differ from the catalog: (brand, name) -> catalog model.
ALIASES = {
    ("Toyota", "Prius v"): "Prius+",
    ("Toyota", "Corolla Cross"): None,
    ("Hyundai", "Elantra GT"): "i30",
    ("Honda", "Fit"): "Jazz",
    ("Volkswagen", "e-Golf"): "Golf",
    ("Volkswagen", "Golf SportsVan"): "Golf Sportsvan",
    ("Volkswagen", "Teramont"): "Atlas",
    ("Volkswagen", "Caravelle"): "Transporter",
    ("Volkswagen", "California"): "Transporter",
    ("Audi", "S3"): "A3",
    ("Audi", "RS3"): "A3",
    ("Audi", "A3 Sportback e-tron"): "A3",
}
# Words after the model name that do not make another model.
TRIM = {"hybrid", "plug-in", "electric", "ev", "euv", "prime", "gt", "gti", "gtd", "gte", "r", "rs", "scout",
        "alltrack", "hatchback", "diesel", "ehybrid", "shooting", "brake", "maxi", "premium", "performance",
        "premier", "essence", "esv", "1500", "nms", "f", "cross", "sport", "gli", "x", "trim", "lightning"}
MAKES = {"Lexus", "Toyota", "Honda", "Acura", "Hyundai", "Kia", "Genesis", "Chevrolet", "GMC", "Buick",
         "Cadillac", "Holden", "Ford", "Lincoln", "Volkswagen", "Audi", "SEAT", "CUPRA", "Škoda", "Nissan",
         "Subaru", "MAN", "Porsche", "Tesla", "BYD"}
BRAND_NAMES = {"SEAT": "Seat", "CUPRA": "Cupra", "Škoda": "Škoda"}


class DataError(Exception):
    pass


def fetch_all() -> int:
    paths = ["LICENSE"]
    paths += [f"opendbc/dbc/generator/{d}/{f}" for d, files in GENERATOR.items() for f in files]
    paths += [f"opendbc/dbc/{f}" for f in PLAIN]
    paths += [f"opendbc/car/{b}/values.py" for b in VALUES]
    for path in paths:
        target = os.path.join(CACHE, *path.split("/"))
        os.makedirs(os.path.dirname(target), exist_ok=True)
        request = urllib.request.Request(RAW + path, headers={"User-Agent": "Caruspex opendbc import"})
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read()
        with open(target, "wb") as f:
            f.write(data)
        print(f"{len(data):8d} {path}")
    with open(os.path.join(CACHE, "COMMIT"), "w", encoding="utf-8") as f:
        f.write(COMMIT + "\n")
    return 0


# ---------------------------------------------------------------------------
# DBC files
# ---------------------------------------------------------------------------

IMPORT = re.compile(r'CM_ "IMPORT (.*?)";')
BO = re.compile(r"^BO_ (\d+) (\w+)\s*:")
SG = re.compile(r'^\s*SG_ (\w+)\s*(M|m\d+)?\s*:\s*(\d+)\|(\d+)@([01])([+-])\s*\(([^,]+),([^)]+)\)\s*\[[^\]]*\]\s*"([^"]*)"')


def read(path: str) -> str:
    if not os.path.exists(path):
        raise DataError(f"{os.path.relpath(path, ROOT)} is missing; run --fetch first")
    with open(path, encoding="utf-8") as f:
        return f.read()


def dbc_text(name: str) -> str:
    if "/" in name:
        folder, base = name.split("/")
        directory = os.path.join(CACHE, "opendbc", "dbc", "generator", folder)
        text = read(os.path.join(directory, base + ".dbc"))
        parts = [read(os.path.join(directory, inc)) for inc in IMPORT.findall(text)]
        return "\n".join(parts + [IMPORT.sub("", text)])
    return read(os.path.join(CACHE, "opendbc", "dbc", name + ".dbc"))


def parse_dbc(text: str) -> dict[str, tuple[int, dict]]:
    """Message name -> (id, {signal name: fields}). Later definitions win, as
    in a joined generator file."""
    messages: dict[str, tuple[int, dict]] = {}
    current = None
    for line in text.splitlines():
        m = BO.match(line)
        if m:
            current = m.group(2)
            messages[current] = (int(m.group(1)), {})
            continue
        m = SG.match(line)
        if m and current:
            messages[current][1][m.group(1)] = {
                "mux": m.group(2) or "", "start": int(m.group(3)), "length": int(m.group(4)),
                "order": m.group(5), "sign": m.group(6), "factor": float(m.group(7)),
                "offset": float(m.group(8)), "unit": m.group(9),
            }
    return messages


def number(value: float) -> str:
    text = f"{value:.10g}"
    if "e" in text or "E" in text:
        text = f"{value:.12f}".rstrip("0").rstrip(".")
    return text


def equation(sig: dict, scale: float) -> str:
    return (f"DBC:{sig['start']}|{sig['length']}@{sig['order']}{sig['sign']}"
            f"({number(sig['factor'] * scale)},{number(sig['offset'] * scale)})")


# ---------------------------------------------------------------------------
# Cars from values.py, matched to the catalog
# ---------------------------------------------------------------------------

def fold(text: str) -> str:
    text = unicodedata.normalize("NFKD", text)
    return "".join(c for c in text if not unicodedata.combining(c)).lower().strip()


def catalog() -> dict[str, dict[str, tuple[str, set[str]]]]:
    """Folded brand -> folded model or alias -> (brand, model). A model name
    wins over a search alias of another model ("K5" is a model of its own,
    not the Optima it was sold as)."""
    out: dict[str, dict[str, tuple[str, str]]] = {}
    rows: list[dict] = []
    for name in sorted(os.listdir(VEHICLES)):
        if not name.endswith(".csv") or name == "brands.csv":
            continue
        with open(os.path.join(VEHICLES, name), encoding="utf-8") as f:
            lines = [line for line in f if line.strip() and not line.startswith("#")]
        rows += list(csv.DictReader(lines))
    for row in rows:
        out.setdefault(fold(row["brand"]), {}).setdefault(fold(row["model"]), (row["brand"], row["model"]))
    for row in rows:
        for alias in filter(None, row["aliases"].split("|")):
            out[fold(row["brand"])].setdefault(fold(alias), (row["brand"], row["model"]))
    return out


def years(text: str) -> list[tuple[int, int]]:
    out = []
    for part in text.split(","):
        part = part.strip()
        m = re.fullmatch(r"(\d{4})(?:-(\d{2,4}))?", part)
        if not m:
            raise DataError(f"model years '{text}'")
        start = int(m.group(1))
        end = m.group(2)
        end = start if end is None else (int(end) if len(end) == 4 else start // 100 * 100 + int(end))
        out.append((start, end))
    return out


def platform_cars(dbc_names: list[str]) -> list[str]:
    """CarDocs names of all platforms in values.py whose powertrain DBC is
    one of dbc_names."""
    names: list[str] = []
    for brand in VALUES:
        text = read(os.path.join(CACHE, "opendbc", "car", brand, "values.py"))
        defaults = {}
        for m in re.finditer(r"class (\w+)\((\w*PlatformConfig)\):(.*?)(?=\nclass |\n@dataclass|\Z)", text, re.S):
            # Bus.party: the Tesla platforms name their main bus so.
            d = re.search(r"Bus\.(?:pt|party):\s*['\"](\w+)['\"]", m.group(3))
            if d:
                defaults[m.group(1)] = d.group(1)
            elif m.group(2) in defaults:
                defaults[m.group(1)] = defaults[m.group(2)]
        for block in re.split(r"\n  (?=[A-Z][A-Z0-9_]+ = )", text):
            m = re.match(r"([A-Z][A-Z0-9_]+) = (\w+)\(", block)
            if not m:
                continue
            d = (re.search(r"Bus\.(?:pt|party):\s*['\"](\w+)['\"]", block)
                 or re.search(r"(?:radar_)?dbc_dict\(['\"](\w+)['\"]", block))
            dbc = d.group(1) if d else defaults.get(m.group(2), "")
            if dbc in dbc_names:
                names += re.findall(r"CarDocs\w*\(\s*[\"']([^\"']+)[\"']", block)
    return names


def match_cars(names: list[str], cat: dict, unmatched: list[str]) -> list[tuple[str, str, int, int]]:
    found: dict[tuple[str, str], list[int]] = {}
    for full in names:
        m = re.fullmatch(r"(.+?) ((?:\d{4}(?:-\d{2,4})?)(?:, \d{4}(?:-\d{2,4})?)*)", full.strip())
        if not m:
            unmatched.append(full)
            continue
        name = re.sub(r"\s*\([^)]*\)", "", m.group(1)).strip()
        make = next((b for b in sorted(MAKES, key=len, reverse=True) if name.startswith(b + " ")), "")
        if not make:
            unmatched.append(full)
            continue
        brand = BRAND_NAMES.get(make, make)
        words = name[len(make) + 1:].split()
        hit = None
        if (brand, " ".join(words)) in ALIASES:
            target = ALIASES[(brand, " ".join(words))]
            if target is None:
                continue
            hit = cat.get(fold(brand), {}).get(fold(target))
        while not hit and words:
            hit = cat.get(fold(brand), {}).get(fold(" ".join(words)))
            if hit or len(words) == 1 or fold(words[-1]) not in TRIM:
                break
            words = words[:-1]
        if not hit:
            unmatched.append(full)
            continue
        span = found.setdefault(hit, [9999, 0])
        for a, b in years(m.group(2)):
            span[0], span[1] = min(span[0], a), max(span[1], b)
    return sorted((b, mo, a, z) for (b, mo), (a, z) in found.items())


# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

HEADER = ["Name", "ShortName", "ModeAndPID", "Equation", "Min Value", "Max Value", "Units",
          "Header", "Response", "Name DE", "ShortName DE"]


def write_pack(pack: dict, cars: list, rows: list[list[str]]) -> str:
    path = os.path.join(OUT, pack["id"] + ".csv")
    lines = [
        f"# Generated by tools/gen_opendbc.py from opendbc ({COMMIT[:12]}), do not edit by hand.",
        "# Values the car broadcasts on its CAN bus by itself; Caruspex only listens.",
        "# Many of these buses sit behind the gateway and are silent at the OBD socket.",
        f"#@ id: {pack['id']}",
        f"#@ title_de: {pack['title_de']}",
        f"#@ title_en: {pack['title_en']}",
        # The battery page wants state of charge and health, which no pack
        # here has; the values go to the live page.
        "#@ category: custom",
        "#@ listen: yes",
    ]
    for brand, model, a, z in cars:
        lines.append(f"#@ match: {brand}|{model}|{a}-{z}")
    lines.append(",".join(HEADER))
    lines.append(f"# Source: opendbc {pack['dbc'].split('/')[-1]}.dbc (MIT licence, comma.ai)")
    buf = io.StringIO()
    writer = csv.writer(buf, lineterminator="\n")
    for r in rows:
        writer.writerow(r)
    text = "\n".join(lines) + "\n" + buf.getvalue()
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    return path


def convert() -> int:
    cat = catalog()
    os.makedirs(OUT, exist_ok=True)
    total = 0
    report = []
    for pack in PACKS:
        messages = parse_dbc(dbc_text(pack["dbc"]))
        rows = []
        for entry in pack["values"]:
            kind, msg, sig_name = entry[:3]
            scale = entry[3] if len(entry) > 3 else 1.0
            if msg not in messages:
                raise DataError(f"{pack['id']}: message {msg} not in {pack['dbc']}")
            ident, signals = messages[msg]
            if ident > 0x7FF:
                raise DataError(f"{pack['id']}: {msg} has a 29-bit id")
            if sig_name not in signals:
                raise DataError(f"{pack['id']}: signal {msg}.{sig_name} not in {pack['dbc']}")
            sig = signals[sig_name]
            if sig["mux"]:
                raise DataError(f"{pack['id']}: {msg}.{sig_name} is multiplexed")
            if kind == "pedal" and sig["unit"] != "%" and abs(sig["factor"] - 0.392157) > 1e-6 \
                    and not sig["unit"].endswith("PerCent"):
                raise DataError(f"{pack['id']}: {msg}.{sig_name} is not in %")
            if kind in BOOLEAN_KINDS and sig["length"] != 1:
                raise DataError(f"{pack['id']}: {msg}.{sig_name} is not a single bit")
            en, de, short_en, short_de, unit, low, high = KINDS[kind]
            rows.append([en, short_en, f"CAN{ident:03X}", equation(sig, scale), str(low), str(high), unit,
                         "", "", de, short_de])
        names = platform_cars(pack["cars"]) if pack["cars"] else []
        unmatched: list[str] = []
        cars = match_cars(names, cat, unmatched)
        if "only" in pack:
            cars = [c for c in cars if (c[0], c[1]) in pack["only"]]
        for brand, model, a, z in pack.get("manual_cars", []):
            cars.append((brand, model, a, z))
        if not cars:
            raise DataError(f"{pack['id']}: no car of the catalog")
        write_pack(pack, cars, rows)
        total += len(rows)
        report.append(f"{pack['id']}: {len(rows)} values, {len(cars)} catalog models"
                      + (f", not in the catalog: {'; '.join(unmatched)}" if unmatched else ""))
    license_src = read(os.path.join(CACHE, "LICENSE"))
    with open(os.path.join(OUT, "LICENSE"), "w", encoding="utf-8", newline="\n") as f:
        f.write(license_src)
    print("\n".join(report))
    print(f"{len(PACKS)} packs, {total} values")
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        if "--fetch" in sys.argv:
            return fetch_all()
        return convert()
    except DataError as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
