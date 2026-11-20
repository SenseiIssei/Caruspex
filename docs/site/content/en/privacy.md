# Privacy

Caruspex works on your computer. It has no account, no cloud and no tracking.

## What is stored

Everything stays on your computer:

| What | When | Where |
|---|---|---|
| Settings | always | the settings folder of your system (the button **Open storage location** in the settings shows it) |
| Garage: VIN, car, checks, odometer readings, the last working connection | after connecting a car | next to the settings, in the Caruspex data folder |
| Recordings and CSV files | only when you start them | where you save them |
| Log file | only if you switch it on in the settings | the path you choose |
| Crash reports | only if Caruspex crashes | the `crash-reports` folder next to the garage |
| Test reports of the KKL first test | only when you save one | the `test-reports` folder in the settings folder; the VIN and the immobiliser number are shortened unless you tick the box |

You can delete any of these files at any time.

## What is sent

On every installation, also after updating from an older version, Caruspex asks once whether it should look for updates. Only after a yes does it ask GitHub whether there is a newer version, at start and every six hours while it runs. Nothing about you or the car is sent with it. Without a yes no request goes to GitHub. You can change this any time in the settings under **Window and language**. Apart from that Caruspex does not connect to the internet. Talking to a Wi-Fi adapter, an ENET cable or a DoIP car stays in your local network.

Only the release builds published on GitHub look for updates; a copy you build yourself never does. When Caruspex looks for an update or downloads one, GitHub sees, like with any download, your computer's IP address and the version of Caruspex; Caruspex itself sends and stores nothing else, and nobody else, the project included, sees it. GitHub's privacy statement applies to that. Updates are downloaded only from the releases of the Caruspex repository. Buttons such as **Report on GitHub** only open your browser; what you post there is up to you.

## Android

Up to Android 11 the system lets an app find Bluetooth LE adapters only with the location permission. Caruspex asks for it for that reason alone and never reads your location. From Android 12 on it asks for the Bluetooth permission instead, marked as never used for location.

## Crash reports only on request

If Caruspex crashes, it writes a short report to your computer: version, system, the place in the code and the call chain. VINs, the bytes of the car traffic and your home folder are cut out. Nothing is sent. Caruspex shows you the report and offers two buttons: **Report on GitHub** opens a prefilled issue in your browser that you read and change before you post it, **Don't report** puts it away.

## Sharing recordings without personal data

Recordings of real cars help to make Caruspex better for more cars. **Share recording** prepares a copy and shows you exactly what stays and what goes:

- **Removed**: the last digits of the VIN (everywhere in the file), the port name and the Bluetooth or network address of the adapter, the time of day.
- **Never in a recording**: your location and user data.
- **Stays**: the requests and answers, the adapter type, the protocol and the date.

The file stays on your computer until you save it and attach it to a GitHub issue yourself. Nothing is uploaded automatically.

## The test programme

The [test report form](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) asks for the car, year, engine and adapter. It asks for a shortened VIN only; never post the full VIN, your number plate or your name.
