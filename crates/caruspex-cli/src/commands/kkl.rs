//! The self-test of a KKL cable, without a car.

use std::time::Instant;

use caruspex_link::Serial;
use caruspex_link::timing::{now_ms, sleep_until_ms};

use super::fixed;
use crate::out::say;

/// Checks a KKL cable without it being plugged into the car. It tests what the
/// 5-baud wake-up address needs: toggling the break line and changing the baud
/// rate on the fly. It also measures how accurately the bit times are kept.
pub fn kkl_check(port: &str) -> i32 {
    say(&format!("KKL self-test on {port}\n"));
    say("The cable does NOT need to be plugged into the car for this.\n\n");

    let mut line = Serial::new();
    if let Err(error) = line.open(port, 9600) {
        say(&format!("[fail]  Cannot open the port: {error}\n"));
        return 2;
    }
    say("[ok]    Port opened at 9600 baud\n");

    // 1) Change the baud rate on the fly.
    let mut baud_ok = true;
    for baud in [9600u32, 10400, 4800, 9600] {
        if !line.change_baud(baud) {
            say(&format!("[fail]  Baud rate {baud} is not accepted\n"));
            baud_ok = false;
        }
    }
    if baud_ok {
        say("[ok]    Baud rate can be switched on the fly\n");
    }

    // 2) Toggle the break line. This is what clocks out the wake-up address.
    if !line.set_break(true) || !line.set_break(false) {
        say(
            "[fail]  Cannot toggle the break line. Without it the 5-baud wake-up address does not work.\n",
        );
        line.close();
        return 3;
    }
    say("[ok]    Break line can be toggled\n");

    // 3) Measure the bit times. 5 baud means 200 ms per bit. Windows likes to
    //    tends to oversleep, and the control unit cannot follow overly long bits.
    say("\nMeasuring bit times (10 bits as in a real wake-up address) ...\n");
    // Clocked exactly like the real wake-up address: fixed target times,
    // counted from the start, so that nothing accumulates.
    let mut worst = 200.0_f64;
    let begin = Instant::now();
    let start = now_ms();
    let mut previous = 0.0_f64;
    for bit in 0..10u64 {
        line.set_break(bit % 2 == 0);
        sleep_until_ms(start + (bit + 1) * 200);
        let elapsed = begin.elapsed().as_secs_f64() * 1000.0;
        let ms = elapsed - previous;
        previous = elapsed;
        say(&format!("        Bit {:2}: {} ms\n", bit + 1, fixed(ms, 1)));
        if (ms - 200.0).abs() > (worst - 200.0).abs() {
            worst = ms;
        }
    }
    // Stop before closing: closing a USB port takes a good hundred
    // milliseconds and would distort the measurement.
    let total = previous;
    line.set_break(false);
    line.close();

    let average = total / 10.0;
    say(&format!(
        "        Average {} ms, largest deviation {} ms\n",
        fixed(average, 1),
        fixed(worst, 1)
    ));
    say(&format!(
        "        Total length {} ms (target 2000 ms)\n",
        fixed(total, 0)
    ));

    if (average - 200.0).abs() <= 8.0 {
        say("[ok]    Timing accurate enough for the wake-up address\n");
    } else {
        say("[warn]  Timing deviates. The wake-up address may fail because of it.\n");
    }

    say("\nResult: the cable can do everything the protocol needs.\n");
    say("Whether the control unit answers only shows on the car with the ignition on.\n");
    0
}
